package dev.lifestats.screentimelogger

import android.app.AppOpsManager
import android.app.usage.UsageEvents
import android.app.usage.UsageStatsManager
import android.content.Context
import android.os.Process
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Locale
import java.util.TimeZone

enum class ForegroundEventType {
    RESUMED,
    INACTIVE,
    CLEAR_ALL,
}

data class ForegroundEvent(
    val timestampMillis: Long,
    val activityId: String,
    val type: ForegroundEventType,
)

fun interface UsageEventSource {
    fun query(beginTimeMillis: Long, endTimeMillis: Long): Collection<ForegroundEvent>
}

fun interface UsageAccessChecker {
    fun hasAccess(): Boolean
}

data class UsageWindow(
    val localDate: String,
    val beginTimeMillis: Long,
    val eventQueryBeginTimeMillis: Long,
    val endTimeMillis: Long,
) {
    companion object {
        fun today(nowMillis: Long, timeZone: TimeZone): UsageWindow {
            val calendar = Calendar.getInstance(timeZone).apply {
                timeInMillis = nowMillis
                set(Calendar.HOUR_OF_DAY, 0)
                set(Calendar.MINUTE, 0)
                set(Calendar.SECOND, 0)
                set(Calendar.MILLISECOND, 0)
            }
            val formatter = SimpleDateFormat("yyyy-MM-dd", Locale.ROOT).apply {
                this.timeZone = timeZone
            }
            val previousMidnight = (calendar.clone() as Calendar).apply {
                add(Calendar.DAY_OF_MONTH, -1)
            }
            return UsageWindow(
                localDate = formatter.format(nowMillis),
                beginTimeMillis = calendar.timeInMillis,
                eventQueryBeginTimeMillis = previousMidnight.timeInMillis,
                endTimeMillis = nowMillis,
            )
        }
    }
}

data class ScreenTimeSnapshot(
    val localDate: String,
    val minutes: Long,
)

object ScreenTimeCalculator {
    fun totalForegroundMinutes(
        events: Collection<ForegroundEvent>,
        beginTimeMillis: Long,
        endTimeMillis: Long,
    ): Long {
        require(beginTimeMillis <= endTimeMillis)

        val activeActivities = mutableSetOf<String>()
        var cursor = beginTimeMillis
        var totalMillis = 0L

        events.sortedBy(ForegroundEvent::timestampMillis).forEach { event ->
            if (event.timestampMillis > endTimeMillis) {
                return@forEach
            }

            val intervalEnd = event.timestampMillis.coerceAtLeast(beginTimeMillis)
            if (activeActivities.isNotEmpty() && intervalEnd > cursor) {
                totalMillis = Math.addExact(totalMillis, intervalEnd - cursor)
            }
            cursor = maxOf(cursor, intervalEnd)

            when (event.type) {
                ForegroundEventType.RESUMED -> activeActivities += event.activityId
                ForegroundEventType.INACTIVE -> activeActivities -= event.activityId
                ForegroundEventType.CLEAR_ALL -> activeActivities.clear()
            }
        }

        if (activeActivities.isNotEmpty() && endTimeMillis > cursor) {
            totalMillis = Math.addExact(totalMillis, endTimeMillis - cursor)
        }
        return totalMillis / MILLIS_PER_MINUTE
    }

    private const val MILLIS_PER_MINUTE = 60_000L
}

class UsageAccessRequiredException : IllegalStateException(
    "Usage Access is required to read screen time",
)

class ScreenTimeReader(
    private val usageEventSource: UsageEventSource,
    private val usageAccessChecker: UsageAccessChecker,
    private val nowMillis: () -> Long = System::currentTimeMillis,
    private val timeZone: TimeZone = TimeZone.getDefault(),
) {
    fun readToday(): ScreenTimeSnapshot {
        if (!usageAccessChecker.hasAccess()) {
            throw UsageAccessRequiredException()
        }

        val window = UsageWindow.today(nowMillis(), timeZone)
        val events = usageEventSource.query(
            window.eventQueryBeginTimeMillis,
            window.endTimeMillis,
        )
        return ScreenTimeSnapshot(
            localDate = window.localDate,
            minutes = ScreenTimeCalculator.totalForegroundMinutes(
                events = events,
                beginTimeMillis = window.beginTimeMillis,
                endTimeMillis = window.endTimeMillis,
            ),
        )
    }
}

class AndroidUsageEventSource(context: Context) : UsageEventSource {
    private val usageStatsManager = context.getSystemService(UsageStatsManager::class.java)

    override fun query(
        beginTimeMillis: Long,
        endTimeMillis: Long,
    ): Collection<ForegroundEvent> {
        val events = usageStatsManager.queryEvents(beginTimeMillis, endTimeMillis)
        val event = UsageEvents.Event()
        val result = mutableListOf<ForegroundEvent>()

        while (events.hasNextEvent()) {
            events.getNextEvent(event)
            val type = when (event.eventType) {
                UsageEvents.Event.ACTIVITY_RESUMED -> ForegroundEventType.RESUMED
                UsageEvents.Event.ACTIVITY_PAUSED,
                UsageEvents.Event.ACTIVITY_STOPPED,
                -> ForegroundEventType.INACTIVE
                UsageEvents.Event.DEVICE_SHUTDOWN,
                UsageEvents.Event.DEVICE_STARTUP,
                UsageEvents.Event.SCREEN_NON_INTERACTIVE,
                UsageEvents.Event.KEYGUARD_SHOWN,
                -> ForegroundEventType.CLEAR_ALL
                else -> null
            } ?: continue

            result += ForegroundEvent(
                timestampMillis = event.timeStamp,
                activityId = activityId(event),
                type = type,
            )
        }

        return result
    }

    private fun activityId(event: UsageEvents.Event): String =
        "${event.packageName}/${event.className}"
}

class AndroidUsageAccessChecker(private val context: Context) : UsageAccessChecker {
    override fun hasAccess(): Boolean {
        val appOpsManager = context.getSystemService(AppOpsManager::class.java)
        val mode = appOpsManager.checkOpNoThrow(
            AppOpsManager.OPSTR_GET_USAGE_STATS,
            Process.myUid(),
            context.packageName,
        )
        return mode == AppOpsManager.MODE_ALLOWED
    }
}
