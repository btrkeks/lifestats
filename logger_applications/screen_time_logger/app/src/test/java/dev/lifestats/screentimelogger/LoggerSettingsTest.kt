package dev.lifestats.screentimelogger

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class LoggerSettingsTest {
    @Test
    fun `settings normalize a trailing slash`() {
        assertEquals(
            LoggerSettings(
                baseUrl = "http://localhost:3000",
                totalStatisticId = 23,
                goalStatisticId = 24,
                goalExemptPackages = setOf("com.ichi2.anki"),
            ),
            SettingsValidator.parse(
                baseUrlInput = " http://localhost:3000/ ",
                totalStatisticIdInput = "23",
                goalStatisticIdInput = "24",
                goalExemptPackages = setOf(" com.ichi2.anki ", ""),
            ),
        )
    }

    @Test
    fun `goal statistic remains optional for existing installations`() {
        assertEquals(
            null,
            SettingsValidator.parse(
                baseUrlInput = "http://localhost:3000",
                totalStatisticIdInput = "23",
                goalStatisticIdInput = "",
                goalExemptPackages = emptySet(),
            ).goalStatisticId,
        )
    }

    @Test
    fun `settings reject non-http URLs and invalid statistic IDs`() {
        assertTrue(
            runCatching {
                SettingsValidator.parse("file:///tmp/lifestats", "23", "", emptySet())
            }
                .exceptionOrNull() is IllegalArgumentException,
        )
        assertTrue(
            runCatching {
                SettingsValidator.parse("http://localhost:3000", "0", "", emptySet())
            }
                .exceptionOrNull() is IllegalArgumentException,
        )
        assertTrue(
            runCatching {
                SettingsValidator.parse("http://localhost:3000", "23", "23", emptySet())
            }
                .exceptionOrNull() is IllegalArgumentException,
        )
    }
}
