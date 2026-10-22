package expo.modules.nativelib

import android.Manifest
import android.content.ContentResolver
import android.content.ContentUris
import android.content.ContentValues
import android.content.pm.PackageManager
import android.database.Cursor
import android.net.Uri
import android.provider.CalendarContract.Calendars
import android.provider.CalendarContract.Events
import android.provider.CalendarContract.Instances
import expo.modules.kotlin.exception.Exceptions
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition
import expo.modules.kotlin.records.Field
import expo.modules.kotlin.records.Record

/** `instanceTime` is null for an event that does not repeat. */
class OccurrenceInput : Record {
    @Field val calendarId: Long = 0
    @Field val accountName: String = ""
    @Field val eventId: Long = 0
    @Field val instanceTime: Long? = null
    @Field val syncId: String? = null
    @Field val rrule: String? = null
    @Field val title: String = ""
    @Field val organizer: String? = null
    @Field val sourceStartMs: Long? = null
    @Field val sourceDuration: String? = null
    @Field val sourceTimezone: String? = null
}

class SpanInput : Record {
    @Field val beginMs: Long = 0
    @Field val endMs: Long = 0
}

class LiftLogCalendarModule : Module() {
    // Serializes resolve-then-write, so two runs can never both see no exception and insert one each.
    private val writeLock = Any()

    private data class Span(val beginMs: Long, val endMs: Long)

    private sealed class Target {
        abstract val current: Span

        data class Untouched(val masterId: Long, val instanceTime: Long, val status: Int?, override val current: Span) :
            Target()

        data class Row(val id: Long, val event: EventRow, override val current: Span) : Target()
    }

    private sealed class Resolved {
        data class Found(val target: Target) : Resolved()
        data class Problem(val status: String) : Resolved()
    }

    private data class CalendarRow(val accountName: String?, val accessLevel: Int, val ownerAccount: String?, val accountType: String?)

    private data class EventRow(
        val calendarId: Long,
        val deleted: Boolean,
        val status: Int?,
        val rrule: String?,
        val rdate: String?,
        val originalId: Long?,
        val allDay: Boolean,
        val dtstart: Long?,
        val dtend: Long?,
        val syncId: String?,
        val title: String?,
        val organizer: String?,
        val guestsCanModify: Int?,
        val originalTime: Long?,
        val exrule: String?,
        val exdate: String?,
        val duration: String?,
        val timezone: String?,
    )

