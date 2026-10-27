package com.opennutritracker.ont.opennutritracker.quicklog

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.ExistingWorkPolicy
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import androidx.work.workDataOf

/**
 * Widget work that has to outlive the code that asked for it, on WorkManager
 * (which Glance already depends on). Each job is unique by name.
 */
object QuickLogTimers {
    private const val REFRESH = "quick_log_refresh"
    const val KIND = "kind"
    const val KIND_REFRESH = "refresh"

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
            else -> Log.w("QuickLogTimers", "unknown job ${inputData.keyValueMap}")
        }
        return Result.success()
    }
}
