package dev.lifestats.screentimelogger

import android.content.Context
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import java.util.concurrent.TimeUnit

object SyncScheduler {
    private val networkConstraint = Constraints.Builder()
        .setRequiredNetworkType(NetworkType.CONNECTED)
        .build()

    fun ensurePeriodicWork(context: Context) {
        val request = PeriodicWorkRequestBuilder<ScreenTimeSyncWorker>(
            PERIODIC_INTERVAL_HOURS,
            TimeUnit.HOURS,
            PERIODIC_FLEX_HOURS,
            TimeUnit.HOURS,
        )
            .setConstraints(networkConstraint)
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, BACKOFF_MINUTES, TimeUnit.MINUTES)
            .addTag(PERIODIC_WORK_NAME)
            .build()

        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            PERIODIC_WORK_NAME,
            ExistingPeriodicWorkPolicy.UPDATE,
            request,
        )
    }

    fun enqueueImmediate(context: Context) {
        val request = OneTimeWorkRequestBuilder<ScreenTimeSyncWorker>()
            .setConstraints(networkConstraint)
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, BACKOFF_MINUTES, TimeUnit.MINUTES)
            .addTag(IMMEDIATE_WORK_NAME)
            .build()

        WorkManager.getInstance(context).enqueueUniqueWork(
            IMMEDIATE_WORK_NAME,
            ExistingWorkPolicy.REPLACE,
            request,
        )
    }

    private const val PERIODIC_WORK_NAME = "screen-time-periodic-sync"
    private const val IMMEDIATE_WORK_NAME = "screen-time-immediate-sync"
    private const val PERIODIC_INTERVAL_HOURS = 4L
    private const val PERIODIC_FLEX_HOURS = 1L
    private const val BACKOFF_MINUTES = 15L
}