    override fun definition() = ModuleDefinition {
        Name("LiftLogCalendar")

        AsyncFunction("listCalendars") {
            if (!hasAccess()) return@AsyncFunction mapOf("status" to "noPermission")
            val calendars = query(
                Calendars.CONTENT_URI,
                arrayOf(Calendars._ID, Calendars.CALENDAR_DISPLAY_NAME, Calendars.ACCOUNT_NAME),
                "${Calendars.VISIBLE} = 1 AND ${Calendars.SYNC_EVENTS} = 1 AND " +
                    "${Calendars.CALENDAR_ACCESS_LEVEL} >= ${Calendars.CAL_ACCESS_CONTRIBUTOR} AND " +
                    "${Calendars.ACCOUNT_TYPE} = 'com.google'",
                emptyArray(),
            ) { c ->
                mapOf(
                    "id" to c.getLong(0),
                    "displayName" to c.stringOrNull(1),
                    "accountName" to c.stringOrNull(2),
                )
            }
            mapOf("status" to "ok", "calendars" to calendars.filter { it["accountName"] != null })
        }

        /** Returns timed starts on the local day and all-day matches by their UTC calendar date. */
        AsyncFunction("findInstances") { calendarId: Long, accountName: String, beginMs: Long, endMs: Long ->
            if (!hasAccess()) return@AsyncFunction mapOf("status" to "noPermission")
            val calendar = readCalendar(calendarId)
            if (calendar == null || calendar.accountName != accountName || calendar.accountType != "com.google") {
                return@AsyncFunction mapOf("status" to "calendarMissing")
            }
            val uri = Instances.CONTENT_URI.buildUpon()
                .appendPath((beginMs - 86_400_000).toString())
                .appendPath((endMs + 86_400_000).toString())
                .build()
            val instances = query(
                uri,
                arrayOf(
                    Instances.EVENT_ID, Instances.BEGIN, Instances.END, Instances.TITLE, Instances.ALL_DAY,
                    Instances.RRULE, Instances.RDATE, Instances.ORIGINAL_ID, Instances.ORIGINAL_INSTANCE_TIME,
                    Instances.ORGANIZER, Instances.GUESTS_CAN_MODIFY, Events._SYNC_ID,
                ),
                "${Instances.CALENDAR_ID} = ? AND (${Instances.STATUS} IS NULL OR " +
                    "${Instances.STATUS} != ${Events.STATUS_CANCELED})",
                arrayOf(calendarId.toString()),
            ) { c ->
                val allDay = c.getInt(4) != 0
                val begins = c.getLong(1)
                val onDay = instanceStartsOnDay(begins, allDay, beginMs, endMs)
                if (!onDay) return@query null
                val originalId = c.longOrNull(7)
                val master = readEvent(originalId ?: c.getLong(0))
                val repeats = originalId == null && (!c.stringOrNull(5).isNullOrEmpty() || !c.stringOrNull(6).isNullOrEmpty())
                val organizer = c.stringOrNull(9)
                mapOf(
                    "eventId" to (originalId ?: c.getLong(0)),
                    "instanceTime" to when {
                        originalId != null -> c.longOrNull(8)
                        repeats -> c.getLong(1)
                        else -> null
                    },
                    "syncId" to if (originalId != null) master?.syncId else c.stringOrNull(11),
                    "rrule" to if (originalId != null) master?.rrule else c.stringOrNull(5),
                    "rdate" to if (originalId != null) master?.rdate else c.stringOrNull(6),
                    "title" to c.stringOrNull(3),
                    "beginMs" to if (allDay) beginMs else begins,
                    "endMs" to if (allDay) endMs else c.getLong(2),
                    "organizer" to master?.organizer,
                    "sourceStartMs" to if (originalId != null || repeats) master?.dtstart else null,
                    "sourceDuration" to if (originalId != null || repeats) master?.duration else null,
                    "sourceTimezone" to master?.timezone,
                    "exclusions" to (master == null || !master.exrule.isNullOrEmpty() || !master.exdate.isNullOrEmpty()),
                    "allDay" to (c.getInt(4) != 0),
                    "editable" to (
                        master?.title == c.stringOrNull(3) && calendar.accessLevel >= Calendars.CAL_ACCESS_CONTRIBUTOR &&
                            (organizer.isNullOrEmpty() || organizer == calendar.ownerAccount || c.getInt(10) != 0)
                        ),
                )
            }
            mapOf("status" to "ok", "instances" to instances.filterNotNull())
        }

        AsyncFunction("inspectOccurrence") { input: OccurrenceInput ->
            when (val resolved = resolve(input)) {
                is Resolved.Problem -> mapOf("status" to resolved.status)
                is Resolved.Found -> spanResult("ok", resolved.target.current)
            }
        }

        /** Moves only this occurrence if it still shows expected, and verifies the resulting time. */
        AsyncFunction("moveOccurrence") { input: OccurrenceInput, next: SpanInput, expected: SpanInput ->
            val to = Span(next.beginMs, next.endMs)
            require(to.endMs > to.beginMs && (to.endMs - to.beginMs) % 1000 == 0L) {
                "A moved occurrence must last a positive number of whole seconds"
            }
            synchronized(writeLock) {
                val target = when (val resolved = resolve(input)) {
                    is Resolved.Problem -> return@synchronized mapOf("status" to resolved.status)
                    is Resolved.Found -> resolved.target
                }
                if (target.current == to) return@synchronized mapOf("status" to "unchanged")
                if (target.current != Span(expected.beginMs, expected.endMs)) {
                    return@synchronized spanResult("changed", target.current)
                }
                when (target) {
                    is Target.Untouched -> insertException(target, to)
                    is Target.Row -> if (!updateRow(input, target, to)) {
                        return@synchronized readBack(input, to)
                    }
                }
                readBack(input, to)
            }
        }
    }

