package dev.lifestats.bedtime

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.time.LocalDate

class UploadBedtimeWorker(context: Context, params: WorkerParameters) :
    CoroutineWorker(context, params) {

    override suspend fun doWork(): Result {
        val bedtime = Bedtime(
            LocalDate.parse(inputData.getString(KEY_DATE)!!),
            inputData.getString(KEY_TIME)!!,
        )
        val store = BedtimeStore(applicationContext)

        for (baseUrl in BuildConfig.LIFESTATS_URLS.split(',').map { it.trim() }.filter { it.isNotEmpty() }) {
            val code = post(logUrl(baseUrl, BuildConfig.BEDTIME_STATISTIC_ID, bedtime.eveningDate), logRequestBody(bedtime.time))
            Log.i(TAG, "POST $bedtime to $baseUrl -> ${code ?: "unreachable"}")
            when {
                code == null -> continue
                code in 200..299 -> {
                    store.markIfLatest(bedtime, SyncStatus.SENT)
                    refreshTile(applicationContext)
                    return Result.success()
                }
                code in 400..499 && code != 408 && code != 429 -> {
                    store.markIfLatest(bedtime, SyncStatus.REJECTED)
                    refreshTile(applicationContext)
                    return Result.failure()
                }
            }
        }
        return Result.retry()
    }

    /** Returns the HTTP status, or null when the server could not be reached. */
    private suspend fun post(url: String, body: String): Int? = withContext(Dispatchers.IO) {
        try {
            val connection = URL(url).openConnection() as HttpURLConnection
            try {
                connection.requestMethod = "POST"
                connection.connectTimeout = 5_000
                connection.readTimeout = 10_000
                connection.doOutput = true
                connection.setRequestProperty("Content-Type", "application/json")
                connection.outputStream.use { it.write(body.toByteArray()) }
                connection.responseCode
            } finally {
                connection.disconnect()
            }
        } catch (e: IOException) {
            Log.w(TAG, "POST $url failed: $e")
            null
        }
    }

    companion object {
        const val KEY_DATE = "date"
        const val KEY_TIME = "time"
        private const val TAG = "BedtimeUpload"
    }
}
