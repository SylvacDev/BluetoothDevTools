package com.example.bluetooth.simulation

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.content.Intent
import android.os.Handler
import android.os.Looper
import android.util.Log
import com.example.bluetooth.Sylvac
import com.example.bluetooth.SylvacInstrument
import java.util.Locale

/**
 * UI test helper: fake instruments, without any Bluetooth traffic.
 * `adb shell am start -n com.example.bluetooth/.MainActivity --ei simNearby 50 --ei simConnected 13`
 *
 * Fake addresses are locally administered (02:5C:…). A fake connected instrument shows a value
 * that climbs slowly and answers `?`, `ID?` and `VER?`; any other command gets no reply.
 * Nothing else in the app is needed to understand the real Bluetooth code: the only hooks are
 * MainActivity.addSimulated and SylvacInstrument.simulate.
 */
object Simulation {
    private const val EXTRA_NEARBY = "simNearby"
    private const val EXTRA_CONNECTED = "simConnected"
    private const val ADDRESS_PREFIX = "02:5C:00:00:"
    private const val FLAGS_SIMPLE = 0x06 // advertised as the SIMPLE profile
    private const val UNIT = "mm"
    private const val TICK_MS = 1_000L // the value moves by STEP every tick…
    private const val STEP = 0.01
    private const val RANGE = 25.0 // …and starts again at 0 here
    private val MODELS = listOf("SY289", "SY295", "SYSW293")

    private val main = Handler(Looper.getMainLooper())

    /** One fake instrument: [connected] ones go to the instrument list, the others to the scanner. */
    class Fake(val device: BluetoothDevice, val name: String, val flags: Int, val value: String?, val connected: Boolean)

    /** The fake instruments asked for by the launch extras of [intent] (none without them). */
    fun fakes(intent: Intent, adapter: BluetoothAdapter?): List<Fake> {
        val nearby = intent.getIntExtra(EXTRA_NEARBY, 0)
        val connected = intent.getIntExtra(EXTRA_CONNECTED, 0)
        if (adapter == null || nearby + connected <= 0) return emptyList()
        Log.i(Sylvac.TAG, "Simulated $nearby nearby and $connected connected instruments")
        return List(nearby + connected) { i ->
            Fake(
                device = adapter.getRemoteDevice("$ADDRESS_PREFIX%02X:%02X".format(i / 256, i % 256)),
                name = MODELS[i % MODELS.size],
                flags = FLAGS_SIMPLE,
                value = if (i % 7 == 3) null else format((i * 37 % 2500) / 100.0), // some advertise no value
                connected = i < connected,
            )
        }
    }

    fun isSimulated(address: String) = address.startsWith(ADDRESS_PREFIX)

    /** Makes [instrument] a fake one, starting at [startValue], until it is disconnected. */
    fun drive(instrument: SylvacInstrument, startValue: String?) {
        var value = startValue?.toDoubleOrNull() ?: 0.0
        val answer = { command: String ->
            when (command.uppercase(Locale.ROOT)) {
                "?" -> "%+09.4f".format(Locale.ROOT, value)
                "ID?" -> instrument.name
                "VER?" -> "SIM 1.0"
                else -> null
            }
        }
        instrument.simulate(format(value), UNIT, answer)
        main.postDelayed(object : Runnable {
            override fun run() {
                if (instrument.state == SylvacInstrument.State.DISCONNECTED) return // removed
                value = (value + STEP) % RANGE
                instrument.simulate(format(value), UNIT, answer)
                main.postDelayed(this, TICK_MS)
            }
        }, TICK_MS)
    }

    private fun format(value: Double) = "%.4f".format(Locale.ROOT, value)
}
