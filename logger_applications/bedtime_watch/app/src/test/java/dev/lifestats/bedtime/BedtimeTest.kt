package dev.lifestats.bedtime

import org.junit.Assert.assertEquals
import org.junit.Test
import java.time.LocalDate
import java.time.LocalDateTime

class BedtimeTest {
    @Test
    fun eveningBedtimeIsFiledUnderTheSameDay() {
        assertEquals(
            Bedtime(LocalDate.of(2026, 10, 13), "22:30"),
            Bedtime.at(LocalDateTime.of(2026, 10, 13, 22, 30, 59)),
        )
    }

    @Test
    fun bedtimeAfterMidnightIsFiledUnderThePreviousEvening() {
        assertEquals(
            Bedtime(LocalDate.of(2026, 10, 13), "00:24"),
            Bedtime.at(LocalDateTime.of(2026, 10, 14, 0, 24)),
        )
    }

    @Test
    fun eveningStartsAtNoon() {
        assertEquals(LocalDate.of(2026, 10, 13), Bedtime.at(LocalDateTime.of(2026, 10, 14, 11, 59)).eveningDate)
        assertEquals(LocalDate.of(2026, 10, 14), Bedtime.at(LocalDateTime.of(2026, 10, 14, 12, 0)).eveningDate)
    }

    @Test
    fun buildsTheOverwritingLogRequest() {
        assertEquals(
            "http://192.168.68.74:3000/statistics/20/log/2026-10-13",
            logUrl("http://192.168.68.74:3000/", 20, LocalDate.of(2026, 10, 13)),
        )
        assertEquals("""{"value":"00:24","update_policy":"overwrite"}""", logRequestBody("00:24"))
    }
}
