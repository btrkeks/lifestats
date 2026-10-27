package com.opennutritracker.ont.opennutritracker.quicklog.widget

import android.graphics.Bitmap
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.min
import androidx.compose.ui.unit.sp
import androidx.glance.ColorFilter
import androidx.glance.GlanceModifier
import androidx.glance.Image
import androidx.glance.ImageProvider
import androidx.glance.LocalContext
import androidx.glance.action.Action
import androidx.glance.action.actionParametersOf
import androidx.glance.action.actionStartActivity
import androidx.glance.action.clickable
import androidx.glance.appwidget.action.actionRunCallback
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.appWidgetBackground
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.lazy.LazyColumn
import androidx.glance.appwidget.lazy.items
import androidx.glance.background
import androidx.glance.color.ColorProvider
import androidx.glance.layout.Alignment
import androidx.glance.layout.Box
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxHeight
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.layout.padding
import androidx.glance.layout.size
import androidx.glance.layout.width
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import androidx.glance.unit.ColorProvider
import com.opennutritracker.ont.opennutritracker.LaunchInbox
import com.opennutritracker.ont.opennutritracker.MainActivity
import com.opennutritracker.ont.opennutritracker.R
import com.opennutritracker.ont.opennutritracker.quicklog.QuickLogPickerActivity
import com.opennutritracker.ont.opennutritracker.quicklog.RecentAction
import com.opennutritracker.ont.opennutritracker.quicklog.Row as FoodRowData
import com.opennutritracker.ont.opennutritracker.quicklog.Status
import com.opennutritracker.ont.opennutritracker.quicklog.WidgetModel
import com.opennutritracker.ont.opennutritracker.quicklog.WidgetSnapshot

/**
 * The app's colour scheme (lib/core/styles/color_schemes.dart) as the mock
 * uses it. Day/night pairs resolve on the launcher side, so the widget
 * follows system dark mode even when the app is not running.
 */
private object Palette {
    private fun pair(day: Long, night: Long) = ColorProvider(day = Color(day), night = Color(night))

    val background = pair(0xFFFCFDF7, 0xFF1A1C19)
    val onBackground = pair(0xFF1A1C19, 0xFFE2E3DD)
    val onBackgroundVariant = pair(0xFF424940, 0xFFC1C9BE)
    val accent = pair(0xFF006E2B, 0xFF33E36A)
    val onAccent = pair(0xFFFFFFFF, 0xFF003913)
    val row = pair(0xFFD4E8D1, 0xFF3A4B3A)
    val onRow = pair(0xFF0F1F11, 0xFFD4E8D1)
    val onRowMuted = pair(0xC70F1F11, 0xC7D4E8D1)
    val scan = pair(0xFFBDEAF3, 0xFF1F4D54)
    val onScan = pair(0xFF001F24, 0xFFBDEAF3)
    val track = pair(0xFFDDE5D9, 0xFF424940)
    val protein = ColorProvider(Color(0xFF2A8C82))
    val proteinPending = ColorProvider(Color(0x992A8C82))
    val onProtein = ColorProvider(Color.White)
}

private val Padding = 14.dp
private val ColumnGap = 12.dp
private val RowGap = 6.dp
private val StatusWidth = 124.dp
private val ButtonHeight = 48.dp

/** Under the mock's 70 dp: a real 4×2 cell is narrower than the mock's, and at 70 dp the subtitle ellipsized. */
private val PillWidth = 62.dp

/** Below this the rings and the scan button no longer fit beside the list: the 4×1 layout. */
private val CompactBelow = 150.dp

/** The 4×1 has no room to scroll, so it shows the top of the list. */
private const val CompactChips = 3

