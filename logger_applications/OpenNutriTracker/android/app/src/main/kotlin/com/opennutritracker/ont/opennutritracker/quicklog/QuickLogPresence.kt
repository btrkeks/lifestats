package com.opennutritracker.ont.opennutritracker.quicklog

import android.appwidget.AppWidgetManager
import android.content.ComponentName
import android.content.Context
import com.opennutritracker.ont.opennutritracker.quicklog.widget.QuickLogWidgetReceiver

object QuickLogPresence {
    fun isPlaced(context: Context): Boolean = AppWidgetManager.getInstance(context)
        .getAppWidgetIds(ComponentName(context, QuickLogWidgetReceiver::class.java)).isNotEmpty()
}