    private val resolver: ContentResolver
        get() = appContext.reactContext?.contentResolver ?: throw Exceptions.ReactContextLost()

    private fun hasAccess(): Boolean {
        val context = appContext.reactContext ?: return false
        return listOf(Manifest.permission.READ_CALENDAR, Manifest.permission.WRITE_CALENDAR).all {
            context.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED
        }
    }

    private fun resolve(input: OccurrenceInput): Resolved {
        if (!hasAccess()) return Resolved.Problem("noPermission")
        val calendar = readCalendar(input.calendarId)
        if (calendar == null || calendar.accountName != input.accountName || calendar.accountType != "com.google") return Resolved.Problem("calendarMissing")
        if (calendar.accessLevel < Calendars.CAL_ACCESS_CONTRIBUTOR) return Resolved.Problem("readOnly")

        val event = readEvent(input.eventId)
        if (event == null || event.deleted || event.calendarId != input.calendarId) return Resolved.Problem("gone")
        if (event.status == Events.STATUS_CANCELED) return Resolved.Problem("gone")
        if (!input.syncId.isNullOrEmpty() && event.syncId != input.syncId) return Resolved.Problem("gone")
        if (event.allDay || !event.rdate.isNullOrEmpty() || !event.exrule.isNullOrEmpty() || !event.exdate.isNullOrEmpty()) {
            return Resolved.Problem("unsupported")
        }
        if (!editable(event, calendar)) return Resolved.Problem("readOnly")
        if (event.title != input.title || event.organizer.orEmpty() != input.organizer.orEmpty() ||
            event.timezone.orEmpty() != input.sourceTimezone.orEmpty()) return Resolved.Problem("changed")

        val instanceTime = input.instanceTime ?: return resolveSingle(input.eventId, event)

        if (event.rrule.isNullOrEmpty()) {
            return Resolved.Problem(if (event.rdate.isNullOrEmpty()) "gone" else "unsupported")
        }
        if (event.rrule != input.rrule) return Resolved.Problem("gone")
        if (event.dtstart != input.sourceStartMs || event.duration.orEmpty() != input.sourceDuration.orEmpty()) {
            return Resolved.Problem("changed")
        }
        // The provider drops the other occurrences of a series without a sync id when one of them is moved.
        if (event.syncId.isNullOrEmpty()) return Resolved.Problem("unsynced")

        val exceptions = query(
            Events.CONTENT_URI,
            arrayOf(Events._ID),
            "${Events.ORIGINAL_ID} = ? AND ${Events.ORIGINAL_INSTANCE_TIME} = ? AND ${Events.DELETED} = 0 AND " +
                "${Events.CALENDAR_ID} = ?",
            arrayOf(input.eventId.toString(), instanceTime.toString(), input.calendarId.toString()),
        ) { c ->
            val id = c.getLong(0)
            val row = readEvent(id)
            when {
                row == null || row.status == Events.STATUS_CANCELED -> Resolved.Problem("gone")
                row.calendarId != input.calendarId || row.originalId != input.eventId || row.originalTime != instanceTime ->
                    Resolved.Problem("gone")
                row.allDay || !row.rrule.isNullOrEmpty() || !row.rdate.isNullOrEmpty() ||
                    !row.exrule.isNullOrEmpty() || !row.exdate.isNullOrEmpty() || row.dtstart == null || row.dtend == null ->
                    Resolved.Problem("unsupported")
                row.title != input.title || row.organizer.orEmpty() != input.organizer.orEmpty() ||
                    row.timezone.orEmpty() != input.sourceTimezone.orEmpty() -> Resolved.Problem("changed")
                !editable(row, calendar) -> Resolved.Problem("readOnly")
                else -> Resolved.Found(Target.Row(id, row, Span(row.dtstart, row.dtend)))
            }
        }
        if (exceptions.size > 1) return Resolved.Problem("duplicate")
        exceptions.singleOrNull()?.let { return it }

        val instance = query(
            Instances.CONTENT_URI.buildUpon()
                .appendPath(instanceTime.toString())
                .appendPath((instanceTime + 1).toString())
                .build(),
            arrayOf(Instances.BEGIN, Instances.END),
            "${Instances.EVENT_ID} = ? AND ${Instances.BEGIN} = ?",
            arrayOf(input.eventId.toString(), instanceTime.toString()),
        ) { c -> Span(c.getLong(0), c.getLong(1)) }.firstOrNull() ?: return Resolved.Problem("gone")
        return Resolved.Found(Target.Untouched(input.eventId, instanceTime, event.status, instance))
    }

