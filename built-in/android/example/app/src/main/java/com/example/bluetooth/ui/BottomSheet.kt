package com.example.bluetooth.ui

import android.annotation.SuppressLint
import android.app.Dialog
import android.content.Context
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.Window
import android.view.WindowInsets
import android.widget.LinearLayout

private const val HANDLE_W = 32 // the drag handle (dp)
private const val HANDLE_H = 4

/**
 * A framework Dialog anchored at the bottom (a "bottom sheet" without libraries): a handle and
 * [header], which can be dragged down to close it, then [body]. Back or a tap on the dimmed
 * screen behind close it too. [height] in px, or as tall as its content.
 */
fun Context.bottomSheet(header: View, body: View, height: Int = WRAP_CONTENT, onShow: () -> Unit = {}): Dialog {
    val dialog = Dialog(this)
    val radius = dp(RADIUS * 2).toFloat()
    dialog.requestWindowFeature(Window.FEATURE_NO_TITLE)
    dialog.setContentView(vertical().apply {
        isFocusableInTouchMode = true // keeps the focus, so a text field opens the keyboard only when tapped
        addView(dragToClose(dialog, vertical().apply {
            addView(View(context).apply {
                background = GradientDrawable().apply { setColor(overlay(0.3f)); cornerRadius = dp(HANDLE_H) / 2f }
            }, LinearLayout.LayoutParams(dp(HANDLE_W), dp(HANDLE_H)).apply { gravity = Gravity.CENTER_HORIZONTAL })
            addView(header)
        }))
        addView(body, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f))
        // Keep the content clear of the navigation bar and the keyboard.
        setOnApplyWindowInsetsListener { v, insets ->
            val bottom = insets.getInsets(WindowInsets.Type.navigationBars() or WindowInsets.Type.ime()).bottom
            v.setPadding(dp(PAD), dp(GAP), dp(PAD), dp(GAP) + bottom)
            insets
        }
    })
    dialog.setOnShowListener { dialog.window?.decorView?.translationY = 0f; onShow() }
    dialog.window?.apply {
        setBackgroundDrawable(GradientDrawable().apply {
            setColor(themeColor(android.R.attr.colorBackground))
            cornerRadii = floatArrayOf(radius, radius, radius, radius, 0f, 0f, 0f, 0f) // top corners only
        })
        setGravity(Gravity.BOTTOM)
        setLayout(MATCH_PARENT, height)
        setWindowAnimations(android.R.style.Animation_InputMethod) // framework slide-up from the bottom
    }
    return dialog
}

/** Makes [grip] drag [sheet] down: it follows the finger and closes when released past a third of its height. */
@SuppressLint("ClickableViewAccessibility") // back and a tap on the dimmed screen are the accessible ways to close
private fun dragToClose(sheet: Dialog, grip: View) = grip.apply {
    var downY = 0f
    setOnTouchListener { _, e ->
        val decor = sheet.window!!.decorView
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> downY = e.rawY // raw: the grip moves with the sheet
            MotionEvent.ACTION_MOVE -> decor.translationY = maxOf(0f, e.rawY - downY)
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL ->
                if (decor.translationY > decor.height / 3f) sheet.dismiss()
                else decor.animate().translationY(0f).setDuration(150)
        }
        true
    }
}
