package dev.lifestats.screentimelogger

import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.nio.charset.StandardCharsets

data class LogValueRequest(
    val url: String,
    val body: String,
)

object LifestatsRequestFactory {
    fun logScreenTime(settings: LoggerSettings, snapshot: ScreenTimeSnapshot): List<LogValueRequest> {
        require(snapshot.totalMinutes >= 0) { "Total screen-time minutes cannot be negative" }
        require(snapshot.goalMinutes >= 0) { "Goal screen-time minutes cannot be negative" }
        require(snapshot.goalMinutes <= snapshot.totalMinutes) {
            "Goal screen time cannot exceed total screen time"
        }
        require(LOCAL_DATE_PATTERN.matches(snapshot.localDate)) { "Date must use YYYY-MM-DD" }

        return buildList {
            add(logValue(settings.baseUrl, settings.totalStatisticId, snapshot.localDate, snapshot.totalMinutes))
            settings.goalStatisticId?.let { statisticId ->
                add(logValue(settings.baseUrl, statisticId, snapshot.localDate, snapshot.goalMinutes))
            }
        }
    }

    private fun logValue(
        baseUrl: String,
        statisticId: Long,
        localDate: String,
        minutes: Long,
    ) = LogValueRequest(
        url = "$baseUrl/statistics/$statisticId/log/$localDate",
        body = "{\"value\":\"$minutes\",\"update_policy\":\"overwrite\"}",
    )

    private val LOCAL_DATE_PATTERN = Regex("\\d{4}-\\d{2}-\\d{2}")
}

class LifestatsHttpException(
    val statusCode: Int,
    responseBody: String,
) : IOException("Lifestats returned HTTP $statusCode: ${responseBody.take(512)}") {
    val isRetryable: Boolean
        get() = statusCode == 408 || statusCode == 429 || statusCode >= 500
}

class LifestatsApiClient(
    private val connectTimeoutMillis: Int = 15_000,
    private val readTimeoutMillis: Int = 15_000,
) {
    fun logScreenTime(settings: LoggerSettings, snapshot: ScreenTimeSnapshot) {
        LifestatsRequestFactory.logScreenTime(settings, snapshot).forEach(::execute)
    }

    private fun execute(request: LogValueRequest) {
        val bodyBytes = request.body.toByteArray(StandardCharsets.UTF_8)
        val connection = URL(request.url).openConnection() as HttpURLConnection
        try {
            connection.requestMethod = "POST"
            connection.connectTimeout = connectTimeoutMillis
            connection.readTimeout = readTimeoutMillis
            connection.doOutput = true
            connection.setRequestProperty("Content-Type", "application/json; charset=utf-8")
            connection.setFixedLengthStreamingMode(bodyBytes.size)
            connection.outputStream.use { it.write(bodyBytes) }

            val statusCode = connection.responseCode
            if (statusCode !in 200..299) {
                val responseBody = connection.errorStream
                    ?.bufferedReader(StandardCharsets.UTF_8)
                    ?.use { it.readText() }
                    .orEmpty()
                throw LifestatsHttpException(statusCode, responseBody)
            }
        } finally {
            connection.disconnect()
        }
    }
}
