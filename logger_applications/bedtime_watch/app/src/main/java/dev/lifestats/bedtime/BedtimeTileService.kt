package dev.lifestats.bedtime

import androidx.wear.protolayout.ActionBuilders
import androidx.wear.protolayout.ColorBuilders.argb
import androidx.wear.protolayout.ModifiersBuilders
import androidx.wear.protolayout.ResourceBuilders
import androidx.wear.protolayout.TimelineBuilders
import androidx.wear.protolayout.material.Colors
import androidx.wear.protolayout.material.CompactChip
import androidx.wear.protolayout.material.Text
import androidx.wear.protolayout.material.Typography
import androidx.wear.protolayout.material.layouts.PrimaryLayout
import androidx.wear.tiles.RequestBuilders
import androidx.wear.tiles.TileBuilders
import androidx.wear.tiles.TileService
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture

class BedtimeTileService : TileService() {

    override fun onTileRequest(requestParams: RequestBuilders.TileRequest): ListenableFuture<TileBuilders.Tile> {
        val device = requestParams.deviceConfiguration
        val last = BedtimeStore(this).last()

        val logNow = ModifiersBuilders.Clickable.Builder()
            .setId("log")
            .setOnClick(
                ActionBuilders.LaunchAction.Builder()
                    .setAndroidActivity(
                        ActionBuilders.AndroidActivity.Builder()
                            .setPackageName(packageName)
                            .setClassName(LogBedtimeActivity::class.java.name)
                            .addKeyToExtraMapping(
                                LogBedtimeActivity.EXTRA_LOG_NOW,
                                ActionBuilders.AndroidBooleanExtra.Builder().setValue(true).build(),
                            )
                            .build()
                    )
                    .build()
            )
            .build()

        val layout = PrimaryLayout.Builder(device)
            .setPrimaryLabelTextContent(
                Text.Builder(this, "Bedtime")
                    .setTypography(Typography.TYPOGRAPHY_CAPTION1)
                    .setColor(argb(Colors.DEFAULT.primary))
                    .build()
            )
            .setContent(
                Text.Builder(this, last?.bedtime?.time ?: "--:--")
                    .setTypography(Typography.TYPOGRAPHY_DISPLAY1)
                    .setColor(argb(Colors.DEFAULT.onSurface))
                    .build()
            )
            .setSecondaryLabelTextContent(
                Text.Builder(this, last?.let { statusLabel(it) } ?: "Nothing logged yet")
                    .setTypography(Typography.TYPOGRAPHY_CAPTION2)
                    .setColor(argb(Colors.DEFAULT.onSurface))
                    .build()
            )
            .setPrimaryChipContent(CompactChip.Builder(this, "Log now", logNow, device).build())
            .build()

        return Futures.immediateFuture(
            TileBuilders.Tile.Builder()
                .setResourcesVersion(RESOURCES_VERSION)
                .setTileTimeline(TimelineBuilders.Timeline.fromLayoutElement(layout))
                .build()
        )
    }

    override fun onTileResourcesRequest(
        requestParams: RequestBuilders.ResourcesRequest,
    ): ListenableFuture<ResourceBuilders.Resources> =
        Futures.immediateFuture(ResourceBuilders.Resources.Builder().setVersion(RESOURCES_VERSION).build())

    private companion object {
        const val RESOURCES_VERSION = "1"
    }
}

fun statusLabel(logged: LoggedBedtime): String {
    val date = "${logged.bedtime.eveningDate.monthValue}/${logged.bedtime.eveningDate.dayOfMonth}"
    return when (logged.status) {
        SyncStatus.PENDING -> "$date · waiting to send"
        SyncStatus.SENT -> "$date · sent ✓"
        SyncStatus.REJECTED -> "$date · rejected by server"
    }
}
