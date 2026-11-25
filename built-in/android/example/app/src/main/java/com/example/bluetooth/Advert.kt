package com.example.bluetooth

import android.bluetooth.BluetoothDevice

/** Information about an advertising device. */
data class Advert(
    val device: BluetoothDevice,
    var name: String,
    var flags: Int,
    var value: String?,
) {
    val profile get() = Sylvac.advProfile(name, flags)
}
