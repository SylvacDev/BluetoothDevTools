package com.example.bluetooth

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.app.AlertDialog
import android.app.Dialog
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.content.res.ColorStateList
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.ParcelUuid
import android.provider.Settings
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.WindowInsets
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.example.bluetooth.simulation.Simulation
import com.example.bluetooth.ui.BOLD
import com.example.bluetooth.ui.CappedScrollView
import com.example.bluetooth.ui.GAP
import com.example.bluetooth.ui.MEDIUM
import com.example.bluetooth.ui.PAD
import com.example.bluetooth.ui.ROW_PAD
import com.example.bluetooth.ui.bottomSheet
import com.example.bluetooth.ui.dp
import com.example.bluetooth.ui.filledButton
import com.example.bluetooth.ui.horizontal
import com.example.bluetooth.ui.listContainer
import com.example.bluetooth.ui.overlay
import com.example.bluetooth.ui.text
import com.example.bluetooth.ui.textButton
import com.example.bluetooth.ui.themeColor
import com.example.bluetooth.ui.vertical

/**
 * Dashboard of Sylvac instruments: one row per connected instrument (tap for its command
 * console, swipe left to remove), and an "Add instrument" button that opens the scanner, where
 * nearby instruments are boxes in order of arrival (tap to connect). Console and scanner are
 * bottom sheets.
 *
 * UI is built in code with plain framework views (no AndroidX), with the generic builders and
 * widgets of the `ui` package. Colours come from the theme, so the app follows the phone's
 * light / dark setting.
 * Debug helper: `adb shell am start -n com.example.bluetooth/.MainActivity --es connect AA:BB:CC:DD:EE:FF,…`
 * connects the listed addresses as soon as they are seen in the scan.
 */
@SuppressLint("MissingPermission", "SetTextI18n")
class MainActivity : Activity(), SylvacInstrument.Listener {

    private val main = Handler(Looper.getMainLooper())
    private val adapter: BluetoothAdapter? by lazy { getSystemService(BluetoothManager::class.java)?.adapter }

    private val adverts = linkedMapOf<String, Advert>()
    private val instruments = linkedMapOf<String, SylvacInstrument>()
    private val autoConnect = mutableSetOf<String>()
    private var scanning = false
    private var visible = false // between onStart and onStop
    private var scannerWanted = false // "Add instrument" was tapped and waits for a permission or for Bluetooth

    // Theme colours, resolved in onCreate.
    private var accent = 0
    private var textPrimary = 0
    private var textSecondary = 0
    private var colorError = 0

    private lateinit var subtitle: TextView
    private lateinit var connectedEmpty: TextView
    private lateinit var connectedList: LinearLayout
    private lateinit var scanner: Dialog
    private lateinit var nearbyLabel: TextView
    private lateinit var nearbyList: LinearLayout
    private lateinit var nearbyEmpty: TextView
    private val rows = hashMapOf<String, InstrumentRow>()
    private val tiles = linkedMapOf<String, NearbyTile>()
    private val console by lazy { InstrumentConsole(this, ::removeWithUndo) }

