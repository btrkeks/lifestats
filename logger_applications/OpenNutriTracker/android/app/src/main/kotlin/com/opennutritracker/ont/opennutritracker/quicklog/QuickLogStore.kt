package com.opennutritracker.ont.opennutritracker.quicklog

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import com.opennutritracker.ont.opennutritracker.quicklog.widget.QuickLogWidget
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.StateFlow

/**
 * Everything the widget renders, and the only code that writes it.
 *
 * Facts with one owner each, merged when read rather than kept in one
 * blob that two writers rewrite:
 *  - snapshot: owned by Dart, written only by [publish].
 *  - recent action: owned by the widget's actions, [showAdded] / [clearAdded].
 *  - pending logs: original commands retained until a definitive reply.
 *  - in flight: foods with a log waiting on Dart, kept in memory only.
 *
 * Writes use commit(), not apply(): the callback that wrote is about to
 * return, and the process may die right after it. Every write repaints the
 * widget, so no caller can forget to.
 */
object QuickLogStore {
    private const val TAG = "QuickLogStore"
    private const val PREFS = "quick_log"
    private const val KEY_SNAPSHOT = "snapshot"
    private const val KEY_RECENT_ACTION = "recent_action"

    const val UNDO_WINDOW_MS = 5_000L

    private var state: QuickLogState? = null
    private var pending: PendingLogs? = null
    private const val KEY_PENDING = "pending_logs"
    private val repaints = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    /** The day boundary last handed to WorkManager in this process. */
    private var scheduledDayEnd: Long? = null

    /** The merged model. Emits on every write in this process. */
    fun model(context: Context): StateFlow<WidgetModel> = flow(context)

    fun current(context: Context): WidgetModel = flow(context).value

    /** Replaces the snapshot with what Dart just published. */
    fun publish(context: Context, json: String) {
        prefs(context).edit().putString(KEY_SNAPSHOT, json).commit()
        val snapshot = WidgetSnapshot.parse(json)
        Log.i(TAG, "publish ${describe(snapshot)}")
        if (snapshot is WidgetSnapshot.Ready) {
            QuickLogShortcut.update(context, snapshot.labels.scanProduct)
            if (snapshot.validUntilMs != scheduledDayEnd) {
                QuickLogTimers.scheduleDayRefresh(context, snapshot.validUntilMs)
                scheduledDayEnd = snapshot.validUntilMs
            }
        }
        update(context) { it.copy(snapshot = snapshot) }
    }

    fun beginLog(context: Context, key: String, create: () -> QuickLogCommand.Log): QuickLogCommand.Log? {
        val command = pending(context).begin(key, create)
        if (command != null) update(context) { it.copy(inFlight = pending(context).inFlight()) }
        return command
    }

    fun endLog(context: Context, key: String, reply: QuickLogReply) {
        pending(context).finish(key, reply)
        update(context) { it.copy(inFlight = pending(context).inFlight()) }
    }

    private fun pending(context: Context): PendingLogs = synchronized(this) {
        pending ?: PendingLogs(PendingLogs.decode(prefs(context).getString(KEY_PENDING, null))) {
            prefs(context).edit().putString(KEY_PENDING, PendingLogs.encode(it)).commit()
        }.also { pending = it }
    }

    /**
     * Shows "Added" with Undo on [foodKey]'s row for [UNDO_WINDOW_MS].
     * Replaces any earlier one, whose Undo is forfeited.
     */
    fun showAdded(
        context: Context,
        opId: String,
        foodKey: String,
        profileId: String,
        addedText: String,
        nowMs: Long = System.currentTimeMillis(),
    ): RecentAction {
        val action = RecentAction(opId, foodKey, profileId, addedText, nowMs, nowMs + UNDO_WINDOW_MS)
        state(context).show(action)
        Log.i(TAG, "added opId=$opId food=$foodKey expiresAtMs=${action.expiresAtMs}")
        repaint(context)
        return action
    }

    /** Clears only if [opId] is still the one shown, so a newer one survives. */
    fun clearAdded(context: Context, opId: String) {
        state(context).clear(opId)
        repaint(context)
    }

    private fun update(context: Context, change: (WidgetModel) -> WidgetModel) {
        state(context).update(change)
        repaint(context)
    }

    private fun repaint(context: Context) {
        val app = context.applicationContext
        repaints.launch { QuickLogWidget.refreshAll(app) }
    }

    private fun flow(context: Context): MutableStateFlow<WidgetModel> = state(context).flow

    private fun state(context: Context): QuickLogState = synchronized(this) {
        state ?: QuickLogState(read(context)) { action ->
            val editor = prefs(context).edit()
            if (action == null) editor.remove(KEY_RECENT_ACTION)
            else editor.putString(KEY_RECENT_ACTION, action.toJson())
            editor.commit()
        }.also { state = it }
    }

    private fun read(context: Context): WidgetModel {
        val prefs = prefs(context)
        return WidgetModel(
            snapshot = WidgetSnapshot.parse(prefs.getString(KEY_SNAPSHOT, null)),
            recentAction = RecentAction.parse(prefs.getString(KEY_RECENT_ACTION, null)),
            inFlight = pending(context).inFlight(),
        )
    }

    private fun prefs(context: Context): SharedPreferences =
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    private fun describe(snapshot: WidgetSnapshot): String = when (snapshot) {
        is WidgetSnapshot.Ready ->
            "ready profile=${snapshot.profileId} rows=${snapshot.rows.size} " +
                "energy=${snapshot.status.energyValueText} protein=${snapshot.status.proteinText}"
        else -> snapshot.toString()
    }
}
