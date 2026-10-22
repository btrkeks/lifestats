package expo.modules.nativelib

import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

internal fun instanceStartsOnDay(
    beginMs: Long,
    allDay: Boolean,
    dayStartMs: Long,
    dayEndMs: Long,
    zone: TimeZone = TimeZone.getDefault(),
): Boolean {
    if (!allDay) return beginMs >= dayStartMs && beginMs < dayEndMs
    val format = SimpleDateFormat("yyyy-MM-dd", Locale.ROOT)
    format.timeZone = TimeZone.getTimeZone("UTC")
    val calendarDate = format.format(Date(beginMs))
    format.timeZone = zone
    return calendarDate == format.format(Date(dayStartMs))
}