    private fun resolveSingle(eventId: Long, event: EventRow): Resolved {
        if (!event.rrule.isNullOrEmpty() || !event.rdate.isNullOrEmpty() || event.originalId != null) {
            return Resolved.Problem("gone")
        }
        if (event.status == Events.STATUS_CANCELED) return Resolved.Problem("gone")
        val start = event.dtstart ?: return Resolved.Problem("unsupported")
        val end = event.dtend ?: return Resolved.Problem("unsupported")
        return Resolved.Found(Target.Row(eventId, event, Span(start, end)))
    }

    /** The provider rejects DTEND here and turns DURATION into the exception's DTEND. */
    private fun insertException(target: Target.Untouched, to: Span) {
        val values = ContentValues().apply {
            put(Events.ORIGINAL_INSTANCE_TIME, target.instanceTime)
            put(Events.DTSTART, to.beginMs)
            put(Events.DURATION, "P${(to.endMs - to.beginMs) / 1000}S")
            put(Events.STATUS, target.status ?: Events.STATUS_CONFIRMED)
        }
        resolver.insert(ContentUris.withAppendedId(Events.CONTENT_EXCEPTION_URI, target.masterId), values)
            ?: throw IllegalStateException("The calendar provider refused the moved occurrence")
    }

    private fun updateRow(input: OccurrenceInput, target: Target.Row, to: Span): Boolean {
        val values = ContentValues().apply {
            put(Events.DTSTART, to.beginMs)
            put(Events.DTEND, to.endMs)
        }
        val row = target.event
        val occurrence = if (input.instanceTime == null) "${Events.ORIGINAL_ID} IS NULL AND ${Events.ORIGINAL_INSTANCE_TIME} IS NULL"
            else "${Events.ORIGINAL_ID} = ? AND ${Events.ORIGINAL_INSTANCE_TIME} = ?"
        val selection = "${Events._ID} = ? AND ${Events.CALENDAR_ID} = ? AND ${Events.DELETED} = 0 AND " +
            "(${Events.RRULE} IS NULL OR ${Events.RRULE} = '') AND (${Events.RDATE} IS NULL OR ${Events.RDATE} = '') AND " +
            "(${Events.EXRULE} IS NULL OR ${Events.EXRULE} = '') AND (${Events.EXDATE} IS NULL OR ${Events.EXDATE} = '') AND " +
            "${Events.ALL_DAY} = 0 AND (${Events.STATUS} IS NULL OR ${Events.STATUS} != ${Events.STATUS_CANCELED}) AND " +
            "${Events.DTSTART} = ? AND ${Events.DTEND} = ? AND ${Events.TITLE} IS ? AND ${Events.ORGANIZER} IS ? AND " +
            "${Events.GUESTS_CAN_MODIFY} IS ? AND $occurrence"
        val args = mutableListOf<String?>(target.id.toString(), input.calendarId.toString(),
            target.current.beginMs.toString(), target.current.endMs.toString(), row.title, row.organizer,
            row.guestsCanModify?.toString())
        if (input.instanceTime != null) {
            args.add(input.eventId.toString())
            args.add(input.instanceTime.toString())
        }
        return resolver.update(Events.CONTENT_URI, values, selection, args.toTypedArray()) == 1
    }