/** Pure presentation of [model] at [nowMs]; reads no storage and no clock. */
@Composable
fun QuickLogContent(model: WidgetModel, nowMs: Long, size: DpSize) {
    val context = LocalContext.current
    val openApp = actionStartActivity<MainActivity>()
    val scan = actionStartActivity(LaunchInbox.intentFor(context, LaunchInbox.Request.ScanProduct))
    val search = actionStartActivity(LaunchInbox.intentFor(context, LaunchInbox.Request.SearchFood))
    val surface = GlanceModifier
        .fillMaxSize()
        .appWidgetBackground()
        .background(Palette.background)
        .cornerRadius(android.R.dimen.system_app_widget_background_radius)
    val frame = surface.padding(Padding)
    when (val snapshot = model.snapshot) {
        is WidgetSnapshot.Ready -> if (size.height < CompactBelow) {
            val verticalPadding = ((size.height - 22.dp - 6.dp - ButtonHeight) / 2f).coerceIn(0.dp, 10.dp)
            Compact(snapshot, model, nowMs, scan, search, surface.padding(start = 12.dp, end = 12.dp, top = verticalPadding, bottom = verticalPadding))
        } else Row(frame) {
            val ringSize = min(StatusWidth, size.height - Padding * 2 - ButtonHeight - RowGap)
            if (snapshot.rows.isEmpty()) {
                Column(
                    GlanceModifier.width(StatusWidth).fillMaxHeight(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) { Rings(snapshot.status, min(StatusWidth, size.height - Padding * 2)) }
                Spacer(GlanceModifier.width(ColumnGap))
                Empty(snapshot, scan, search)
            } else {
                Column(
                    GlanceModifier.width(StatusWidth).fillMaxHeight(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Rings(snapshot.status, ringSize)
                    Spacer(GlanceModifier.defaultWeight())
                    Pill(
                        icon = R.drawable.ql_ic_barcode,
                        label = snapshot.labels.scanProduct,
                        background = Palette.scan,
                        content = Palette.onScan,
                        fontSize = 13.sp,
                        action = scan,
                        modifier = GlanceModifier.width(StatusWidth),
                    )
                }
                Spacer(GlanceModifier.width(ColumnGap))
                LazyColumn(GlanceModifier.defaultWeight().fillMaxHeight()) {
                    val added = model.visibleAction(nowMs)
                    items(snapshot.rows, itemId = { it.foodKey.hashCode().toLong() }) { row ->
                        Box(GlanceModifier.fillMaxWidth().padding(bottom = RowGap)) {
                            if (added?.foodKey == row.foodKey) {
                                AddedRow(row, added, snapshot.labels.undo)
                            } else {
                                FoodRow(row, snapshot.profileId, logging = row.foodKey in model.inFlight)
                            }
                        }
                    }
                }
            }
        }
        // Not set up, unreadable or never published: zero rings and a way in.
        else -> Row(frame.clickable(openApp), verticalAlignment = Alignment.CenterVertically) {
            Box(GlanceModifier.width(StatusWidth), contentAlignment = Alignment.Center) {
                RingImages(0f, 0f, min(StatusWidth, size.height - Padding * 2))
            }
            Spacer(GlanceModifier.width(ColumnGap))
            Text(
                text = (snapshot as? WidgetSnapshot.NeedsSetup)?.title
                    ?: context.getString(R.string.app_name),
                style = TextStyle(color = Palette.onBackground, fontSize = 15.sp, fontWeight = FontWeight.Medium),
                maxLines = 2,
            )
        }
    }
}

/** s4: one status line, the top foods as chips, an icon-only scan tile. */
@Composable
private fun Compact(
    snapshot: WidgetSnapshot.Ready,
    model: WidgetModel,
    nowMs: Long,
    scan: Action,
    search: Action,
    modifier: GlanceModifier,
) {
    val status = snapshot.status
    Column(modifier) {
        Row(GlanceModifier.height(22.dp).padding(horizontal = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            MiniRing(status.energyFraction, Palette.accent)
            Spacer(GlanceModifier.width(6.dp))
            Text(
                status.energyValueText,
                style = TextStyle(color = Palette.onBackground, fontSize = 17.sp, fontWeight = FontWeight.Bold),
                maxLines = 1,
            )
            Spacer(GlanceModifier.width(4.dp))
            Text(
                status.energyLabelText,
                style = TextStyle(color = Palette.onBackgroundVariant, fontSize = 12.sp),
                maxLines = 1,
            )
            Spacer(GlanceModifier.width(18.dp))
            MiniRing(status.proteinFraction, Palette.protein)
            Spacer(GlanceModifier.width(6.dp))
            ProteinProgress(status.proteinCompactText, large = 17.sp)
        }
        Spacer(GlanceModifier.height(6.dp))
        Row(GlanceModifier.fillMaxWidth()) {
            val added = model.visibleAction(nowMs)
            if (snapshot.rows.isEmpty()) {
                Pill(
                    icon = R.drawable.ql_ic_search,
                    label = snapshot.labels.searchFood,
                    background = Palette.row,
                    content = Palette.onRow,
                    fontSize = 14.sp,
                    action = search,
                    modifier = GlanceModifier.defaultWeight(),
                )
            }
            snapshot.rows.take(CompactChips).forEachIndexed { i, row ->
                if (i > 0) Spacer(GlanceModifier.width(RowGap))
                Box(GlanceModifier.defaultWeight()) {
                    if (added?.foodKey == row.foodKey) {
                        AddedChip(row, added, snapshot.labels.undo)
                    } else {
                        FoodChip(row, snapshot.profileId, logging = row.foodKey in model.inFlight)
                    }
                }
            }
            Spacer(GlanceModifier.width(RowGap))
            Box(
                GlanceModifier
                    .size(ButtonHeight)
                    .background(Palette.scan)
                    .cornerRadius(16.dp)
                    .clickable(scan),
                contentAlignment = Alignment.Center,
            ) {
                Image(
                    provider = ImageProvider(R.drawable.ql_ic_barcode),
                    contentDescription = snapshot.labels.scanProduct,
                    modifier = GlanceModifier.size(24.dp),
                    colorFilter = ColorFilter.tint(Palette.onScan),
                )
            }
        }
    }
}

/** The whole chip is the pill: the 4×1 has room for one tap target per food. */
@Composable
private fun FoodChip(row: FoodRowData, profileId: String, logging: Boolean) {
    Column(
        GlanceModifier
            .fillMaxWidth()
            .height(ButtonHeight)
            .background(Palette.row)
            .cornerRadius(16.dp)
            .padding(horizontal = 4.dp)
            .clickable(actionRunCallback<LogPillAction>(actionParametersOf(FoodKeyParam to row.foodKey, ProfileIdParam to profileId))),
        verticalAlignment = Alignment.CenterVertically,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            row.title,
            style = TextStyle(color = Palette.onRow, fontSize = 13.sp, fontWeight = FontWeight.Medium),
            maxLines = 1,
        )
        Spacer(GlanceModifier.height(3.dp))
        ChipBadge(row.chipText, if (logging) Palette.proteinPending else Palette.protein, Palette.onProtein)
    }
}

@Composable
private fun AddedChip(row: FoodRowData, action: RecentAction, undoLabel: String) {
    Column(
        GlanceModifier
            .fillMaxWidth()
            .height(ButtonHeight)
            .background(Palette.accent)
            .cornerRadius(16.dp)
            .padding(horizontal = 4.dp)
            .clickable(
                actionRunCallback<UndoAction>(
                    actionParametersOf(OpIdParam to action.opId, ProfileIdParam to action.profileId),
                ),
            ),
        verticalAlignment = Alignment.CenterVertically,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            row.title,
            style = TextStyle(color = Palette.onAccent, fontSize = 13.sp, fontWeight = FontWeight.Medium),
            maxLines = 1,
        )
        Spacer(GlanceModifier.height(3.dp))
        ChipBadge(undoLabel, Palette.onAccent, Palette.accent)
    }
}

@Composable
private fun ChipBadge(text: String, background: ColorProvider, content: ColorProvider) {
    Box(
        GlanceModifier
            .height(16.dp)
            .background(background)
            .cornerRadius(8.dp)
            .padding(horizontal = 6.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text,
            style = TextStyle(color = content, fontSize = 12.sp, fontWeight = FontWeight.Bold),
            maxLines = 1,
        )
    }
}

@Composable
private fun Rings(status: Status, size: Dp) {
    Box(GlanceModifier.size(size), contentAlignment = Alignment.Center) {
        RingImages(status.energyFraction, status.proteinFraction, size)
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(
                status.energyValueText,
                style = TextStyle(color = Palette.onBackground, fontSize = 22.sp, fontWeight = FontWeight.Bold),
                maxLines = 1,
            )
            Text(
                status.energyLabelText,
                style = TextStyle(color = Palette.onBackgroundVariant, fontSize = 12.sp),
                maxLines = 1,
            )
            Spacer(GlanceModifier.height(2.dp))
            ProteinProgress(status.proteinText)
        }
    }
}

