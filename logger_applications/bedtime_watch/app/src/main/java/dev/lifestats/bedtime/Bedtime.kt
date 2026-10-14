package dev.lifestats.bedtime

import java.time.LocalDate
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

/** A bedtime as Lifestats stores it: under the evening's date, even when it is past midnight. */
data class Bedtime(val eveningDate: LocalDate, val time: String) {
    companion object {
        private val TIME_FORMAT = DateTimeFormatter.ofPattern("HH:mm")

        /** Bedtimes before this hour belong to the previous evening. */
        private const val EVENING_STARTS_AT_HOUR = 12

        fun at(now: LocalDateTime): Bedtime {
            val date = now.toLocalDate()
            val eveningDate = if (now.hour < EVENING_STARTS_AT_HOUR) date.minusDays(1) else date
            return Bedtime(eveningDate, now.format(TIME_FORMAT))
        }
    }
}

fun logUrl(baseUrl: String, statisticId: Long, date: LocalDate): String =
    "${baseUrl.trimEnd('/')}/statistics/$statisticId/log/$date"

fun logRequestBody(time: String): String = """{"value":"$time","update_policy":"overwrite"}"""
