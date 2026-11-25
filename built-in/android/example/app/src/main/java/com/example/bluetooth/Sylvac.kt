package com.example.bluetooth

import java.math.BigDecimal
import java.math.RoundingMode
import java.util.UUID

/**
 * Sylvac Bluetooth protocol constants and decoders.
 *
 * Source: BluetoothDevTools/gatt-profiles (README, SDS and SMS specifications).
 * - SDS (Simple Data Service) 0x5000 on the Bluetooth SIG base: measured value.
 * - SMS (Sylvac Metrology Service) 0x5000 on the Sylvac base: ASCII commands.
 */
object Sylvac {
    /** Logcat tag: `adb logcat -s SylvacBle`. */
    const val TAG = "SylvacBle"

    private fun sig(short: Int): UUID =
        UUID.fromString("0000%04X-0000-1000-8000-00805F9B34FB".format(short))

    private fun sylvac(short: Int): UUID =
        UUID.fromString("C1B2%04X-CAAF-6D0E-4C33-7DAE30052840".format(short))

    // Simple Data Service
    val SDS_SERVICE: UUID = sig(0x5000)
    val SDS_MEASUREMENT: UUID = sig(0x5020)
    val SDS_PARAMETERS: UUID = sig(0x5021)

    // Sylvac Metrology Service
    val SMS_SERVICE: UUID = sylvac(0x5000)
    val SMS_DATA_SEND: UUID = sylvac(0x5010)
    val SMS_REMOTE_REQUEST: UUID = sylvac(0x5012)
    val SMS_REMOTE_RESPONSE: UUID = sylvac(0x5013)

    // Standard services
    val BATTERY_SERVICE: UUID = sig(0x180F)
    val BATTERY_LEVEL: UUID = sig(0x2A19)

    // Descriptors
    val CCCD: UUID = sig(0x2902)
    val PRESENTATION_FORMAT: UUID = sig(0x2904)

    /** SMS strings are split into chunks of at most 20 bytes (ATT MTU 23). */
    const val SMS_CHUNK = 20

    private val NAME_PREFIXES = listOf("SY", "MTY")

    fun isSylvacName(name: String?): Boolean =
        name != null && NAME_PREFIXES.any { name.startsWith(it) }

    /** Profile guessed from the advertising flags (SPEC Table 1). */
    enum class AdvProfile(val label: String) {
        SIMPLE("SIMPLE"),
        PAIR("PAIR"),
        PAIR_BONDED("PAIR (bonded)"),
        HID("HID (unsupported)"),
        UNKNOWN("?"),
    }

    fun advProfile(name: String?, flags: Int): AdvProfile = when {
        name != null && name.endsWith("HID") -> AdvProfile.HID
        flags < 0 -> AdvProfile.UNKNOWN
        flags and 0x01 != 0 -> AdvProfile.PAIR            // 0x05: LE Limited Discoverable
        flags and 0x02 != 0 -> AdvProfile.SIMPLE          // 0x06: LE General Discoverable
        else -> AdvProfile.PAIR_BONDED                    // 0x04: not discoverable
    }

    fun int32le(b: ByteArray, off: Int = 0): Int? {
        if (b.size < off + 4) return null
        return (b[off].toInt() and 0xFF) or
            ((b[off + 1].toInt() and 0xFF) shl 8) or
            ((b[off + 2].toInt() and 0xFF) shl 16) or
            ((b[off + 3].toInt() and 0xFF) shl 24)
    }

    fun uint16le(b: ByteArray, off: Int = 0): Int? {
        if (b.size < off + 2) return null
        return (b[off].toInt() and 0xFF) or ((b[off + 1].toInt() and 0xFF) shl 8)
    }

    fun isInvalid(raw: Int) = raw == Int.MAX_VALUE || raw == Int.MIN_VALUE

    /** Value broadcast in the 0x5000 service data: display units x 10^-4, no unit. */
    fun formatAdvValue(serviceData: ByteArray?): String? {
        val raw = serviceData?.let { int32le(it) } ?: return null
        if (isInvalid(raw)) return "---"
        return BigDecimal.valueOf(raw.toLong(), 4).toPlainString()
    }

    /** Characteristic Presentation Format descriptor (0x2904). */
    data class PresentationFormat(val exponent: Int, val unit: Int) {
        companion object {
            fun parse(b: ByteArray): PresentationFormat? {
                if (b.size < 4) return null
                return PresentationFormat(exponent = b[1].toInt(), unit = uint16le(b, 2)!!)
            }
        }
    }

    /** SDS Parameters bitmap: unit (15-12), resolution (11-8), mode (1-0). */
    data class Parameters(val raw: Int) {
        val unit get() = (raw shr 12) and 0xF
        val resolution get() = (raw shr 8) and 0xF
        val mode get() = raw and 0x3

        val unitLabel: String?
            get() = when (unit) {
                1 -> "mm"; 2 -> "in"; 4 -> "rad"; 5 -> "°"; 6 -> "°'"; else -> null
            }

        val modeLabel: String?
            get() = when (mode) {
                1 -> "MIN"; 2 -> "MAX"; 3 -> "DELTA"; else -> null
            }

        /** Number of decimals to display, from the resolution code. */
        val decimals: Int?
            get() = when {
                resolution !in 1..5 -> null
                unit == 1 -> resolution                       // 0.1 mm .. 0.00001 mm
                unit == 2 -> resolution + 2                   // 0.005 in .. 0.000005 in
                unit == 5 && resolution == 2 -> 2             // 0.01°
                unit == 4 && resolution == 2 -> 4             // 0.0001 rad
                else -> null
            }
    }

    private const val UNIT_METRE = 0x2701
    private const val UNIT_RADIAN = 0x2720
    private const val UNIT_DEGREE = 0x2763

    /**
     * Converts an SDS Measurement to display text and unit, e.g. ("20.580", "mm").
     * value = raw x 10^exponent in the 0x2904 unit, then converted to the
     * display unit and resolution of the Parameters characteristic (if known).
     */
    fun formatMeasurement(raw: Int, format: PresentationFormat?, params: Parameters?): Pair<String, String> {
        if (isInvalid(raw)) return "---" to ""
        if (format == null) return raw.toString() to "raw"
        val si = BigDecimal.valueOf(raw.toLong(), -format.exponent)
        val (value, unit, fallbackDecimals) = when (format.unit) {
            UNIT_METRE -> if (params?.unit == 2) {
                Triple(si.divide(BigDecimal("0.0254"), 10, RoundingMode.HALF_UP), "in", maxOf(0, -format.exponent - 1))
            } else {
                Triple(si.movePointRight(3), "mm", maxOf(0, -format.exponent - 3))
            }
            UNIT_RADIAN -> if (params?.unit == 5) {
                Triple(si.multiply(BigDecimal(180.0 / Math.PI)), "°", maxOf(0, -format.exponent - 2))
            } else {
                Triple(si, "rad", maxOf(0, -format.exponent))
            }
            UNIT_DEGREE -> Triple(si, "°", maxOf(0, -format.exponent))
            else -> Triple(si, "", maxOf(0, -format.exponent))
        }
        val decimals = params?.decimals ?: fallbackDecimals
        return value.setScale(decimals, RoundingMode.HALF_UP).toPlainString() to unit
    }
}
