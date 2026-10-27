package com.opennutritracker.ont.opennutritracker.quicklog.widget

import android.content.Context
import android.util.Log
import androidx.glance.GlanceId
import androidx.glance.action.ActionParameters
import androidx.glance.appwidget.action.ActionCallback
import com.opennutritracker.ont.opennutritracker.quicklog.QuickLogBridge
import com.opennutritracker.ont.opennutritracker.quicklog.QuickLogCommand
import com.opennutritracker.ont.opennutritracker.quicklog.QuickLogReply
import com.opennutritracker.ont.opennutritracker.quicklog.QuickLogStore
import com.opennutritracker.ont.opennutritracker.quicklog.QuickLogTimers
import com.opennutritracker.ont.opennutritracker.quicklog.WidgetSnapshot
import com.opennutritracker.ont.opennutritracker.quicklog.undoSucceeded
import java.util.UUID

private const val TAG = "QuickLogActions"

val FoodKeyParam = ActionParameters.Key<String>("foodKey")
val OpIdParam = ActionParameters.Key<String>("opId")
val ProfileIdParam = ActionParameters.Key<String>("profileId")

/**
 * Undo lands where the pill was, so the second half of a double tap would
 * otherwise undo the log the first half just made.
 */
private const val UNDO_ARM_MS = 600L

class LogPillAction : ActionCallback {
    override suspend fun onAction(context: Context, glanceId: GlanceId, parameters: ActionParameters) {
        val foodKey = parameters[FoodKeyParam] ?: return
        val profileId = parameters[ProfileIdParam] ?: return
        val model = QuickLogStore.current(context)
        val snapshot = model.snapshot as? WidgetSnapshot.Ready ?: return
        if (snapshot.profileId != profileId) return
        val row = model.row(foodKey) ?: return
        // A second callback may arrive after the first write has already acked.
        if (model.visibleAction(System.currentTimeMillis())?.foodKey == foodKey) return
        logFood(context, foodKey, snapshot.profileId, row.baseAmount, row.unit)
    }
}

/**
 * Logs [foodKey] through Dart and, once Dart acked, shows Added with Undo on
 * its row. The pill and the amount picker both log through here. Null when
 * a log of the same food is still waiting on Dart: a double tap.
 */
suspend fun logFood(
    context: Context,
    foodKey: String,
    profileId: String,
    baseAmount: Double,
    unit: String,
    actionKey: String? = null,
): QuickLogReply? {
    val key = actionKey ?: "pill:$profileId:$foodKey"
    val command = QuickLogStore.beginLog(context, key) {
        QuickLogCommand.Log(
            opId = UUID.randomUUID().toString(),
            foodKey = foodKey,
            profileId = profileId,
            baseAmount = baseAmount,
            unit = unit,
            tappedAtMs = System.currentTimeMillis(),
        )
    } ?: return null
    var outcome: QuickLogReply = QuickLogReply.Unavailable("interrupted")
    try {
        val reply = QuickLogBridge.send(context, command)
        outcome = reply
        if (reply is QuickLogReply.Ok && reply.addedText != null) {
            val action = QuickLogStore.showAdded(context, command.opId, foodKey, profileId, reply.addedText)
            QuickLogTimers.scheduleUndoExpiry(context, command.opId, action.expiresAtMs)
        }
        return reply
    } finally {
        QuickLogStore.endLog(context, key, outcome)
    }
}

class UndoAction : ActionCallback {
    override suspend fun onAction(context: Context, glanceId: GlanceId, parameters: ActionParameters) {
        val opId = parameters[OpIdParam] ?: return
        val profileId = parameters[ProfileIdParam] ?: return
        val shown = QuickLogStore.current(context).recentAction?.takeIf { it.opId == opId }
        if (shown != null && System.currentTimeMillis() - shown.shownAtMs < UNDO_ARM_MS) {
            Log.i(TAG, "undo ignored: $UNDO_ARM_MS ms not yet passed since Added")
            return
        }
        if (undoSucceeded(QuickLogBridge.send(context, QuickLogCommand.Undo(opId, profileId)))) {
            QuickLogStore.clearAdded(context, opId)
        }
    }
}
