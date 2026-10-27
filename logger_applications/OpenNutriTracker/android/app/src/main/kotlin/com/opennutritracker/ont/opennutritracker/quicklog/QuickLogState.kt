package com.opennutritracker.ont.opennutritracker.quicklog

import kotlinx.coroutines.flow.MutableStateFlow

internal class QuickLogState(
    initial: WidgetModel,
    private val persistRecent: (RecentAction?) -> Unit,
) {
    val flow = MutableStateFlow(initial)

    @Synchronized
    fun update(change: (WidgetModel) -> WidgetModel) {
        flow.value = change(flow.value)
    }

    @Synchronized
    fun show(action: RecentAction) {
        persistRecent(action)
        update { it.copy(recentAction = action) }
    }

    @Synchronized
    fun clear(opId: String) {
        if (flow.value.recentAction?.opId != opId) return
        persistRecent(null)
        update { it.copy(recentAction = null) }
    }
}

internal fun undoSucceeded(reply: QuickLogReply): Boolean = reply is QuickLogReply.Ok
