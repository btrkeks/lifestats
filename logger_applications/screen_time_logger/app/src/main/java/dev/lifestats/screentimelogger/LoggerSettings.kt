package dev.lifestats.screentimelogger

import android.content.Context
import java.net.URI

data class LoggerSettings(
    val baseUrl: String,
    val totalStatisticId: Long,
    val goalStatisticId: Long?,
    val goalExemptPackages: Set<String>,
)

object SettingsValidator {
    fun parse(
        baseUrlInput: String,
        totalStatisticIdInput: String,
        goalStatisticIdInput: String,
        goalExemptPackages: Set<String>,
    ): LoggerSettings {
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

        val totalStatisticId = totalStatisticIdInput.trim().toLongOrNull()
        require(totalStatisticId != null && totalStatisticId > 0) {
            "Total statistic ID must be a positive number"
        }

        val goalStatisticId = goalStatisticIdInput.trim()
            .takeIf(String::isNotEmpty)
            ?.toLongOrNull()
        require(goalStatisticIdInput.isBlank() || goalStatisticId != null && goalStatisticId > 0) {
            "Goal statistic ID must be blank or a positive number"
        }
        require(goalStatisticId == null || goalStatisticId != totalStatisticId) {
            "Total and goal statistic IDs must be different"
        }

        return LoggerSettings(
            baseUrl = baseUrl,
            totalStatisticId = totalStatisticId,
            goalStatisticId = goalStatisticId,
            goalExemptPackages = goalExemptPackages.map(String::trim).filter(String::isNotEmpty).toSet(),
        )
    }
}

class MissingConfigurationException : IllegalStateException(
    "Save a Lifestats base URL and statistic ID before syncing",
)

class SettingsRepository(context: Context) {
    private val preferences = context.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)

    fun loadOrNull(): LoggerSettings? {
        val baseUrl = preferences.getString(BASE_URL_KEY, null) ?: return null
        val totalStatisticId = preferences.getLong(TOTAL_STATISTIC_ID_KEY, 0)
            .takeIf { it > 0 } ?: return null
        val goalStatisticId = preferences.getLong(GOAL_STATISTIC_ID_KEY, 0).takeIf { it > 0 }
        val goalExemptPackages = preferences.getStringSet(GOAL_EXEMPT_PACKAGES_KEY, emptySet())
            .orEmpty()
            .toSet()
        return runCatching {
            SettingsValidator.parse(
                baseUrlInput = baseUrl,
                totalStatisticIdInput = totalStatisticId.toString(),
                goalStatisticIdInput = goalStatisticId?.toString().orEmpty(),
                goalExemptPackages = goalExemptPackages,
            )
        }.getOrNull()
    }

    fun load(): LoggerSettings = loadOrNull() ?: throw MissingConfigurationException()

    fun save(settings: LoggerSettings) {
        val editor = preferences.edit()
            .putString(BASE_URL_KEY, settings.baseUrl)
            .putLong(TOTAL_STATISTIC_ID_KEY, settings.totalStatisticId)
            .putStringSet(GOAL_EXEMPT_PACKAGES_KEY, settings.goalExemptPackages)
        settings.goalStatisticId?.let {
            editor.putLong(GOAL_STATISTIC_ID_KEY, it)
        } ?: editor.remove(GOAL_STATISTIC_ID_KEY)
        editor.apply()
    }

    companion object {
        private const val PREFERENCES_NAME = "screen_time_logger"
        private const val BASE_URL_KEY = "base_url"
        // Keep the original key so existing installs migrate without losing the total statistic.
        private const val TOTAL_STATISTIC_ID_KEY = "statistic_id"
        private const val GOAL_STATISTIC_ID_KEY = "goal_statistic_id"
        private const val GOAL_EXEMPT_PACKAGES_KEY = "goal_exempt_packages"
    }
}
