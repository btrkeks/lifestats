package expo.modules.workoutworker.utils

import org.junit.Assert.assertEquals
import org.junit.Test

class RestProgressTest {
    private fun rest(elapsed: Long) = RestProgress(elapsed, minRest = 60, maxRest = 90)

    @Test
    fun theBarStartsFullWithTheMinimumRestMarkedFromTheFarEnd() {
        assertEquals(90, rest(0).barProgress)
        assertEquals(30, rest(0).barMinRestMark)
    }

    @Test
    fun theBarDrainsAsRestRuns() {
        assertEquals(60, rest(30).barProgress)
        assertEquals(30, rest(60).barProgress)
        assertEquals(15, rest(75).barProgress)
    }

    @Test
    fun theBarIsEmptyPastTheMaximumRest() {
        assertEquals(0, rest(100).barProgress)
    }

    @Test
    fun aFixedRestHasNoMark() {
        assertEquals(null, RestProgress(0, minRest = 60, maxRest = 60).barMinRestMark)
    }

    @Test
    fun theCountdownIsTheTimeLeftInTheCurrentWindow() {
        assertEquals(45L to 60L, rest(15).let { it.windowLeft to it.windowLength })
        assertEquals(15L to 30L, rest(75).let { it.windowLeft to it.windowLength })
        assertEquals(0L to 0L, rest(100).let { it.windowLeft to it.windowLength })
    }

    @Test
    fun overtimeCountsUpPastTheMaximumRest() {
        assertEquals(0L, rest(75).overtime)
        assertEquals(10L, rest(100).overtime)
    }
}
