package dev.lifestats.screentimelogger

import android.content.Context
import java.net.URI

data class LoggerSettings(
    val baseUrl: String,
    val statisticId: Long,
)

object SettingsValidator {
    fun parse(baseUrlInput: String, statisticIdInput: String): LoggerSettings {
        val baseUrl = baseUrlInput.trim().trimEnd('/')
        require(baseUrl.isNotEmpty()) { "Base URL is required" }

        val uri = runCatching { URI(baseUrl) }
            .getOrElse { throw IllegalArgumentException("Base URL is not a valid URL") }
        require(uri.scheme.equals("http", ignoreCase = true) || uri.scheme.equals("https", ignoreCase = true)) {
            "Base URL must use http or https"
        }
        require(!uri.host.isNullOrBlank()) { "Base URL must include a host" }
        require(uri.rawQuery == null && uri.rawFragment == null && uri.rawUserInfo == null) {
            "Base URL cannot include credentials, a query, or a fragment"
        }

        val statisticId = statisticIdInput.trim().toLongOrNull()
        require(statisticId != null && statisticId > 0) { "Statistic ID must be a positive number" }

        return LoggerSettings(baseUrl = baseUrl, statisticId = statisticId)
    }
}

class MissingConfigurationException : IllegalStateException(
    "Save a Lifestats base URL and statistic ID before syncing",
)

class SettingsRepository(context: Context) {
    private val preferences = context.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)

    fun loadOrNull(): LoggerSettings? {
        val baseUrl = preferences.getString(BASE_URL_KEY, null) ?: return null
        val statisticId = preferences.getLong(STATISTIC_ID_KEY, 0).takeIf { it > 0 } ?: return null
        return runCatching { SettingsValidator.parse(baseUrl, statisticId.toString()) }.getOrNull()
    }

    fun load(): LoggerSettings = loadOrNull() ?: throw MissingConfigurationException()

    fun save(settings: LoggerSettings) {
        preferences.edit()
            .putString(BASE_URL_KEY, settings.baseUrl)
            .putLong(STATISTIC_ID_KEY, settings.statisticId)
            .apply()
    }

    companion object {
        private const val PREFERENCES_NAME = "screen_time_logger"
        private const val BASE_URL_KEY = "base_url"
        private const val STATISTIC_ID_KEY = "statistic_id"
    }
}
