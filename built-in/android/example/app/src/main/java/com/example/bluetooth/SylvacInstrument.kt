package com.example.bluetooth

import android.annotation.SuppressLint
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.BluetoothStatusCodes
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.util.Log

/**
 * One connected Sylvac instrument.
 *
 * All state lives on the main thread: GATT callbacks (binder threads) are
 * posted to [main], and GATT operations run one at a time from [ops],
 * because Android supports only one outstanding GATT operation per link.
 *
 * Sequence: connect -> (bond if the instrument advertises the PAIR profile)
 * -> discover services -> read 0x2904 and battery -> enable notifications
 * (Parameters, Measurement, RemoteResponse) and indications (DataSend) -> READY.
 * An unexpected disconnection reconnects with autoConnect until [disconnect].
 */
@SuppressLint("MissingPermission") // BLUETOOTH_CONNECT is checked by MainActivity before use.
class SylvacInstrument(
    private val context: Context,
    val device: BluetoothDevice,
    var name: String,
    /** Profile guessed from the advertising flags. PAIR: bond right after connecting. */
    val profile: Sylvac.AdvProfile,
    private val listener: Listener,
) {
    interface Listener {
        fun onInstrumentChanged(instrument: SylvacInstrument)
    }

    enum class State { CONNECTING, BONDING, SETTING_UP, READY, RECONNECTING, DISCONNECTED }

    var state = State.CONNECTING
        private set
    var displayValue = "—"
        private set
    var displayUnit = ""
        private set
    /** True for the fake instruments of the `simulation` package (no Bluetooth link). */
    val simulated get() = simulatedAnswer != null
    var buttonText: String? = null
        private set
    var batteryPercent: Int? = null
        private set
    /** The last command given to [sendCommand], and the lines that came back since (or why it was not sent). */
    var lastCommand: String? = null
        private set
    val lastReply = ArrayDeque<String>()

    val address: String get() = device.address
    val modeText: String? get() = params?.modeLabel

    private var format: Sylvac.PresentationFormat? = null
    private var params: Sylvac.Parameters? = null
    private var lastRaw: Int? = null
    private var simulatedAnswer: ((command: String) -> String?)? = null

    private val main = Handler(Looper.getMainLooper())
    private var gatt: BluetoothGatt? = null
    private var userDisconnect = false
    private var servicesReady = false
    private var reconnectDelayMs = RECONNECT_MIN_MS
    private var lastDirectAttempt = 0L
    private val rxBuffer = StringBuilder()

    private sealed class Op(val label: String) {
        var retried = false

        class Read(val ch: BluetoothGattCharacteristic, label: String) : Op(label)
        class ReadDescriptor(val d: BluetoothGattDescriptor, label: String) : Op(label)
        class Subscribe(val ch: BluetoothGattCharacteristic, val indicate: Boolean, label: String) : Op(label)
        class Write(val ch: BluetoothGattCharacteristic, val data: ByteArray, label: String) : Op(label)
    }

    private val ops = ArrayDeque<Op>()
    private var current: Op? = null
    /**
     * The instrument did not answer service discovery or an operation in time. Skipping it would
     * end in a "ready" instrument that lacks a subscription, so drop the link and set it up again.
     */
    private val opTimeout = Runnable {
        log("Timeout: ${current?.label ?: "service discovery"}")
        dropLink()
    }
    private val replyTimeout = Runnable { if (lastReply.isEmpty()) addReply("no reply") }

    private val bondReceiver = object : BroadcastReceiver() {
        override fun onReceive(c: Context, intent: Intent) {
            val dev = intent.getParcelableExtra(BluetoothDevice.EXTRA_DEVICE, BluetoothDevice::class.java)
            if (dev?.address != device.address) return
            when (intent.getIntExtra(BluetoothDevice.EXTRA_BOND_STATE, BluetoothDevice.ERROR)) {
                BluetoothDevice.BOND_BONDING -> log("Bonding…")
                BluetoothDevice.BOND_BONDED -> onBonded()
                BluetoothDevice.BOND_NONE -> onBondFailed()
            }
        }
    }

    init {
        context.registerReceiver(bondReceiver, IntentFilter(BluetoothDevice.ACTION_BOND_STATE_CHANGED), Context.RECEIVER_EXPORTED)
    }

    // ---------------------------------------------------------------- public API

    fun connect() {
        userDisconnect = false
        setState(State.CONNECTING)
        log("Connecting (${profile.label})")
        gatt = device.connectGatt(context, false, callback, BluetoothDevice.TRANSPORT_LE)
    }

    /**
     * Called by the scanner each time this instrument advertises. While reconnecting, a
     * direct connection started right after an advertising packet is much faster than
     * Android's background autoConnect, especially when the instrument is in its idle
     * advertising phase (one packet every ~7 s). This mirrors Sylvac's own library.
     */
    fun onAdvertSeen() {
        if (userDisconnect || state != State.RECONNECTING) return
        val now = SystemClock.elapsedRealtime()
        if (now - lastDirectAttempt < DIRECT_RETRY_MS) return
        lastDirectAttempt = now
        gatt?.close()
        log("Advertising seen, connecting directly")
        gatt = device.connectGatt(context, false, callback, BluetoothDevice.TRANSPORT_LE)
    }

    /**
     * Bluetooth was switched off. Android may report no disconnection for that (some phones
     * report none): the instrument would stay "ready" on a link that no longer exists.
     * So the link is dropped here, and [onBluetoothOn] starts the reconnection.
     */
    fun onBluetoothOff() {
        if (simulated || userDisconnect) return
        log("Bluetooth off: link lost")
        main.removeCallbacks(reconnect)
        gatt?.close()
        gatt = null
        resetLink()
        setState(State.RECONNECTING)
    }

    /** Bluetooth is back: reconnect at once, as after any lost link. */
    fun onBluetoothOn() {
        if (simulated || userDisconnect || state != State.RECONNECTING) return
        reconnectDelayMs = RECONNECT_MIN_MS
        lastDirectAttempt = 0
        main.removeCallbacks(reconnect)
        main.post(reconnect)
    }

    /**
     * UI test helper (see the `simulation` package): shows [value] without any Bluetooth link;
     * [answer] gives the reply to a command, or null for none. Call it again to change the value.
     */
    fun simulate(value: String, unit: String, answer: (command: String) -> String?) {
        simulatedAnswer = answer
        displayValue = value
        displayUnit = unit
        setState(State.READY)
    }

    /** User-requested disconnection: stops auto-reconnect and releases resources. */
    fun disconnect() {
        userDisconnect = true
        main.removeCallbacksAndMessages(null)
        gatt?.let {
            it.disconnect()
            it.close()
        }
        gatt = null
        resetLink()
        setState(State.DISCONNECTED)
        log("Disconnected by user")
        runCatching { context.unregisterReceiver(bondReceiver) }
    }

    /** Sends an SMS command (CR appended), split into 20-byte chunks. */
    fun sendCommand(command: String) {
        val ch = gatt?.getService(Sylvac.SMS_SERVICE)?.getCharacteristic(Sylvac.SMS_REMOTE_REQUEST)
        log("→ $command")
        main.removeCallbacks(replyTimeout)
        lastCommand = command
        lastReply.clear()
        simulatedAnswer?.let { answer ->
            notifyChanged()
            main.postDelayed(replyTimeout, REPLY_TIMEOUT_MS)
            answer(command)?.let(::addReply)
            return
        }
        if (state != State.READY || ch == null) {
            // Shown as the reply, so a tap never goes unanswered.
            addReply("not sent (${state.name.lowercase().replace('_', ' ')})")
            return
        }
        notifyChanged()
        val bytes = (command + "\r").toByteArray(Charsets.US_ASCII)
        bytes.toList().chunked(Sylvac.SMS_CHUNK).forEach {
            ops.addLast(Op.Write(ch, it.toByteArray(), "write '$command'"))
        }
        main.postDelayed(replyTimeout, REPLY_TIMEOUT_MS) // not cancelled if the link drops: the wait must end
        nextOp()
    }

    // ---------------------------------------------------------------- GATT callback

    private val callback = object : BluetoothGattCallback() {
        override fun onConnectionStateChange(g: BluetoothGatt, status: Int, newState: Int) {
            main.post { onConnectionState(g, status, newState) }
        }

        override fun onServicesDiscovered(g: BluetoothGatt, status: Int) {
            main.post { onServices(g, status) }
        }

        override fun onCharacteristicRead(g: BluetoothGatt, ch: BluetoothGattCharacteristic, value: ByteArray, status: Int) {
            main.post { onRead(ch, value, status) }
        }

        override fun onDescriptorRead(g: BluetoothGatt, d: BluetoothGattDescriptor, status: Int, value: ByteArray) {
            main.post { onDescriptorRead(d, value, status) }
        }

        override fun onDescriptorWrite(g: BluetoothGatt, d: BluetoothGattDescriptor, status: Int) {
            main.post { completeOp(status) }
        }

        override fun onCharacteristicWrite(g: BluetoothGatt, ch: BluetoothGattCharacteristic, status: Int) {
            main.post { completeOp(status) }
        }

        override fun onCharacteristicChanged(g: BluetoothGatt, ch: BluetoothGattCharacteristic, value: ByteArray) {
            main.post { handleValue(ch, value) }
        }
    }

    // ---------------------------------------------------------------- connection

    private fun onConnectionState(g: BluetoothGatt, status: Int, newState: Int) {
        if (g !== gatt) return // stale callback from a closed BluetoothGatt
        if (newState == BluetoothProfile.STATE_CONNECTED && status == BluetoothGatt.GATT_SUCCESS) {
            log("Connected")
            if (profile == Sylvac.AdvProfile.PAIR && device.bondState == BluetoothDevice.BOND_NONE) {
                setState(State.BONDING)
                log("PAIR profile: starting bonding (Just Works)")
                if (!device.createBond()) {
                    log("createBond refused, continuing without it")
                    discoverServices()
                }
            } else {
                discoverServices()
            }
            return
        }
        // Disconnected (or connection failed)
        log("Disconnected (status $status)")
        linkLost(g)
    }

    /** Gives up on a link that does not work, without waiting for Android to report it. */
    private fun dropLink() {
        val g = gatt ?: return
        log("Dropping the link to start again")
        g.disconnect()
        linkLost(g)
    }

    /** Closes [g] (its late callbacks are then ignored) and, unless the user disconnected, reconnects. */
    private fun linkLost(g: BluetoothGatt) {
        g.close()
        gatt = null
        resetLink()
        if (userDisconnect) return
        setState(State.RECONNECTING)
        val delay = reconnectDelayMs
        reconnectDelayMs = minOf(reconnectDelayMs * 2, RECONNECT_MAX_MS)
        main.postDelayed(reconnect, delay)
    }

    private val reconnect = Runnable {
        // Not while Bluetooth is off: onBluetoothOn runs this again.
        val bluetoothOn = context.getSystemService(BluetoothManager::class.java)?.adapter?.isEnabled == true
        if (!userDisconnect && gatt == null && bluetoothOn) {
            log("Reconnecting (waits until the instrument advertises)")
            // autoConnect = true: the stack connects whenever the device is seen again.
            gatt = device.connectGatt(context, true, callback, BluetoothDevice.TRANSPORT_LE)
        }
    }

    private fun resetLink() {
        main.removeCallbacks(opTimeout)
        main.removeCallbacks(startDiscovery)
        ops.clear()
        current = null
        servicesReady = false
        rxBuffer.clear()
    }

    private fun discoverServices() {
        setState(State.SETTING_UP)
        // Short delay: lets encryption / connection-parameter updates settle first.
        main.postDelayed(startDiscovery, 300)
    }

    private val startDiscovery = Runnable {
        if (gatt?.discoverServices() == true) {
            main.postDelayed(opTimeout, OP_TIMEOUT_MS)
        } else {
            log("discoverServices refused")
            dropLink()
        }
    }

    private fun onServices(g: BluetoothGatt, status: Int) {
        if (g !== gatt) return
        main.removeCallbacks(opTimeout)
        if (status != BluetoothGatt.GATT_SUCCESS) {
            log("Service discovery failed (status $status)")
            dropLink()
            return
        }
        servicesReady = true
        val sds = g.getService(Sylvac.SDS_SERVICE)
        val sms = g.getService(Sylvac.SMS_SERVICE)
        if (sds == null && sms == null) log("Warning: no SDS/SMS service, not a Sylvac instrument?")

        val measurement = sds?.getCharacteristic(Sylvac.SDS_MEASUREMENT)
        measurement?.getDescriptor(Sylvac.PRESENTATION_FORMAT)?.let {
            ops.addLast(Op.ReadDescriptor(it, "read Measurement format (0x2904)"))
        }
        g.getService(Sylvac.BATTERY_SERVICE)?.getCharacteristic(Sylvac.BATTERY_LEVEL)?.let {
            ops.addLast(Op.Read(it, "read Battery"))
            if (it.properties and BluetoothGattCharacteristic.PROPERTY_NOTIFY != 0) {
                ops.addLast(Op.Subscribe(it, false, "subscribe Battery"))
            }
        }
        sds?.getCharacteristic(Sylvac.SDS_PARAMETERS)?.let {
            if (it.properties and BluetoothGattCharacteristic.PROPERTY_READ != 0) {
                ops.addLast(Op.Read(it, "read Parameters"))
            }
            ops.addLast(Op.Subscribe(it, false, "subscribe Parameters"))
        }
        measurement?.let { ops.addLast(Op.Subscribe(it, false, "subscribe Measurement")) }
        sms?.getCharacteristic(Sylvac.SMS_REMOTE_RESPONSE)?.let {
            ops.addLast(Op.Subscribe(it, false, "subscribe RemoteResponse"))
        }
        sms?.getCharacteristic(Sylvac.SMS_DATA_SEND)?.let {
            ops.addLast(Op.Subscribe(it, true, "subscribe DataSend (indicate)"))
        }
        nextOp()
    }

    // ---------------------------------------------------------------- bonding

    private fun onBonded() {
        log("Bonded")
        if (state == State.BONDING) resumeSetup() // a late broadcast must not restart a link that moved on
    }

    private fun onBondFailed() {
        if (state != State.BONDING) return
        log("Bonding failed or was refused")
        // Drop the operation that was waiting for encryption and carry on.
        if (ops.firstOrNull()?.retried == true) ops.removeFirst()
        resumeSetup()
    }

    /** Bonding is over: discover the services, or carry on with the operations that waited for it. */
    private fun resumeSetup() {
        if (!servicesReady) discoverServices() else {
            setState(State.SETTING_UP)
            nextOp()
        }
    }

    // ---------------------------------------------------------------- operation queue

    private fun nextOp() {
        val g = gatt ?: return
        if (current != null || state == State.BONDING) return
        val op = ops.removeFirstOrNull()
        if (op == null) {
            if (state == State.SETTING_UP && servicesReady) {
                reconnectDelayMs = RECONNECT_MIN_MS // only now: a link that never gets ready backs off
                setState(State.READY)
                log("Ready")
            }
            return
        }
        current = op
        main.postDelayed(opTimeout, OP_TIMEOUT_MS)
        val started = when (op) {
            is Op.Read -> g.readCharacteristic(op.ch)
            is Op.ReadDescriptor -> g.readDescriptor(op.d)
            is Op.Subscribe -> subscribe(g, op)
            // SMS RemoteRequest only supports Write Without Response.
            is Op.Write -> g.writeCharacteristic(op.ch, op.data, BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE) ==
                BluetoothStatusCodes.SUCCESS
        }
        if (!started) {
            log("Could not start: ${op.label}")
            main.removeCallbacks(opTimeout)
            current = null
            main.post { nextOp() }
        }
    }

    private fun completeOp(status: Int) {
        val op = current ?: return
        main.removeCallbacks(opTimeout)
        current = null
        if (status != BluetoothGatt.GATT_SUCCESS) {
            if (status in AUTH_ERRORS && !op.retried) {
                // PAIR profile: the instrument requires encryption. Bond, then retry.
                op.retried = true
                ops.addFirst(op)
                log("${op.label}: security required (status $status)")
                when (device.bondState) {
                    BluetoothDevice.BOND_NONE -> {
                        setState(State.BONDING)
                        if (!device.createBond()) onBondFailed()
                    }
                    BluetoothDevice.BOND_BONDING -> setState(State.BONDING)
                    else -> main.postDelayed({ nextOp() }, 1000) // encryption still starting
                }
                return
            }
            log("Failed: ${op.label} (status $status)")
        }
        nextOp()
    }

    private fun subscribe(g: BluetoothGatt, op: Op.Subscribe): Boolean {
        val cccd = op.ch.getDescriptor(Sylvac.CCCD) ?: return false
        if (!g.setCharacteristicNotification(op.ch, true)) return false
        val value = if (op.indicate) {
            BluetoothGattDescriptor.ENABLE_INDICATION_VALUE
        } else {
            BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
        }
        return g.writeDescriptor(cccd, value) == BluetoothStatusCodes.SUCCESS
    }

    // ---------------------------------------------------------------- data

    private fun onRead(ch: BluetoothGattCharacteristic, value: ByteArray, status: Int) {
        if (status == BluetoothGatt.GATT_SUCCESS) handleValue(ch, value)
        completeOp(status)
    }

    private fun onDescriptorRead(d: BluetoothGattDescriptor, value: ByteArray, status: Int) {
        if (status == BluetoothGatt.GATT_SUCCESS && d.uuid == Sylvac.PRESENTATION_FORMAT) {
            format = Sylvac.PresentationFormat.parse(value)
            format?.let { log("Format: unit 0x%04X, exponent %d".format(it.unit, it.exponent)) }
            refreshValue()
        }
        completeOp(status)
    }

    private fun handleValue(ch: BluetoothGattCharacteristic, value: ByteArray) {
        when (ch.uuid) {
            Sylvac.SDS_MEASUREMENT -> {
                lastRaw = Sylvac.int32le(value) ?: return
                refreshValue()
            }
            Sylvac.SDS_PARAMETERS -> {
                val raw = Sylvac.uint16le(value) ?: return
                params = Sylvac.Parameters(raw)
                log("Parameters 0x%04X: %s %s".format(raw, params?.unitLabel ?: "?", params?.modeLabel ?: ""))
                refreshValue()
            }
            Sylvac.BATTERY_LEVEL -> {
                batteryPercent = value.firstOrNull()?.toInt()?.and(0xFF)
                log("Battery: $batteryPercent %")
                notifyChanged()
            }
            Sylvac.SMS_REMOTE_RESPONSE -> onResponseChunk(value)
            Sylvac.SMS_DATA_SEND -> {
                val text = String(value, Charsets.US_ASCII).trim('\r', '\n', '\u0000', ' ')
                buttonText = text
                log("Button value: $text")
                notifyChanged()
            }
        }
    }

    /** RemoteResponse: ASCII chunks, a response is complete when it ends with CR. */
    private fun onResponseChunk(value: ByteArray) {
        rxBuffer.append(String(value, Charsets.US_ASCII))
        while (true) {
            val end = rxBuffer.indexOf("\r")
            if (end < 0) break
            val line = rxBuffer.substring(0, end).trim('\n', '\u0000', ' ')
            rxBuffer.delete(0, end + 1)
            if (line.isNotEmpty()) addReply(line)
        }
    }

    private fun addReply(line: String) {
        log("← $line")
        lastReply.addLast(line)
        while (lastReply.size > MAX_REPLY_LINES) lastReply.removeFirst()
        notifyChanged()
    }

    private fun refreshValue() {
        val raw = lastRaw ?: return
        Sylvac.formatMeasurement(raw, format, params).let { (v, u) ->
            displayValue = v
            displayUnit = u
        }
        notifyChanged()
    }

    // ---------------------------------------------------------------- helpers

    private fun setState(s: State) {
        state = s
        notifyChanged()
    }

    private fun notifyChanged() = listener.onInstrumentChanged(this)

    /** Android's own log: read it with `adb logcat -s SylvacBle`. */
    private fun log(line: String) {
        Log.i(Sylvac.TAG, "$name ${address.takeLast(5)}: $line")
    }

    private companion object {
        const val OP_TIMEOUT_MS = 10_000L
        const val REPLY_TIMEOUT_MS = 3_000L
        const val RECONNECT_MIN_MS = 1_000L
        const val RECONNECT_MAX_MS = 15_000L
        const val DIRECT_RETRY_MS = 12_000L
        const val MAX_REPLY_LINES = 8

        /** Insufficient authentication / encryption, and Android's GATT_AUTH_FAIL. */
        val AUTH_ERRORS = setOf(
            BluetoothGatt.GATT_INSUFFICIENT_AUTHENTICATION,
            BluetoothGatt.GATT_INSUFFICIENT_ENCRYPTION,
            137,
        )
    }
}
