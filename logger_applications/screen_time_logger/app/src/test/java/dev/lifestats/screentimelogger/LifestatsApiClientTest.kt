package dev.lifestats.screentimelogger

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class LifestatsApiClientTest {
    @Test
    fun `requests log total and goal screen time with overwrite`() {
        val requests = LifestatsRequestFactory.logScreenTime(
            settings = LoggerSettings(
                baseUrl = "http://localhost:3000",
                totalStatisticId = 42,
                goalStatisticId = 43,
                goalExemptPackages = setOf("com.ichi2.anki"),
            ),
            snapshot = ScreenTimeSnapshot("2026-08-05", totalMinutes = 187, goalMinutes = 52),
        )

        assertEquals(
            "http://localhost:3000/statistics/42/log/2026-08-05",
            requests[0].url,
        )
        assertEquals(
            "{\"value\":\"187\",\"update_policy\":\"overwrite\"}",
            requests[0].body,
        )
        assertEquals(
            "http://localhost:3000/statistics/43/log/2026-08-05",
            requests[1].url,
        )
        assertEquals(
            "{\"value\":\"52\",\"update_policy\":\"overwrite\"}",
            requests[1].body,
        )
    }

    @Test
    fun `existing settings without a goal statistic still log the raw total`() {
        val requests = LifestatsRequestFactory.logScreenTime(
            settings = LoggerSettings(
                baseUrl = "http://localhost:3000",
                totalStatisticId = 42,
                goalStatisticId = null,
                goalExemptPackages = emptySet(),
            ),
            snapshot = ScreenTimeSnapshot("2026-08-05", totalMinutes = 187, goalMinutes = 187),
        )

        assertEquals(1, requests.size)
        assertEquals("http://localhost:3000/statistics/42/log/2026-08-05", requests.single().url)
    }

    @Test
    fun `retry classification is limited to transient HTTP failures`() {
        assertTrue(LifestatsHttpException(408, "").isRetryable)
        assertTrue(LifestatsHttpException(429, "").isRetryable)
        assertTrue(LifestatsHttpException(503, "").isRetryable)
        assertEquals(false, LifestatsHttpException(400, "").isRetryable)
    }
}
