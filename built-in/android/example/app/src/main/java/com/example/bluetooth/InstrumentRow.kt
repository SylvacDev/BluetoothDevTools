package com.example.bluetooth

import android.content.Context
import android.os.Bundle
import android.view.View
import android.view.accessibility.AccessibilityNodeInfo
import com.example.bluetooth.ui.SwipeToRemove
import com.example.bluetooth.ui.themeResource

private val A11Y_REMOVE = View.generateViewId() // id of the row's "Remove" accessibility action

/**
 * One connected instrument in the list: tap it for its console ([onOpen]), swipe it left to
 * remove it ([onRemove]).
 */
class InstrumentRow(
    context: Context,
    val instrument: SylvacInstrument,
    onOpen: () -> Unit,
    onRemove: () -> Unit,
) {
    private val summary = InstrumentSummary(context)

    val view = SwipeToRemove(summary.view.apply {
        setBackgroundResource(context.themeResource(android.R.attr.selectableItemBackground))
        setOnClickListener { onOpen() }
        // Screen readers can't swipe: offer the same Remove as an accessibility action
        // (TalkBack: actions menu on the focused row).
        accessibilityDelegate = object : View.AccessibilityDelegate() {
            override fun onInitializeAccessibilityNodeInfo(host: View, info: AccessibilityNodeInfo) {
                super.onInitializeAccessibilityNodeInfo(host, info)
                info.addAction(AccessibilityNodeInfo.AccessibilityAction(A11Y_REMOVE, "Remove"))
            }

            override fun performAccessibilityAction(host: View, action: Int, args: Bundle?): Boolean {
                if (action != A11Y_REMOVE) return super.performAccessibilityAction(host, action, args)
                onRemove()
                return true
            }
        }
    }, onRemove)

    init {
        bind()
    }

    fun bind() = summary.bind(instrument)
}