    // ---------------------------------------------------------------- lifecycle

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        accent = themeColor(android.R.attr.colorAccent)
        textPrimary = themeColor(android.R.attr.textColorPrimary)
        textSecondary = themeColor(android.R.attr.textColorSecondary)
        colorError = themeColor(android.R.attr.colorError)
        buildUi()
        buildScanner()
        intent.getStringExtra(EXTRA_CONNECT)?.split(',', ';', ' ')
            ?.map { normalizeAddress(it) }?.filter { it.length == 17 }
            ?.let { autoConnect += it }
        if (autoConnect.isNotEmpty()) log("Auto-connect: ${autoConnect.joinToString()}")
        addSimulated()
        // Also while the app is in the background: the connected instruments must know.
        // Exported, like any receiver of a broadcast sent by Android itself (only Android can send this one).
        registerReceiver(bluetoothSwitched, IntentFilter(BluetoothAdapter.ACTION_STATE_CHANGED), RECEIVER_EXPORTED)
        if (!hasPermissions()) requestPermissions(PERMISSIONS, REQUEST_PERMISSIONS)
    }

    override fun onStart() {
        super.onStart()
        visible = true
        main.post(refreshTick)
        updateScan()
        openScannerIfWanted() // back from the app's settings page, with the permission
    }

    override fun onStop() {
        super.onStop()
        visible = false
        main.removeCallbacks(refreshTick)
        updateScan()
    }

    override fun onDestroy() {
        super.onDestroy()
        unregisterReceiver(bluetoothSwitched)
        instruments.values.toList().forEach { it.disconnect() }
    }

    // ---------------------------------------------------------------- permissions, Bluetooth on / off

    private fun hasPermissions() = PERMISSIONS.all { checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED }

    override fun onRequestPermissionsResult(requestCode: Int, perms: Array<out String>, results: IntArray) {
        super.onRequestPermissionsResult(requestCode, perms, results)
        when {
            hasPermissions() -> {
                updateScan()
                if (scannerWanted) openScanner() // goes on to Bluetooth, then to the scanner
            }
            scannerWanted -> explainDenied()
            else -> log("Bluetooth permissions denied") // asked at launch: no message, "Add instrument" asks again
        }
    }

    /**
     * "Add instrument" was refused the permission. Android shows its own dialog only twice: after
     * that a request is refused at once, without anything on screen, and only the app's settings
     * page can grant the permission. So say why it is needed, and offer the way that still works.
     */
    private fun explainDenied() {
        scannerWanted = false
        val canAskAgain = PERMISSIONS.any { shouldShowRequestPermissionRationale(it) }
        log("Bluetooth permissions denied" + if (canAskAgain) "" else " (Android no longer asks)")
        AlertDialog.Builder(this)
            .setMessage(
                "To find and connect instruments, this app needs the \"Nearby devices\" permission." +
                    if (canAskAgain) "" else "\n\nAllow it in the app's settings, under Permissions.",
            )
            .setNegativeButton("Cancel", null)
            .setPositiveButton(if (canAskAgain) "Ask again" else "Open settings") { _, _ ->
                if (canAskAgain) {
                    openScanner()
                } else {
                    scannerWanted = true // the scanner opens on the way back, see onStart
                    startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", packageName, null)))
                }
            }
            .show()
    }

    /** Answer to "turn on Bluetooth?", asked by openScanner. */
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != REQUEST_ENABLE) return
        // Accepted: opens the scanner, here or in bluetoothSwitched if Bluetooth is still turning on.
        if (resultCode == RESULT_OK) openScannerIfWanted() else scannerWanted = false
    }

    /**
     * Bluetooth turned on or off, from the app or from the phone's quick settings. Neither a
     * running scan nor (on some phones) a connected instrument gets a callback for that: without
     * this, the app would still believe it scans and is connected once Bluetooth is off, and
     * would start nothing again once it is back on.
     */
    private val bluetoothSwitched = object : BroadcastReceiver() {
        override fun onReceive(c: Context, intent: Intent) {
            val state = intent.getIntExtra(BluetoothAdapter.EXTRA_STATE, BluetoothAdapter.ERROR)
            if (state != BluetoothAdapter.STATE_ON && state != BluetoothAdapter.STATE_OFF) return // turning on / off
            log("Bluetooth turned ${if (state == BluetoothAdapter.STATE_ON) "on" else "off"}")
            if (state == BluetoothAdapter.STATE_OFF) {
                forgetNearby() // they can't be connected now: show "Bluetooth is off" instead
                instruments.values.forEach { it.onBluetoothOff() }
            } else {
                instruments.values.forEach { it.onBluetoothOn() }
            }
            updateScan()
            refreshNearby()
            openScannerIfWanted()
        }
    }

    // ---------------------------------------------------------------- scanning

    private val scanCallback = object : ScanCallback() {
        override fun onScanResult(callbackType: Int, result: ScanResult) = onAdvert(result)
        override fun onBatchScanResults(results: MutableList<ScanResult>) = results.forEach(::onAdvert)
        override fun onScanFailed(errorCode: Int) {
            log("Scan failed (error $errorCode)")
            scanning = false
            main.removeCallbacks(restartScan)
        }
    }

    /**
     * The scan runs only when it is useful (it costs battery): while the scanner is open, while an
     * instrument waits to reconnect (it is connected as soon as it advertises, see
     * SylvacInstrument.onAdvertSeen), or while a `connect` launch extra waits for its instruments.
     */
    private fun updateScan() {
        val needed = scanner.isShowing ||
            instruments.values.any { it.state == SylvacInstrument.State.RECONNECTING } ||
            autoConnect.any { it !in instruments }
        if (needed && visible && hasPermissions() && adapter?.isEnabled == true) startScan() else stopScan()
    }

    private fun startScan() {
        if (scanning) return
        val settings = ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()
        // No ScanFilter: Sylvac instruments advertise no service UUID list, so we filter by name in onAdvert.
        adapter?.bluetoothLeScanner?.startScan(null, settings, scanCallback) ?: return
        scanning = true
        log("Scan started")
        main.postDelayed(restartScan, SCAN_RESTART_MS)
    }

    /**
     * Android silently stops delivering results to a scan that runs too long (it becomes
     * "opportunistic": after 30 min on stock Android, 5 min on Samsung), without any callback.
     * So a scan that is still needed is stopped and started again before that.
     */
    private val restartScan = Runnable {
        log("Scan running for long, restarting it")
        stopScan()
        updateScan()
    }

    private fun stopScan() {
        if (!scanning) return
        main.removeCallbacks(restartScan)
        adapter?.bluetoothLeScanner?.stopScan(scanCallback)
        scanning = false
        log("Scan stopped")
    }

    /**
     * "Add instrument": asks for what scanning needs (the permissions, then Bluetooth on), then
     * shows the scanner. While it waits for an answer, [scannerWanted] is set, and the answer
     * calls this again: one tap is enough.
     */
    private fun openScanner() {
        scannerWanted = true
        when {
            !hasPermissions() -> requestPermissions(PERMISSIONS, REQUEST_PERMISSIONS)
            adapter?.isEnabled != true -> startActivityForResult(Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE), REQUEST_ENABLE)
            else -> {
                scannerWanted = false
                forgetNearby() // a fresh list each time: only what advertises now
                scanner.show()
            }
        }
    }

    /** Empties the Nearby list (the simulated boxes stay). */
    private fun forgetNearby() {
        adverts.keys.retainAll { it in instruments || Simulation.isSimulated(it) }
    }

    /** Opens the scanner if "Add instrument" waits for it and nothing is missing any more (so this never asks anything). */
    private fun openScannerIfWanted() {
        if (scannerWanted && visible && hasPermissions() && adapter?.isEnabled == true) openScanner()
    }

    private fun onAdvert(result: ScanResult) {
        val record = result.scanRecord
        // A bonded PAIR instrument advertises only the shortened name "SY": prefer the longer
        // name Android remembered from an earlier connection, if any.
        val name = listOfNotNull(record?.deviceName, result.device.name).maxByOrNull { it.length }
        val sdsData = record?.getServiceData(SDS_UUID)
        if (!Sylvac.isSylvacName(name) && sdsData == null) return
        val address = result.device.address
        // linkedMapOf keeps insertion order: the Nearby list shows instruments in order of arrival.
        val advert = adverts.getOrPut(address) { Advert(result.device, name ?: "?", -1, null) }
        if (name != null && name.length >= advert.name.length) advert.name = name
        advert.flags = record?.advertiseFlags ?: -1
        advert.value = Sylvac.formatAdvValue(sdsData) ?: advert.value

        instruments[address]?.onAdvertSeen()
        if (address in autoConnect && address !in instruments) {
            connect(advert)
            updateScan()
        }
    }

    // ---------------------------------------------------------------- instruments

    private fun connect(advert: Advert) {
        val address = advert.device.address
        if (address in instruments) return
        if (advert.profile == Sylvac.AdvProfile.HID) {
            log("${advert.name}: HID profile is a keyboard, switch the instrument to SIMPLE or PAIR")
            return
        }
        if (advert.profile == Sylvac.AdvProfile.PAIR_BONDED && advert.device.bondState != BluetoothDevice.BOND_BONDED) {
            log("${advert.name}: bonded to another device. Do a Bluetooth reset (BT RST) on the instrument if it refuses the connection.")
        }
        addInstrument(SylvacInstrument(applicationContext, advert.device, advert.name, advert.profile, this)).connect()
    }

    /** Adds a row for [instrument] at [index] (default: last) and returns the instrument. */
    private fun addInstrument(instrument: SylvacInstrument, index: Int = -1): SylvacInstrument {
        instruments[instrument.address] = instrument
        val row = InstrumentRow(this, instrument, onOpen = { console.show(instrument) }, onRemove = { removeWithUndo(instrument) })
        rows[instrument.address] = row
        connectedList.addView(row.view, index)
        refreshHeader()
        refreshNearby()
        return instrument
    }

    /** Swipe (or the accessibility action) "Remove": disconnects, and offers Undo in place of the row for 4 s. */
    private fun removeWithUndo(instrument: SylvacInstrument) {
        val removed = rows[instrument.address]?.view
        val index = connectedList.indexOfChild(removed)
        val height = removed?.height ?: 0
        disconnect(instrument)
        lateinit var undoRow: LinearLayout
        undoRow = horizontal().apply {
            setPadding(dp(PAD), 0, dp(GAP), 0)
            minimumHeight = height // same height as the removed row: the list doesn't jump
            addView(text(15f, textSecondary).apply { text = "${instrument.name} removed" }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            addView(textButton("Undo") {
                val i = connectedList.indexOfChild(undoRow)
                connectedList.removeView(undoRow)
                val again = SylvacInstrument(applicationContext, instrument.device, instrument.name, instrument.profile, this@MainActivity)
                addInstrument(again, i)
                if (instrument.simulated) Simulation.drive(again, instrument.displayValue) else again.connect()
            })
        }
        connectedList.addView(undoRow, index)
        refreshHeader()
        main.postDelayed({
            connectedList.removeView(undoRow)
            refreshHeader()
        }, UNDO_MS)
    }

    private fun disconnect(instrument: SylvacInstrument) {
        autoConnect -= instrument.address
        instrument.disconnect()
        instruments.remove(instrument.address)
        rows.remove(instrument.address)?.let { connectedList.removeView(it.view) }
        console.close(instrument)
        refreshHeader()
        refreshNearby()
        updateScan()
    }

    override fun onInstrumentChanged(instrument: SylvacInstrument) {
        rows[instrument.address]?.bind()
        console.bind(instrument)
        refreshHeader()
        updateScan() // reconnecting instruments need the scan
    }

    // ---------------------------------------------------------------- UI: layout

    private fun buildUi() {
        val root = vertical()

        // App bar: title + status
        subtitle = text(13f, textSecondary)
        root.addView(vertical().apply {
            setPadding(dp(PAD), dp(GAP), 0, dp(ROW_PAD))
            addView(text(22f, textPrimary, BOLD).apply { setText(R.string.app_name) })
            addView(subtitle)
        })

        // Connected instruments: as tall as the list, scrolling once it would squeeze the button.
        connectedList = listContainer()
        root.addView(CappedScrollView(this, dp(ADD_AREA)).apply { addView(connectedList) })

        // "Add instrument" opens the scanner; it sits in the middle of the height the list leaves.
        connectedEmpty = text(15f, textSecondary).apply {
            text = "Connect your first instrument"
            setPadding(0, 0, 0, dp(ROW_PAD))
        }
        root.addView(vertical().apply {
            gravity = Gravity.CENTER
            addView(connectedEmpty, LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT))
            addView(filledButton("+  Add instrument") { openScanner() }.apply {
                textSize = 16f
                backgroundTintList = null
                background = RippleDrawable(
                    ColorStateList.valueOf(overlay(0.16f)),
                    GradientDrawable().apply { setColor(accent); cornerRadius = dp(ADD_HEIGHT) / 2f }, // pill
                    null,
                )
                setPadding(dp(PAD * 2), 0, dp(PAD * 2), 0)
            }, LinearLayout.LayoutParams(WRAP_CONTENT, dp(ADD_HEIGHT)))
        }, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f))

        // targetSdk 35+ draws edge-to-edge: keep content clear of the system bars.
        root.setOnApplyWindowInsetsListener { v, insets ->
            val bars = insets.getInsets(WindowInsets.Type.systemBars())
            v.setPadding(bars.left + dp(PAD), bars.top + dp(GAP), bars.right + dp(PAD), bars.bottom + dp(GAP))
            insets
        }
        setContentView(root)
        refreshHeader()
    }

    /**
     * The scanner: a bottom sheet with the Nearby boxes, emptied each time it opens. It stays open to add several
     * instruments; scanning runs while it is showing.
     */
    private fun buildScanner() {
        nearbyLabel = text(12f, textSecondary, MEDIUM).apply {
            letterSpacing = 0.08f
            setPadding(dp(PAD), dp(ROW_PAD), dp(PAD), dp(ROW_PAD))
        }
        nearbyList = vertical()
        nearbyEmpty = text(13f, textSecondary).apply { setPadding(dp(PAD), dp(ROW_PAD), dp(PAD), dp(ROW_PAD)) }
        scanner = bottomSheet(
            nearbyLabel,
            ScrollView(this).apply {
                addView(vertical().apply {
                    addView(nearbyEmpty)
                    addView(nearbyList)
                })
            },
            (resources.displayMetrics.heightPixels * SCANNER_HEIGHT).toInt(),
        ) { updateScan(); refreshNearby() }
        scanner.setOnDismissListener { updateScan() }
    }

    // ---------------------------------------------------------------- UI: refresh

    private val refreshTick = object : Runnable {
        override fun run() {
            refreshNearby()
            main.postDelayed(this, 500)
        }
    }

    private fun refreshHeader() {
        // "Connected" = ready to measure (green dot); the others are still on their way.
        val ready = instruments.values.count { it.state == SylvacInstrument.State.READY }
        val waiting = instruments.size - ready
        subtitle.text = if (waiting == 0) "$ready connected" else "$ready connected, $waiting to connect"
        subtitle.visibility = if (instruments.isEmpty()) View.GONE else View.VISIBLE
        connectedEmpty.visibility = if (connectedList.childCount == 0) View.VISIBLE else View.GONE
    }

    /** Nearby boxes: instruments not connected, in order of arrival (newest last), 2 per row. */
    private fun refreshNearby() {
        val visible = adverts.values.filter { it.device.address !in instruments }
        val addresses = visible.map { it.device.address }.toSet()

        var changed = tiles.keys.retainAll(addresses)
        visible.forEach { a ->
            val address = a.device.address
            tiles.getOrPut(address) {
                changed = true
                NearbyTile(this) { adverts[address]?.let { connect(it) } }
            }.bind(a)
        }
        if (changed) {
            nearbyList.removeAllViews()
            visible.chunked(2).forEach { pair ->
                val row = horizontal()
                pair.forEachIndexed { i, a ->
                    val tile = tiles.getValue(a.device.address).root
                    (tile.parent as? ViewGroup)?.removeView(tile)
                    row.addView(tile, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f).apply { if (i == 1) marginStart = dp(GAP) })
                }
                if (pair.size == 1) row.addView(View(this), LinearLayout.LayoutParams(0, 0, 1f).apply { marginStart = dp(GAP) })
                nearbyList.addView(row, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { bottomMargin = dp(GAP) })
            }
        }
        nearbyLabel.text = "NEARBY  ${visible.size}"
        nearbyList.visibility = if (visible.isEmpty()) View.GONE else View.VISIBLE
        nearbyEmpty.visibility = if (visible.isEmpty()) View.VISIBLE else View.GONE
        nearbyEmpty.text = when {
            scanning -> "Searching for Sylvac instruments…"
            adapter?.isEnabled != true -> "Bluetooth is off. Turn it on to search for instruments."
            else -> "Not searching: the scan could not start. Close this panel and open it again to retry."
        }
    }

    // ---------------------------------------------------------------- UI: helpers

    /** Android's own log: read it with `adb logcat -s SylvacBle`. */
    private fun log(line: String) {
        Log.i(Sylvac.TAG, line)
    }

    /** UI test helper: adds the fake instruments asked for by the launch extras (see [Simulation]). */
    private fun addSimulated() {
        Simulation.fakes(intent, adapter).forEach { fake ->
            val advert = Advert(fake.device, fake.name, fake.flags, fake.value)
            if (fake.connected) {
                val instrument = SylvacInstrument(applicationContext, fake.device, advert.name, advert.profile, this)
                Simulation.drive(addInstrument(instrument), fake.value)
            } else {
                adverts[fake.device.address] = advert
            }
        }
        refreshNearby()
    }

    private fun normalizeAddress(s: String): String {
        val hex = s.filter { it.isLetterOrDigit() }.uppercase()
        return if (hex.length == 12) hex.chunked(2).joinToString(":") else s.uppercase()
    }

    private companion object {
        const val EXTRA_CONNECT = "connect"
        const val REQUEST_PERMISSIONS = 1
        const val REQUEST_ENABLE = 2
        // Sizes (dp), on top of the spacing values of ui/Ui.kt.
        const val SCANNER_HEIGHT = 0.6 // fraction of the screen height
        const val ADD_HEIGHT = 56 // the "Add instrument" pill
        const val ADD_AREA = 88 // height kept for it under a full list
        const val SCAN_RESTART_MS = 4 * 60_000L // under the shortest known scan timeout (Samsung: 5 min)
        const val UNDO_MS = 4_000L // how long "… removed · Undo" stays
        val PERMISSIONS = arrayOf(Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT)
        val SDS_UUID = ParcelUuid(Sylvac.SDS_SERVICE)
    }
}