/** "30/150 g" with the eaten part large, as in the mock. */
@Composable
private fun ProteinProgress(text: String, large: TextUnit = 16.sp) {
    val split = text.indexOf('/')
    val style = TextStyle(color = Palette.protein, fontSize = 12.sp, fontWeight = FontWeight.Medium)
    if (split <= 0) {
        Text(text, style = style, maxLines = 1)
        return
    }
    Row(verticalAlignment = Alignment.Bottom) {
        Text(
            text.substring(0, split),
            style = style.copy(fontSize = large, fontWeight = FontWeight.Bold),
            maxLines = 1,
        )
        Text(text.substring(split), style = style, maxLines = 1)
    }
}

@Composable
private fun RingImages(energyFraction: Float, proteinFraction: Float, size: Dp) {
    val context = LocalContext.current
    val px = remember(size) { (size.value * context.resources.displayMetrics.density).toInt().coerceAtLeast(1) }
    val rings = RingBitmap.CONCENTRIC
    val tracks = remember(px) { RingBitmap.tracks(px, rings) }
    val energy = remember(px, energyFraction) { RingBitmap.arc(px, rings, rings.radii[0], energyFraction) }
    val protein = remember(px, proteinFraction) { RingBitmap.arc(px, rings, rings.radii[1], proteinFraction) }
    Box(GlanceModifier.size(size)) {
        RingLayer(tracks, Palette.track, size)
        RingLayer(energy, Palette.accent, size)
        RingLayer(protein, Palette.protein, size)
    }
}

