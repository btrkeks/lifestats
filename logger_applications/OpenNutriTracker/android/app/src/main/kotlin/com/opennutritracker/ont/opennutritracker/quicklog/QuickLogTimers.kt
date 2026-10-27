package com.opennutritracker.ont.opennutritracker.quicklog

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.ExistingWorkPolicy
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import androidx.work.workDataOf
import java.util.concurrent.TimeUnit

/**
 * Widget work that has to outlive the code that asked for it, on WorkManager
 * (which Glance already depends on). Each job is unique by name.
 */
object QuickLogTimers {
    private const val REFRESH = "quick_log_refresh"
    private const val UNDO = "quick_log_undo"
    private const val DAY = "quick_log_day"

    /** After the boundary, not on it: Dart must already see the new day. */
    private const val DAY_MARGIN_MS = 1_000L
    const val KIND = "kind"
    const val KIND_REFRESH = "refresh"
    const val KIND_UNDO_EXPIRY = "undoExpiry"
    const val OP_ID = "opId"

    /**
     * Clears "Added" at [atMs]. REPLACE: only the newest action is shown.
     * Firing late only delays the repaint; renders hide an expired action.
     */
    fun scheduleUndoExpiry(context: Context, opId: String, atMs: Long) {
        WorkManager.getInstance(context).enqueueUniqueWork(
            UNDO,
            ExistingWorkPolicy.REPLACE,
            OneTimeWorkRequestBuilder<QuickLogTimerWorker>()
                .setInitialDelay((atMs - System.currentTimeMillis()).coerceAtLeast(0), TimeUnit.MILLISECONDS)
                .setInputData(workDataOf(KIND to KIND_UNDO_EXPIRY, OP_ID to opId))
                .build(),
        )
    }

    /**
     * Asks Dart for a new snapshot once the logical day the current one
     * describes is over: new totals, and the order derived for the new day.
     * REPLACE: each publish carries the boundary that is due next.
     */
    fun scheduleDayRefresh(context: Context, atMs: Long) {
        WorkManager.getInstance(context).enqueueUniqueWork(
            DAY,
            ExistingWorkPolicy.REPLACE,
            OneTimeWorkRequestBuilder<QuickLogTimerWorker>()
                .setInitialDelay(
                    (atMs + DAY_MARGIN_MS - System.currentTimeMillis()).coerceAtLeast(0),
                    TimeUnit.MILLISECONDS,
                )
                .setInputData(workDataOf(KIND to KIND_REFRESH))
                .build(),
        )
    }

    /**
     * Asks Dart for a fresh snapshot, starting the engine if the app is not
     * running. KEEP: requests that arrive while one is queued share it.
     */
    fun requestRefresh(context: Context) {
        WorkManager.getInstance(context).enqueueUniqueWork(
            REFRESH,
            ExistingWorkPolicy.KEEP,
            OneTimeWorkRequestBuilder<QuickLogTimerWorker>()
                .setInputData(workDataOf(KIND to KIND_REFRESH))
                .build(),
        )
    }
}

class QuickLogTimerWorker(context: Context, params: WorkerParameters) :
    CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        when (inputData.getString(QuickLogTimers.KIND)) {
            QuickLogTimers.KIND_REFRESH -> QuickLogBridge.send(applicationContext, QuickLogCommand.Refresh)
            QuickLogTimers.KIND_UNDO_EXPIRY -> inputData.getString(QuickLogTimers.OP_ID)?.let {
                Log.i("QuickLogTimers", "undo window over for $it")
                QuickLogStore.clearAdded(applicationContext, it)
            }
            else -> Log.w("QuickLogTimers", "unknown job ${inputData.keyValueMap}")
        }
        return Result.success()
    }
}
