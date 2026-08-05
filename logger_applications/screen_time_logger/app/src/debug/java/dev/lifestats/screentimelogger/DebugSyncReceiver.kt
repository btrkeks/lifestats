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
        if (!hasBaseUrl && !hasStatisticId) return
        require(hasBaseUrl && hasStatisticId) {
            "base_url and statistic_id must be supplied together"
        }

        val settings = SettingsValidator.parse(
            baseUrlInput = intent.getStringExtra(BASE_URL_EXTRA).orEmpty(),
            statisticIdInput = intent.getStringExtra(STATISTIC_ID_EXTRA).orEmpty(),
        )
        SettingsRepository(context).save(settings)
    }

    companion object {
        const val SYNC_ACTION = "dev.lifestats.screentimelogger.SYNC_NOW"
        const val BASE_URL_EXTRA = "base_url"
        const val STATISTIC_ID_EXTRA = "statistic_id"
    }
}
