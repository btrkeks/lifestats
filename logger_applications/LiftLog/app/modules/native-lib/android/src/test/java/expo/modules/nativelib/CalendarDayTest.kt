package expo.modules.nativelib

import java.time.LocalDate
import java.time.ZoneId
import java.time.ZoneOffset
import java.util.TimeZone
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CalendarDayTest {
    @Test fun allDayUsesUtcCalendarDateInBothOffsetDirections() {
        for (zoneName in listOf("Pacific/Kiritimati", "America/Los_Angeles", "Europe/Berlin")) {
            val zone = ZoneId.of(zoneName)
            for (date in listOf(LocalDate.parse("2026-03-29"), LocalDate.parse("2026-10-25"))) {
                val timeZone = TimeZone.getTimeZone(zoneName)
                val start = date.atStartOfDay(zone).toInstant().toEpochMilli()
                val end = date.plusDays(1).atStartOfDay(zone).toInstant().toEpochMilli()
                val todayUtc = date.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli()
                assertTrue(instanceStartsOnDay(todayUtc, true, start, end, timeZone))
                assertFalse(instanceStartsOnDay(todayUtc - 86_400_000, true, start, end, timeZone))
                assertFalse(instanceStartsOnDay(todayUtc + 86_400_000, true, start, end, timeZone))
                assertTrue(todayUtc >= start - 86_400_000 && todayUtc < end + 86_400_000)
                assertTrue(instanceStartsOnDay(start, false, start, end, timeZone))
                assertFalse(instanceStartsOnDay(start - 1, false, start, end, timeZone))
                assertFalse(instanceStartsOnDay(end, false, start, end, timeZone))
            }
        }
    }
}
