package com.example.bluetooth.ui

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.util.TypedValue
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

/*
 * Small builders for a UI made in code with plain framework views (no AndroidX, no XML layouts).
 * Nothing here knows about Sylvac or Bluetooth. Colours come from the theme, so the app follows
 * the phone's light / dark setting.
 */

// Spacing (dp): every gap, margin and padding of the UI uses one of these.
// Card edges and buttons sit on the page margin; all text sits PAD further in.
const val GAP = 8 // between neighbouring elements
const val PAD = 16 // page margins, padding inside rows, boxes and sheets
const val ROW_PAD = 12 // vertical padding of rows and boxes, horizontal padding of text buttons
const val RADIUS = 12 // corners of lists, boxes and fields

val MEDIUM: Typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
val BOLD: Typeface = Typeface.DEFAULT_BOLD
val MONO: Typeface = Typeface.MONOSPACE

fun Context.dp(v: Int) = (v * resources.displayMetrics.density).toInt()

fun Context.themeColor(attr: Int): Int {
    val a = obtainStyledAttributes(intArrayOf(attr))
    return try { a.getColor(0, Color.GRAY) } finally { a.recycle() }
}

fun Context.themeResource(attr: Int) = TypedValue().also { theme.resolveAttribute(attr, it, true) }.resourceId

/** The text colour at low opacity: a surface tint that works on light and dark themes. */
fun Context.overlay(alpha: Float): Int {
    val c = themeColor(android.R.attr.textColorPrimary)
    return Color.argb((alpha * 255).toInt(), Color.red(c), Color.green(c), Color.blue(c))
}

fun Context.rounded(color: Int) = GradientDrawable().apply {
    setColor(color)
    cornerRadius = dp(RADIUS).toFloat()
}

fun Context.vertical() = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }

fun Context.horizontal() = LinearLayout(this).apply { gravity = Gravity.CENTER_VERTICAL }

fun Context.text(sizeSp: Float, color: Int, face: Typeface = Typeface.DEFAULT) = TextView(this).apply {
    textSize = sizeSp
    setTextColor(color)
    typeface = face
}

/** Rounded group with thin dividers between rows (like Android settings lists). */
fun Context.listContainer() = vertical().apply {
    background = rounded(overlay(0.06f))
    clipToOutline = true
    showDividers = LinearLayout.SHOW_DIVIDER_MIDDLE
    dividerDrawable = GradientDrawable().apply {
        setColor(overlay(0.08f))
        setSize(0, dp(1))
    }
}

fun Context.filledButton(label: String, onClick: () -> Unit) = Button(this).apply {
    val accent = themeColor(android.R.attr.colorAccent)
    text = label
    isAllCaps = false
    backgroundTintList = ColorStateList.valueOf(accent)
    setTextColor(if (Color.luminance(accent) > 0.5f) Color.BLACK else Color.WHITE)
    setOnClickListener { onClick() }
}

/** Flat text button; pressed, it shows a rounded highlight inside its own bounds. */
fun Context.textButton(label: String, color: Int = themeColor(android.R.attr.colorAccent), onClick: () -> Unit) =
    Button(this, null, android.R.attr.borderlessButtonStyle).apply {
        background = RippleDrawable(ColorStateList.valueOf(overlay(0.16f)), null, rounded(Color.WHITE))
        text = label
        isAllCaps = false
        setTextColor(color)
        minWidth = 0
        minimumWidth = 0
        minHeight = 0
        minimumHeight = 0
        setPadding(dp(ROW_PAD), dp(GAP), dp(ROW_PAD), dp(GAP))
        setOnClickListener { onClick() }
    }
