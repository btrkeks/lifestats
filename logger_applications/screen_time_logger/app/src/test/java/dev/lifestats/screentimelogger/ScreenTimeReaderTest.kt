package dev.lifestats.screentimelogger

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.Calendar
import java.util.TimeZone

class ScreenTimeReaderTest {
    private val berlin = TimeZone.getTimeZone("Europe/Berlin")

    @Test
    fun `aggregation unions overlapping foreground activities and floors partial minutes`() {
        val begin = 1_000_000L
        val minutes = ScreenTimeCalculator.totalForegroundMinutes(
            events = listOf(
                event(begin + 10_000, "first", ForegroundEventType.RESUMED),
                event(begin + 60_000, "second", ForegroundEventType.RESUMED),
                event(begin + 100_000, "first", ForegroundEventType.INACTIVE),
                event(begin + 129_999, "second", ForegroundEventType.INACTIVE),
            ),
            beginTimeMillis = begin,
            endTimeMillis = begin + 180_000,
        )

        assertEquals(1, minutes)
    }

    @Test
    fun `aggregation seeds state before midnight and clips the result to today`() {
        val begin = 1_000_000L
        val minutes = ScreenTimeCalculator.totalForegroundMinutes(
            events = listOf(
                event(begin - 60_000, "first", ForegroundEventType.RESUMED),
                event(begin + 90_000, "first", ForegroundEventType.INACTIVE),
            ),
            beginTimeMillis = begin,
            endTimeMillis = begin + 120_000,
        )

        assertEquals(1, minutes)
    }

    @Test
    fun `device restart clears foreground state`() {
        val begin = 1_000_000L
        val minutes = ScreenTimeCalculator.totalForegroundMinutes(
            events = listOf(
                event(begin + 10_000, "first", ForegroundEventType.RESUMED),
                event(begin + 70_000, "device", ForegroundEventType.CLEAR_ALL),
            ),
            beginTimeMillis = begin,
            endTimeMillis = begin + 190_000,
        )

        assertEquals(1, minutes)
    }

    @Test
    fun `reader queries previous local midnight and clips aggregation to today`() {
        val now = localMillis(2026, Calendar.AUGUST, 5, 14, 37)
        var queriedBegin = -1L
        var queriedEnd = -1L
        val reader = ScreenTimeReader(
            usageEventSource = UsageEventSource { begin, end ->
                queriedBegin = begin
                queriedEnd = end
                listOf(
                    event(
                        localMillis(2026, Calendar.AUGUST, 5, 12, 36),
                        "reader-test",
                        ForegroundEventType.RESUMED,
                    ),
                    event(
                        localMillis(2026, Calendar.AUGUST, 5, 14, 37),
                        "reader-test",
                        ForegroundEventType.INACTIVE,
                    ),
                )
            },
            usageAccessChecker = UsageAccessChecker { true },
            nowMillis = { now },
            timeZone = berlin,
        )

        val snapshot = reader.readToday()

        assertEquals("2026-08-05", snapshot.localDate)
        assertEquals(121, snapshot.minutes)
        assertEquals(localMillis(2026, Calendar.AUGUST, 4, 0, 0), queriedBegin)
        assertEquals(now, queriedEnd)
    }

    @Test
    fun `spring daylight-saving boundary uses the real local midnight`() {
        val now = localMillis(2026, Calendar.MARCH, 29, 12, 0)
        val window = UsageWindow.today(now, berlin)

        assertEquals("2026-03-29", window.localDate)
        assertEquals(11 * 60 * 60 * 1000L, window.endTimeMillis - window.beginTimeMillis)
    }

    @Test
    fun `autumn daylight-saving boundary uses the real local midnight`() {
        val now = localMillis(2026, Calendar.OCTOBER, 25, 12, 0)
        val window = UsageWindow.today(now, berlin)

        assertEquals("2026-10-25", window.localDate)
        assertEquals(13 * 60 * 60 * 1000L, window.endTimeMillis - window.beginTimeMillis)
    }

    @Test
    fun `reader refuses to treat missing usage access as zero minutes`() {
        var queried = false
        val reader = ScreenTimeReader(
            usageEventSource = UsageEventSource { _, _ ->
                queried = true
                emptyList()
            },
            usageAccessChecker = UsageAccessChecker { false },
            nowMillis = { 0L },
            timeZone = berlin,
        )

        val error = runCatching { reader.readToday() }.exceptionOrNull()

        assertTrue(error is UsageAccessRequiredException)
        assertEquals(false, queried)
    }

    private fun event(
        timestampMillis: Long,
        activityId: String,
        type: ForegroundEventType,
    ) = ForegroundEvent(timestampMillis, activityId, type)

    private fun localMillis(year: Int, month: Int, day: Int, hour: Int, minute: Int): Long =
        Calendar.getInstance(berlin).apply {
            clear()
            set(year, month, day, hour, minute, 0)
        }.timeInMillis
}
