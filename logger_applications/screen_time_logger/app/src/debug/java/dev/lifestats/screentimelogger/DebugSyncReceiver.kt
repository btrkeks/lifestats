package dev.lifestats.screentimelogger

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

class DebugSyncReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != SYNC_ACTION) return

        try {
            saveOptionalConfiguration(context, intent)
            SyncScheduler.ensurePeriodicWork(context)
            SyncScheduler.enqueueImmediate(context)
            Log.i(ScreenTimeSyncWorker.TAG, "Debug sync queued")
        } catch (error: IllegalArgumentException) {
            Log.e(ScreenTimeSyncWorker.TAG, "Debug sync configuration was invalid", error)
        }
    }

    private fun saveOptionalConfiguration(context: Context, intent: Intent) {
        val hasBaseUrl = intent.hasExtra(BASE_URL_EXTRA)
        val hasStatisticId = intent.hasExtra(STATISTIC_ID_EXTRA)
        val hasGoalStatisticId = intent.hasExtra(GOAL_STATISTIC_ID_EXTRA)
        val hasGoalExemptPackages = intent.hasExtra(GOAL_EXEMPT_PACKAGES_EXTRA)
        if (!hasBaseUrl && !hasStatisticId && !hasGoalStatisticId && !hasGoalExemptPackages) return
        require(hasBaseUrl && hasStatisticId) {
            "base_url and statistic_id must be supplied together when configuring a sync"
        }

        val repository = SettingsRepository(context)
        val existingSettings = repository.loadOrNull()
        val settings = SettingsValidator.parse(
            baseUrlInput = intent.getStringExtra(BASE_URL_EXTRA).orEmpty(),
            totalStatisticIdInput = intent.getStringExtra(STATISTIC_ID_EXTRA).orEmpty(),
            goalStatisticIdInput = if (hasGoalStatisticId) {
                intent.getStringExtra(GOAL_STATISTIC_ID_EXTRA).orEmpty()
            } else {
                existingSettings?.goalStatisticId?.toString().orEmpty()
            },
            goalExemptPackages = if (hasGoalExemptPackages) {
                intent.getStringExtra(GOAL_EXEMPT_PACKAGES_EXTRA)
                    .orEmpty()
                    .split(',')
                    .map(String::trim)
                    .filter(String::isNotEmpty)
                    .toSet()
            } else {
                existingSettings?.goalExemptPackages.orEmpty()
            },
        )
        repository.save(settings)
    }

    companion object {
        const val SYNC_ACTION = "dev.lifestats.screentimelogger.SYNC_NOW"
        const val BASE_URL_EXTRA = "base_url"
        const val STATISTIC_ID_EXTRA = "statistic_id"
        const val GOAL_STATISTIC_ID_EXTRA = "goal_statistic_id"
        const val GOAL_EXEMPT_PACKAGES_EXTRA = "goal_exempt_packages"
    }
}
