package com.opennutritracker.ont.opennutritracker.quicklog

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * Everything the widget renders, and the only code that writes it.
 *
 * Three facts with one owner each, merged when read rather than kept in one
 * blob that two writers rewrite:
 *  - snapshot: owned by Dart, written only by [publish].
 *  - recent action: owned by the widget's actions, [showAdded] / [clearAdded].
 *  - in flight: foods with a log waiting on Dart, kept in memory only.
 *
 * Writes use commit(), not apply(): the callback that wrote is about to
 * return, and the process may die right after it.
 */
object QuickLogStore {
    private const val TAG = "QuickLogStore"
    private const val PREFS = "quick_log"
    private const val KEY_SNAPSHOT = "snapshot"
    private const val KEY_RECENT_ACTION = "recent_action"

    const val UNDO_WINDOW_MS = 5_000L

    private var state: MutableStateFlow<WidgetModel>? = null
    private val inFlight = mutableSetOf<String>()

    /** The merged model. Emits on every write in this process. */
    fun model(context: Context): StateFlow<WidgetModel> = flow(context)

    fun current(context: Context): WidgetModel = flow(context).value

    /** Replaces the snapshot with what Dart just published. */
    fun publish(context: Context, json: String) {
        prefs(context).edit().putString(KEY_SNAPSHOT, json).commit()
        val snapshot = WidgetSnapshot.parse(json)
        Log.i(TAG, "publish ${describe(snapshot)}")
        update(context) { it.copy(snapshot = snapshot) }
    }

    /** False when [foodKey] already has a log waiting: a double tap. */
    fun beginLog(context: Context, foodKey: String): Boolean {
        val began = synchronized(inFlight) { inFlight.add(foodKey) }
        if (began) update(context) { it.copy(inFlight = inFlightNow()) }
        return began
    }

    fun endLog(context: Context, foodKey: String) {
        synchronized(inFlight) { inFlight.remove(foodKey) }
        update(context) { it.copy(inFlight = inFlightNow()) }
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
        prefs(context).edit().putString(KEY_RECENT_ACTION, action.toJson()).commit()
        update(context) { it.copy(recentAction = action) }
        return action
    }

    /** Clears only if [opId] is still the one shown, so a newer one survives. */
    fun clearAdded(context: Context, opId: String) {
        synchronized(this) {
            if (flow(context).value.recentAction?.opId != opId) return
            prefs(context).edit().remove(KEY_RECENT_ACTION).commit()
        }
        update(context) { if (it.recentAction?.opId == opId) it.copy(recentAction = null) else it }
    }

    private fun update(context: Context, change: (WidgetModel) -> WidgetModel) {
        val flow = flow(context)
        synchronized(this) { flow.value = change(flow.value) }
    }

    private fun flow(context: Context): MutableStateFlow<WidgetModel> =
        synchronized(this) {
            state ?: MutableStateFlow(read(context)).also { state = it }
        }

    private fun read(context: Context): WidgetModel {
        val prefs = prefs(context)
        return WidgetModel(
            snapshot = WidgetSnapshot.parse(prefs.getString(KEY_SNAPSHOT, null)),
            recentAction = RecentAction.parse(prefs.getString(KEY_RECENT_ACTION, null)),
            inFlight = inFlightNow(),
        )
    }

    private fun inFlightNow(): Set<String> = synchronized(inFlight) { inFlight.toSet() }

    private fun prefs(context: Context): SharedPreferences =
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    private fun describe(snapshot: WidgetSnapshot): String = when (snapshot) {
        is WidgetSnapshot.Ready ->
            "ready profile=${snapshot.profileId} rows=${snapshot.rows.size} " +
                "energy=${snapshot.status.energyValueText} protein=${snapshot.status.proteinText}"
        else -> snapshot.toString()
    }
}
