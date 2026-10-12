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
        val totals = ScreenTimeCalculator.foregroundMinutes(
            events = listOf(
                event(begin + 10_000, "first.app", "first", ForegroundEventType.RESUMED),
                event(begin + 60_000, "second.app", "second", ForegroundEventType.RESUMED),
                event(begin + 100_000, "first.app", "first", ForegroundEventType.INACTIVE),
                event(begin + 129_999, "second.app", "second", ForegroundEventType.INACTIVE),
            ),
            beginTimeMillis = begin,
            endTimeMillis = begin + 180_000,
            goalExemptPackages = emptySet(),
        )

        assertEquals(ScreenTimeTotals(totalMinutes = 1, goalMinutes = 1), totals)
    }

    @Test
    fun `goal total excludes exempt apps while raw total preserves them`() {
        val begin = 1_000_000L
        val totals = ScreenTimeCalculator.foregroundMinutes(
            events = listOf(
                event(begin, "com.ichi2.anki", "anki", ForegroundEventType.RESUMED),
                event(begin + 60_000, "com.browser", "browser", ForegroundEventType.RESUMED),
                event(begin + 120_000, "com.browser", "browser", ForegroundEventType.INACTIVE),
                event(begin + 180_000, "com.ichi2.anki", "anki", ForegroundEventType.INACTIVE),
            ),
            beginTimeMillis = begin,
            endTimeMillis = begin + 180_000,
            goalExemptPackages = setOf("com.ichi2.anki"),
        )

        assertEquals(ScreenTimeTotals(totalMinutes = 3, goalMinutes = 1), totals)
    }

    @Test
    fun `aggregation seeds state before midnight and clips the result to today`() {
        val begin = 1_000_000L
        val totals = ScreenTimeCalculator.foregroundMinutes(
            events = listOf(
                event(begin - 60_000, "first.app", "first", ForegroundEventType.RESUMED),
                event(begin + 90_000, "first.app", "first", ForegroundEventType.INACTIVE),
            ),
            beginTimeMillis = begin,
            endTimeMillis = begin + 120_000,
            goalExemptPackages = emptySet(),
        )

        assertEquals(ScreenTimeTotals(totalMinutes = 1, goalMinutes = 1), totals)
    }

    @Test
    fun `device restart clears foreground state`() {
        val begin = 1_000_000L
        val totals = ScreenTimeCalculator.foregroundMinutes(
            events = listOf(
                event(begin + 10_000, "first.app", "first", ForegroundEventType.RESUMED),
                event(begin + 70_000, "", "device", ForegroundEventType.CLEAR_ALL),
            ),
            beginTimeMillis = begin,
            endTimeMillis = begin + 190_000,
            goalExemptPackages = emptySet(),
        )

        assertEquals(ScreenTimeTotals(totalMinutes = 1, goalMinutes = 1), totals)
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
                        "reader.test",
                        "reader-test",
                        ForegroundEventType.RESUMED,
                    ),
                    event(
                        localMillis(2026, Calendar.AUGUST, 5, 14, 37),
                        "reader.test",
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
        assertEquals(121, snapshot.totalMinutes)
        assertEquals(121, snapshot.goalMinutes)
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
        packageName: String,
        activityId: String,
        type: ForegroundEventType,
    ) = ForegroundEvent(timestampMillis, packageName, activityId, type)

    private fun localMillis(year: Int, month: Int, day: Int, hour: Int, minute: Int): Long =
        Calendar.getInstance(berlin).apply {
            clear()
            set(year, month, day, hour, minute, 0)
        }.timeInMillis
}
