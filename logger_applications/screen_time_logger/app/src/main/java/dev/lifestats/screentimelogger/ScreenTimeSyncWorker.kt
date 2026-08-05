package dev.lifestats.screentimelogger

import android.content.Context
import android.util.Log
import androidx.work.Worker
import androidx.work.WorkerParameters
import androidx.work.workDataOf
import java.io.IOException

class ScreenTimeSyncWorker(
    appContext: Context,
    workerParameters: WorkerParameters,
) : Worker(appContext, workerParameters) {
    override fun doWork(): Result {
        return try {
            val settings = SettingsRepository(applicationContext).load()
            val snapshot = ScreenTimeReader(
                usageEventSource = AndroidUsageEventSource(applicationContext),
                usageAccessChecker = AndroidUsageAccessChecker(applicationContext),
            ).readToday()

            LifestatsApiClient().logScreenTime(settings, snapshot)
            Log.i(TAG, "Logged ${snapshot.minutes} minutes for ${snapshot.localDate}")
            Result.success(
                workDataOf(
                    "date" to snapshot.localDate,
                    "minutes" to snapshot.minutes,
                ),
            )
        } catch (error: MissingConfigurationException) {
            Log.w(TAG, error.message.orEmpty())
            Result.failure()
        } catch (error: UsageAccessRequiredException) {
            Log.w(TAG, error.message.orEmpty())
            Result.failure()
        } catch (error: LifestatsHttpException) {
            Log.e(TAG, "Sync failed", error)
            if (error.isRetryable) Result.retry() else Result.failure()
        } catch (error: IOException) {
            Log.e(TAG, "Network error; sync will retry", error)
            Result.retry()
        } catch (error: ArithmeticException) {
            Log.e(TAG, "Usage totals overflowed", error)
            Result.failure()
        } catch (error: RuntimeException) {
            Log.e(TAG, "Unexpected sync error", error)
            Result.failure()
        }
    }

    companion object {
        const val TAG = "ScreenTimeLogger"
    }
}
