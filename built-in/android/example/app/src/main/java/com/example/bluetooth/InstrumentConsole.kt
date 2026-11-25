package com.example.bluetooth

import android.annotation.SuppressLint
import android.content.Context
import android.content.res.ColorStateList
import android.graphics.drawable.RippleDrawable
import android.text.InputType
import android.text.SpannableString
import android.text.style.TypefaceSpan
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.LinearLayout
import com.example.bluetooth.ui.GAP
import com.example.bluetooth.ui.MONO
import com.example.bluetooth.ui.PAD
import com.example.bluetooth.ui.ROW_PAD
import com.example.bluetooth.ui.bottomSheet
import com.example.bluetooth.ui.dp
import com.example.bluetooth.ui.filledButton
import com.example.bluetooth.ui.horizontal
import com.example.bluetooth.ui.overlay
import com.example.bluetooth.ui.rounded
import com.example.bluetooth.ui.text
import com.example.bluetooth.ui.textButton
import com.example.bluetooth.ui.themeColor
import com.example.bluetooth.ui.vertical

private const val CHIP = 40 // height of a quick command (dp)
private const val REPLY_SP = 15f // monospace size that looks as large as the 16 sp instrument name

/**
 * The details of one instrument, in a bottom sheet, from most to least used: its summary,
 * quick commands, a free command, the last command with its reply, then address, profile and
 * Remove ([onRemove]).
 */
@SuppressLint("SetTextI18n")
class InstrumentConsole(private val context: Context, onRemove: (SylvacInstrument) -> Unit) {
    private val textPrimary = context.themeColor(android.R.attr.textColorPrimary)
    private val textSecondary = context.themeColor(android.R.attr.textColorSecondary)
    private val colorError = context.themeColor(android.R.attr.colorError)

    private var instrument: SylvacInstrument? = null
    private val summary = InstrumentSummary(context)
    private val address = context.text(13f, textSecondary).apply { setPadding(context.dp(PAD), 0, 0, 0) }
    private val input = EditText(context).apply {
        hint = "Command, e.g. ID?"
        textSize = 15f
        isSingleLine = true
        inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS
        imeOptions = EditorInfo.IME_ACTION_SEND
        background = context.rounded(context.overlay(0.08f))
        setPadding(context.dp(ROW_PAD), 0, context.dp(ROW_PAD), 0)
        setOnEditorActionListener { _, actionId, _ ->
            (actionId == EditorInfo.IME_ACTION_SEND).also { if (it) send(text.toString()) }
        }
    }
    // "Reply to <command>", and under it what the instrument answered. Hidden until the first
    // command, then it stays, so the sheet doesn't move at every command.
    private val asked = context.text(13f, textSecondary)
    private val answer = context.text(REPLY_SP, textPrimary, MONO)
    private val reply = context.vertical().apply {
        setPadding(context.dp(PAD), context.dp(ROW_PAD), context.dp(PAD), 0)
        addView(asked)
        addView(answer)
    }

    private val sheet = with(context) {
        bottomSheet(summary.view, vertical().apply {
            addView(horizontal().apply {
                addView(chip("Read value", "?"))
                addView(chip("ID", "ID?"))
                addView(chip("Version", "VER?"))
            })
            addView(horizontal().apply {
                setPadding(0, dp(GAP), 0, 0)
                addView(input, LinearLayout.LayoutParams(0, dp(44), 1f))
                addView(filledButton("Send") { send(input.text.toString()) }, LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT).apply {
                    marginStart = dp(GAP)
                })
            })
            addView(reply)
            // Footer, away from the commands: what identifies the instrument, and Remove.
            addView(View(context).apply { setBackgroundColor(overlay(0.08f)) }, LinearLayout.LayoutParams(MATCH_PARENT, dp(1)).apply {
                topMargin = dp(ROW_PAD)
            })
            addView(horizontal().apply {
                addView(address, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
                addView(textButton("Remove", colorError) { instrument?.let { onRemove(it) } }) // closes the sheet
            })
        })
    }

    /** A quick command: a filled chip with a plain name; the [command] it sends shows in the reply, and in the field. */
    private fun chip(label: String, command: String) = with(context) {
        textButton(label, textPrimary) {
            input.setText(command) // shows what the chip sends, ready to edit
            input.setSelection(command.length)
            send(command)
        }.apply {
            background = RippleDrawable(ColorStateList.valueOf(overlay(0.16f)), rounded(overlay(0.08f)).apply { cornerRadius = dp(CHIP) / 2f }, null)
            setPadding(dp(PAD), 0, dp(PAD), 0)
            layoutParams = LinearLayout.LayoutParams(WRAP_CONTENT, dp(CHIP)).apply { marginEnd = dp(GAP) }
        }
    }

    fun show(i: SylvacInstrument) {
        instrument = i
        input.text.clear()
        bind(i)
        sheet.show()
    }

    /** Refreshes the sheet if it shows [i]. */
    fun bind(i: SylvacInstrument) {
        if (i != instrument) return
        summary.bind(i)
        address.text = "${i.address} · ${i.profile.label}"
        val command = i.lastCommand
        reply.visibility = if (command == null) View.GONE else View.VISIBLE
        if (command == null) return
        asked.text = SpannableString("Reply to $command").apply { setSpan(TypefaceSpan(MONO), 9, length, 0) }
        answer.text = i.lastReply.joinToString("\n").ifEmpty { "…" }
    }

    /** Closes the sheet if it shows [i]: the instrument was removed. */
    fun close(i: SylvacInstrument) {
        if (i == instrument) sheet.dismiss()
    }

    private fun send(command: String) {
        // A typed command stays in the field, to resend or edit.
        if (command.isNotBlank()) instrument?.sendCommand(command.trim())
    }
}