@Composable
private fun MiniRing(fraction: Float, color: ColorProvider) {
    val context = LocalContext.current
    val size = 20.dp
    val px = (size.value * context.resources.displayMetrics.density).toInt()
    val ring = RingBitmap.MINI
    val tracks = remember(px) { RingBitmap.tracks(px, ring) }
    val arc = remember(px, fraction) { RingBitmap.arc(px, ring, ring.radii[0], fraction) }
    Box(GlanceModifier.size(size)) {
        RingLayer(tracks, Palette.track, size)
        RingLayer(arc, color, size)
    }
}

@Composable
private fun RingLayer(bitmap: Bitmap, color: ColorProvider, size: Dp) {
    Image(
        provider = ImageProvider(bitmap),
        contentDescription = null,
        modifier = GlanceModifier.size(size),
        colorFilter = ColorFilter.tint(color),
    )
}

@Composable
private fun FoodRow(row: FoodRowData, profileId: String, logging: Boolean) {
    val context = LocalContext.current
    Row(
        GlanceModifier
            .fillMaxWidth()
            .height(ButtonHeight)
            .background(Palette.row)
            .cornerRadius(16.dp)
            .padding(start = 14.dp, end = 8.dp)
            .clickable(actionStartActivity(QuickLogPickerActivity.intentFor(context, row.foodKey, profileId))),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(GlanceModifier.defaultWeight()) {
            Text(
                row.title,
                style = TextStyle(color = Palette.onRow, fontSize = 14.sp, fontWeight = FontWeight.Medium),
                maxLines = 1,
            )
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    row.amountText,
                    style = TextStyle(color = Palette.onRow, fontSize = 12.sp, fontWeight = FontWeight.Medium),
                    maxLines = 1,
                )
                Image(
                    provider = ImageProvider(R.drawable.ql_ic_dropdown),
                    contentDescription = null,
                    modifier = GlanceModifier.width(10.dp).height(16.dp),
                    colorFilter = ColorFilter.tint(Palette.onRow),
                )
                Spacer(GlanceModifier.width(2.dp))
                Text(
                    "· ${row.energyText}",
                    style = TextStyle(color = Palette.onRowMuted, fontSize = 12.sp),
                    maxLines = 1,
                )
            }
        }
        Spacer(GlanceModifier.width(8.dp))
        Box(
            GlanceModifier
                .width(PillWidth)
                .height(32.dp)
                .background(if (logging) Palette.proteinPending else Palette.protein)
                .cornerRadius(16.dp)
                .clickable(actionRunCallback<LogPillAction>(actionParametersOf(FoodKeyParam to row.foodKey, ProfileIdParam to profileId))),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                row.pillText,
                style = TextStyle(color = Palette.onProtein, fontSize = 13.sp, fontWeight = FontWeight.Bold),
                maxLines = 1,
            )
        }
    }
}

