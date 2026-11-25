package com.example.bluetooth

import android.annotation.SuppressLint
import android.content.Context
import android.text.TextUtils
import com.example.bluetooth.ui.PAD
import com.example.bluetooth.ui.ROW_PAD
import com.example.bluetooth.ui.dp
import com.example.bluetooth.ui.overlay
import com.example.bluetooth.ui.rounded
import com.example.bluetooth.ui.text
import com.example.bluetooth.ui.themeColor
import com.example.bluetooth.ui.themeResource
import com.example.bluetooth.ui.vertical

/** Small box containing information about an advertising instrument, with a click listener ([onClick]). */
@SuppressLint("SetTextI18n")
class NearbyTile(context: Context, onClick: () -> Unit) {
    private val textPrimary = context.themeColor(android.R.attr.textColorPrimary)
    private val textSecondary = context.themeColor(android.R.attr.textColorSecondary)

    private val name = context.text(13f, textSecondary).apply {
        isSingleLine = true
        ellipsize = TextUtils.TruncateAt.END
    }
    private val value = context.text(22f, textPrimary).apply {
        isSingleLine = true
        fontFeatureSettings = "tnum"
    }
    private val addressText = context.text(11.5f, textSecondary).apply {
        isSingleLine = true
        fontFeatureSettings = "tnum"
    }
    val root = with(context) {
        vertical().apply {
            setPadding(dp(PAD), dp(ROW_PAD), dp(PAD), dp(ROW_PAD))
            background = rounded(overlay(0.06f))
            clipToOutline = true
            foreground = getDrawable(themeResource(android.R.attr.selectableItemBackground))
            setOnClickListener { onClick() }
            addView(name)
            addView(value)
            addView(addressText)
        }
    }

    fun bind(a: Advert) {
        name.text = listOfNotNull(a.name, a.profile.takeIf { it != Sylvac.AdvProfile.SIMPLE }?.label).joinToString(" · ")
        value.text = a.value ?: "—" // live value broadcast in the advertising data
        addressText.text = a.device.address
    }
}
