package dev.lifestats.screentimelogger

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class LifestatsApiClientTest {
    @Test
    fun `request logs the daily total with overwrite`() {
        val request = LifestatsRequestFactory.logScreenTime(
            settings = LoggerSettings("http://localhost:3000", 42),
            snapshot = ScreenTimeSnapshot("2026-08-05", 187),
        )

        assertEquals(
            "http://localhost:3000/statistics/42/log/2026-08-05",
            request.url,
        )
        assertEquals(
            "{\"value\":\"187\",\"update_policy\":\"overwrite\"}",
            request.body,
        )
    }

    @Test
    fun `retry classification is limited to transient HTTP failures`() {
        assertTrue(LifestatsHttpException(408, "").isRetryable)
        assertTrue(LifestatsHttpException(429, "").isRetryable)
        assertTrue(LifestatsHttpException(503, "").isRetryable)
        assertEquals(false, LifestatsHttpException(400, "").isRetryable)
    }
}