    private fun readBack(input: OccurrenceInput, to: Span): Map<String, Any?> =
        when (val after = resolve(input)) {
            is Resolved.Problem -> mapOf("status" to after.status)
            is Resolved.Found ->
                if (after.target is Target.Row && after.target.current == to) mapOf("status" to "moved")
                else spanResult("changed", after.target.current)
        }

    private fun spanResult(status: String, span: Span) =
        mapOf("status" to status, "beginMs" to span.beginMs, "endMs" to span.endMs)

    private fun readCalendar(calendarId: Long): CalendarRow? =
        query(
            ContentUris.withAppendedId(Calendars.CONTENT_URI, calendarId),
            arrayOf(Calendars.ACCOUNT_NAME, Calendars.CALENDAR_ACCESS_LEVEL, Calendars.OWNER_ACCOUNT, Calendars.ACCOUNT_TYPE),
            null,
            null,
        ) { c -> CalendarRow(c.stringOrNull(0), c.getInt(1), c.stringOrNull(2), c.stringOrNull(3)) }.firstOrNull()

    private fun readEvent(eventId: Long): EventRow? =
        query(
            ContentUris.withAppendedId(Events.CONTENT_URI, eventId),
            arrayOf(
                Events.CALENDAR_ID, Events.DELETED, Events.STATUS, Events.RRULE, Events.RDATE, Events.ORIGINAL_ID,
                Events.ALL_DAY, Events.DTSTART, Events.DTEND, Events._SYNC_ID, Events.TITLE, Events.ORGANIZER,
                Events.GUESTS_CAN_MODIFY, Events.ORIGINAL_INSTANCE_TIME, Events.EXRULE, Events.EXDATE, Events.DURATION,
                Events.EVENT_TIMEZONE,
            ),
            null,
            null,
        ) { c ->
            EventRow(
                calendarId = c.getLong(0),
                deleted = c.getInt(1) != 0,
                status = c.intOrNull(2),
                rrule = c.stringOrNull(3),
                rdate = c.stringOrNull(4),
                originalId = c.longOrNull(5),
                allDay = c.getInt(6) != 0,
                dtstart = c.longOrNull(7),
                dtend = c.longOrNull(8),
                syncId = c.stringOrNull(9),
                title = c.stringOrNull(10),
                organizer = c.stringOrNull(11),
                guestsCanModify = c.intOrNull(12),
                originalTime = c.longOrNull(13),
                exrule = c.stringOrNull(14),
                exdate = c.stringOrNull(15),
                duration = c.stringOrNull(16),
                timezone = c.stringOrNull(17),
            )
        }.firstOrNull()

    private fun editable(event: EventRow, calendar: CalendarRow): Boolean =
        event.organizer.isNullOrEmpty() || event.organizer == calendar.ownerAccount || event.guestsCanModify == 1

    private fun <T> query(
        uri: Uri,
        projection: Array<String>,
        selection: String?,
        args: Array<String>?,
        read: (Cursor) -> T,
    ): List<T> =
        resolver.query(uri, projection, selection, args, null)?.use { c ->
            buildList { while (c.moveToNext()) add(read(c)) }
        } ?: emptyList()

    private fun Cursor.stringOrNull(index: Int): String? = if (isNull(index)) null else getString(index)
    private fun Cursor.longOrNull(index: Int): Long? = if (isNull(index)) null else getLong(index)
    private fun Cursor.intOrNull(index: Int): Int? = if (isNull(index)) null else getInt(index)
}
