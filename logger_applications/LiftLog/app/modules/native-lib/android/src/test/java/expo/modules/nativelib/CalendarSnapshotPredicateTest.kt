package expo.modules.nativelib

import org.junit.Assert.assertEquals
import org.junit.Test

class CalendarSnapshotPredicateTest {
    @Test fun nullOrganizerAndGuestFlagNeedNoBindArguments() {
        val predicate = calendarSnapshotPredicate("Gym", null, null)
        assertEquals("title IS ? AND organizer IS NULL AND guestsCanModify IS NULL", predicate.selection)
        assertEquals(listOf("Gym"), predicate.args)
        assertEquals(predicate.args.size, predicate.selection.count { it == '?' })
    }

    @Test fun nonNullSnapshotsKeepTheirBindOrderAndValues() {
        val predicate = calendarSnapshotPredicate("Gym", "me@example.com", 1)
        assertEquals("title IS ? AND organizer IS ? AND guestsCanModify IS ?", predicate.selection)
        assertEquals(listOf("Gym", "me@example.com", "1"), predicate.args)
    }

    @Test fun nullSnapshotsDoNotShiftTheRemainingArguments() {
        val organizer = calendarSnapshotPredicate("Gym", null, 0)
        assertEquals("title IS ? AND organizer IS NULL AND guestsCanModify IS ?", organizer.selection)
        assertEquals(listOf("Gym", "0"), organizer.args)
        val guestFlag = calendarSnapshotPredicate("Gym", "me@example.com", null)
        assertEquals("title IS ? AND organizer IS ? AND guestsCanModify IS NULL", guestFlag.selection)
        assertEquals(listOf("Gym", "me@example.com"), guestFlag.args)
    }

    @Test fun allNullSnapshotsHaveNoPlaceholdersOrArguments() {
        val predicate = calendarSnapshotPredicate(null, null, null)
        assertEquals("title IS NULL AND organizer IS NULL AND guestsCanModify IS NULL", predicate.selection)
        assertEquals(emptyList<String>(), predicate.args)
    }
}
