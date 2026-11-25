package com.example.bluetooth

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.text.SpannableString
import android.text.style.ForegroundColorSpan
import android.text.style.RelativeSizeSpan
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.LinearLayout
import com.example.bluetooth.ui.GAP
import com.example.bluetooth.ui.MEDIUM
import com.example.bluetooth.ui.PAD
import com.example.bluetooth.ui.ROW_PAD
import com.example.bluetooth.ui.dp
import com.example.bluetooth.ui.horizontal
import com.example.bluetooth.ui.text
import com.example.bluetooth.ui.themeColor
import com.example.bluetooth.ui.vertical

private const val DOT = 8 // status dot (dp)
private val COLOR_OK = Color.rgb(61, 190, 122)
private val COLOR_BUSY = Color.rgb(224, 160, 48)

/** What is shown of an instrument in its row and in its console: status dot, name, status, large value. */
@SuppressLint("SetTextI18n")
class InstrumentSummary(context: Context) {
    private val textPrimary = context.themeColor(android.R.attr.textColorPrimary)
    private val textSecondary = context.themeColor(android.R.attr.textColorSecondary)

    private val dot = GradientDrawable().apply {
        shape = GradientDrawable.OVAL
        setSize(context.dp(DOT), context.dp(DOT))
    }
    private val name = context.text(16f, textPrimary, MEDIUM).apply {
        setCompoundDrawablesRelativeWithIntrinsicBounds(dot, null, null, null)
        compoundDrawablePadding = context.dp(GAP)
    }
    // Status starts under the name, after the dot.
    private val status = context.text(13f, textSecondary).apply { setPaddingRelative(context.dp(DOT + GAP), 0, 0, 0) }
    private val value = context.text(26f, textPrimary).apply { fontFeatureSettings = "tnum" }

    val view = with(context) {
        horizontal().apply {
            setPadding(dp(PAD), dp(ROW_PAD), dp(PAD), dp(ROW_PAD))
            addView(vertical().apply {
                addView(name)
                addView(status)
            }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            addView(value)
        }
    }

    fun bind(i: SylvacInstrument) {
        dot.setColor(
            when (i.state) {
                SylvacInstrument.State.READY -> COLOR_OK
                SylvacInstrument.State.DISCONNECTED -> textSecondary
                else -> COLOR_BUSY
            },
        )
        name.text = i.name
        status.text = when (i.state) {
            SylvacInstrument.State.CONNECTING -> "Connecting…"
            SylvacInstrument.State.BONDING -> "Pairing…"
            SylvacInstrument.State.SETTING_UP -> "Setting up…"
            SylvacInstrument.State.RECONNECTING -> "Reconnecting…"
            SylvacInstrument.State.DISCONNECTED -> "Disconnected"
            SylvacInstrument.State.READY -> if (i.simulated) "Simulated" else listOfNotNull(
                i.buttonText?.let { "Button $it" },
                i.modeText,
                i.batteryPercent?.let { "Battery $it%" },
            ).joinToString(" · ").ifEmpty { "Ready" }
        }
        value.text = valueText(i.displayValue, i.displayUnit)
    }

    /** Value in large digits followed by a smaller, secondary unit. */
    private fun valueText(value: String, unit: String): CharSequence {
        if (unit.isEmpty()) return value
        return SpannableString("$value $unit").apply {
            setSpan(RelativeSizeSpan(0.5f), value.length, length, 0)
            setSpan(ForegroundColorSpan(textSecondary), value.length, length, 0)
        }
    }
}
