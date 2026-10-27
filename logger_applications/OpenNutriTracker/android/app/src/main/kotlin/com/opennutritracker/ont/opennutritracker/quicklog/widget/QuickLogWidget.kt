package com.opennutritracker.ont.opennutritracker.quicklog.widget

import android.content.Context
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.glance.GlanceId
import androidx.glance.LocalSize
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.SizeMode
import androidx.glance.appwidget.provideContent
import androidx.glance.appwidget.updateAll
import com.opennutritracker.ont.opennutritracker.quicklog.QuickLogStore
import com.opennutritracker.ont.opennutritracker.quicklog.QuickLogTimers
import com.opennutritracker.ont.opennutritracker.quicklog.WidgetSnapshot

/**
 * The quick-log home-screen widget, 4×2 by default.
 *
 * State lives in [QuickLogStore], not in per-widget Glance state: it is one
 * fact for the whole process, however many copies of the widget are placed.
 */
class QuickLogWidget : GlanceAppWidget() {

    /** Exact: the ring and list are fitted to the real cell size, which varies by launcher. */
    override val sizeMode = SizeMode.Exact

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val store = QuickLogStore.model(context)
        if (store.value.snapshot is WidgetSnapshot.None) QuickLogTimers.requestRefresh(context)
        provideContent {
            val model by store.collectAsState()
            val now = System.currentTimeMillis()
            if (model.isStale(now)) {
                LaunchedEffect(model.snapshot) { QuickLogTimers.requestRefresh(context) }
            }
            QuickLogContent(model, now, LocalSize.current)
        }
    }

    companion object {
        suspend fun refreshAll(context: Context) {
            QuickLogWidget().updateAll(context)
        }
    }
}

/** Declared with its full class name: the develop flavour changes the application id, not this package. */
class QuickLogWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = QuickLogWidget()

    /** The first widget placed fills in without the user opening the app. */
    override fun onEnabled(context: Context) {
        super.onEnabled(context)
        QuickLogTimers.requestRefresh(context)
    }
}
