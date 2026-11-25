package com.example.bluetooth.ui

import android.content.Context
import android.widget.ScrollView

/**
 * A ScrollView as tall as its content, that leaves [reserve] px of the remaining height to the
 * views below it (a weighted sibling's minimum height is ignored by LinearLayout).
 */
class CappedScrollView(context: Context, private val reserve: Int) : ScrollView(context) {
    override fun onMeasure(widthSpec: Int, heightSpec: Int) = super.onMeasure(
        widthSpec,
        MeasureSpec.makeMeasureSpec(maxOf(0, MeasureSpec.getSize(heightSpec) - reserve), MeasureSpec.AT_MOST),
    )
}
