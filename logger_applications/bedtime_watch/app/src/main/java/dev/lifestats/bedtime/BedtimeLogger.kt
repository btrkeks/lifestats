package dev.lifestats.bedtime

import android.content.Context
import androidx.wear.tiles.TileService
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.workDataOf
import java.time.LocalDateTime
import java.util.concurrent.TimeUnit

/** Saves the bedtime on the watch first, then uploads it whenever a network is available. */
fun logBedtime(context: Context, now: LocalDateTime = LocalDateTime.now()): Bedtime {
    val bedtime = Bedtime.at(now)
    BedtimeStore(context).save(bedtime, SyncStatus.PENDING)

    val upload = OneTimeWorkRequestBuilder<UploadBedtimeWorker>()
        .setInputData(
            workDataOf(
                UploadBedtimeWorker.KEY_DATE to bedtime.eveningDate.toString(),
                UploadBedtimeWorker.KEY_TIME to bedtime.time,
            )
        )
        .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
        .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 1, TimeUnit.MINUTES)
        .build()
    // Logging again the same evening replaces the earlier upload.
    WorkManager.getInstance(context)
        .enqueueUniqueWork("bedtime-${bedtime.eveningDate}", ExistingWorkPolicy.REPLACE, upload)

    refreshTile(context)
    return bedtime
}

fun refreshTile(context: Context) {
    TileService.getUpdater(context).requestUpdate(BedtimeTileService::class.java)
}
