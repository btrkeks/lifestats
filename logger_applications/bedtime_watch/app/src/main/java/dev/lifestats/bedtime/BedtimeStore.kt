package dev.lifestats.bedtime

import android.content.Context
import android.content.SharedPreferences
import java.time.LocalDate

enum class SyncStatus { PENDING, SENT, REJECTED }

data class LoggedBedtime(val bedtime: Bedtime, val status: SyncStatus)

/** The most recently logged bedtime and whether it reached Lifestats yet. */
class BedtimeStore(context: Context) {
    val prefs: SharedPreferences = context.getSharedPreferences("bedtime", Context.MODE_PRIVATE)

    fun last(): LoggedBedtime? {
        val date = prefs.getString(KEY_DATE, null) ?: return null
        val time = prefs.getString(KEY_TIME, null) ?: return null
        val status = SyncStatus.valueOf(prefs.getString(KEY_STATUS, SyncStatus.PENDING.name)!!)
        return LoggedBedtime(Bedtime(LocalDate.parse(date), time), status)
    }

    fun save(bedtime: Bedtime, status: SyncStatus) {
        prefs.edit()
            .putString(KEY_DATE, bedtime.eveningDate.toString())
            .putString(KEY_TIME, bedtime.time)
            .putString(KEY_STATUS, status.name)
            .apply()
    }

    /** A late upload must not overwrite the status of a bedtime logged after it. */
    fun markIfLatest(bedtime: Bedtime, status: SyncStatus) {
        if (last()?.bedtime == bedtime) save(bedtime, status)
    }

    private companion object {
        const val KEY_DATE = "date"
        const val KEY_TIME = "time"
        const val KEY_STATUS = "status"
    }
}
