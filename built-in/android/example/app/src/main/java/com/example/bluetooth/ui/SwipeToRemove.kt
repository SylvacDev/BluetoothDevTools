package com.example.bluetooth.ui

import android.annotation.SuppressLint
import android.graphics.Color
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.widget.FrameLayout

/**
 * Wraps [content] so it can be swiped to the left: a red "Remove" layer follows its right edge,
 * and releasing past a third of the width calls [onRemove]. Taps and vertical scrolling still
 * reach [content] and the page; only a mostly horizontal drag is taken over.
 *
 * Screen readers can't swipe: give [content] an accessibility action that does the same.
 */
class SwipeToRemove(private val content: View, private val onRemove: () -> Unit) : FrameLayout(content.context) {
    private val slop = ViewConfiguration.get(context).scaledTouchSlop
    private val reveal = context.text(15f, Color.WHITE, MEDIUM).apply {
        text = "Remove"
        gravity = Gravity.CENTER_VERTICAL
        setPadding(context.dp(PAD), 0, 0, 0)
        setBackgroundColor(context.themeColor(android.R.attr.colorError))
    }
    private var downX = 0f
    private var downY = 0f
    private var dragging = false

    init {
        addView(reveal, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
        addView(content)
        moveTo(0f)
    }

    private fun moveTo(dx: Float) {
        content.translationX = dx
        reveal.translationX = width + dx // left edge of the red layer = right edge of the row
    }

    override fun onLayout(changed: Boolean, l: Int, t: Int, r: Int, b: Int) {
        super.onLayout(changed, l, t, r, b)
        moveTo(content.translationX)
    }

    override fun onInterceptTouchEvent(e: MotionEvent): Boolean {
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> { downX = e.x; downY = e.y; dragging = false }
            MotionEvent.ACTION_MOVE -> {
                val dx = e.x - downX
                if (dx < -slop && -dx > Math.abs(e.y - downY)) {
                    dragging = true
                    parent.requestDisallowInterceptTouchEvent(true) // keep the ScrollView out
                }
            }
        }
        return dragging
    }

    @SuppressLint("ClickableViewAccessibility") // the content's "Remove" accessibility action is the accessible path
    override fun onTouchEvent(e: MotionEvent): Boolean {
        when (e.actionMasked) {
            MotionEvent.ACTION_MOVE -> moveTo(minOf(0f, e.x - downX))
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                dragging = false
                if (-content.translationX > width / 3f) {
                    content.animate().translationX(-width.toFloat()).setDuration(150)
                        .setUpdateListener { reveal.translationX = width + content.translationX }
                        .withEndAction { onRemove() }
                } else {
                    content.animate().translationX(0f).setDuration(150)
                        .setUpdateListener { reveal.translationX = width + content.translationX }
                }
            }
        }
        return true
    }
}
