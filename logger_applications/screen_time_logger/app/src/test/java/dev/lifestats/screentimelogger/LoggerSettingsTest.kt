package dev.lifestats.screentimelogger

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class LoggerSettingsTest {
    @Test
    fun `settings normalize a trailing slash`() {
        assertEquals(
            LoggerSettings("http://localhost:3000", 23),
            SettingsValidator.parse(" http://localhost:3000/ ", "23"),
        )
    }

    @Test
    fun `settings reject non-http URLs and invalid statistic IDs`() {
        assertTrue(
            runCatching { SettingsValidator.parse("file:///tmp/lifestats", "23") }
                .exceptionOrNull() is IllegalArgumentException,
        )
        assertTrue(
            runCatching { SettingsValidator.parse("http://localhost:3000", "0") }
                .exceptionOrNull() is IllegalArgumentException,
        )
    }
}