@Composable
private fun AddedRow(row: FoodRowData, action: RecentAction, undoLabel: String) {
    Row(
        GlanceModifier
            .fillMaxWidth()
            .height(ButtonHeight)
            .background(Palette.accent)
            .cornerRadius(16.dp)
            .padding(start = 14.dp, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(GlanceModifier.defaultWeight()) {
            Text(
                row.title,
                style = TextStyle(color = Palette.onAccent, fontSize = 14.sp, fontWeight = FontWeight.Medium),
                maxLines = 1,
            )
            Row(verticalAlignment = Alignment.CenterVertically) {
                Image(
                    provider = ImageProvider(R.drawable.ql_ic_check),
                    contentDescription = null,
                    modifier = GlanceModifier.size(16.dp),
                    colorFilter = ColorFilter.tint(Palette.onAccent),
                )
                Spacer(GlanceModifier.width(2.dp))
                Text(
                    action.addedText,
                    style = TextStyle(color = Palette.onAccent, fontSize = 12.sp, fontWeight = FontWeight.Medium),
                    maxLines = 1,
                )
            }
        }
        Spacer(GlanceModifier.width(8.dp))
        Row(
            GlanceModifier
                .height(34.dp)
                .background(Palette.onAccent)
                .cornerRadius(17.dp)
                .padding(start = 8.dp, end = 11.dp)
                .clickable(
                    actionRunCallback<UndoAction>(
                        actionParametersOf(OpIdParam to action.opId, ProfileIdParam to action.profileId),
                    ),
                ),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Image(
                provider = ImageProvider(R.drawable.ql_ic_undo),
                contentDescription = null,
                modifier = GlanceModifier.size(18.dp),
                colorFilter = ColorFilter.tint(Palette.accent),
            )
            Spacer(GlanceModifier.width(4.dp))
            Text(
                undoLabel,
                style = TextStyle(color = Palette.accent, fontSize = 13.sp, fontWeight = FontWeight.Bold),
                maxLines = 1,
            )
        }
    }
}

@Composable
private fun Empty(snapshot: WidgetSnapshot.Ready, scan: Action, search: Action) {
    Column(GlanceModifier.fillMaxHeight()) {
        Column(GlanceModifier.padding(start = 6.dp, end = 4.dp, top = 2.dp)) {
            Text(
                snapshot.labels.emptyTitle,
                style = TextStyle(color = Palette.onBackground, fontSize = 15.sp, fontWeight = FontWeight.Medium),
                maxLines = 1,
            )
            Spacer(GlanceModifier.height(4.dp))
            Text(
                snapshot.labels.emptyBody,
                style = TextStyle(color = Palette.onBackgroundVariant, fontSize = 13.sp),
                maxLines = 3,
            )
        }
        Spacer(GlanceModifier.defaultWeight())
        Pill(
            icon = R.drawable.ql_ic_barcode,
            label = snapshot.labels.scanProduct,
            background = Palette.accent,
            content = Palette.onAccent,
            fontSize = 14.sp,
            action = scan,
            modifier = GlanceModifier.fillMaxWidth(),
        )
        Spacer(GlanceModifier.height(RowGap))
        Pill(
            icon = R.drawable.ql_ic_search,
            label = snapshot.labels.searchFood,
            background = Palette.row,
            content = Palette.onRow,
            fontSize = 14.sp,
            action = search,
            modifier = GlanceModifier.fillMaxWidth(),
        )
    }
}

/** A 48 dp fully rounded button: icon then label, centred. */
@Composable
private fun Pill(
    icon: Int,
    label: String,
    background: ColorProvider,
    content: ColorProvider,
    fontSize: TextUnit,
    action: Action,
    modifier: GlanceModifier,
) {
    Row(
        modifier
            .height(ButtonHeight)
            .background(background)
            .cornerRadius(24.dp)
            .clickable(action),
        verticalAlignment = Alignment.CenterVertically,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Image(
            provider = ImageProvider(icon),
            contentDescription = null,
            modifier = GlanceModifier.size(20.dp),
            colorFilter = ColorFilter.tint(content),
        )
        Spacer(GlanceModifier.width(6.dp))
        Text(
            label,
            style = TextStyle(color = content, fontSize = fontSize, fontWeight = FontWeight.Medium),
            maxLines = 1,
        )
    }
}
