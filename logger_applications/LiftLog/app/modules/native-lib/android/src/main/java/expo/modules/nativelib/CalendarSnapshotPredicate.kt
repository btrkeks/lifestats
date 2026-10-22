package expo.modules.nativelib

import android.provider.CalendarContract.Events

internal data class CalendarSnapshotPredicate(val selection: String, val args: List<String>)

internal fun calendarSnapshotPredicate(
    title: String?,
    organizer: String?,
    guestsCanModify: Int?,
): CalendarSnapshotPredicate {
    val args = mutableListOf<String>()
    fun equalSnapshot(column: String, value: String?): String {
        if (value == null) return "$column IS NULL"
        args.add(value)
        return "$column IS ?"
    }
    val selection = listOf(
        equalSnapshot(Events.TITLE, title),
        equalSnapshot(Events.ORGANIZER, organizer),
        equalSnapshot(Events.GUESTS_CAN_MODIFY, guestsCanModify?.toString()),
    ).joinToString(" AND ")
    return CalendarSnapshotPredicate(selection, args)
}
