package expo.modules.workoutworker.utils

import org.junit.Assert.assertEquals
import org.junit.Test

class RestProgressTest {
    private fun rest(elapsed: Long) = RestProgress(elapsed, minRestSecs = 60, maxRestSecs = 90)

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
        assertEquals(null, RestProgress(0, minRestSecs = 60, maxRestSecs = 60).barMinRestMark)
    }

    @Test
    fun theCountdownIsTheTimeLeftInTheCurrentWindow() {
        assertEquals(45L to 60L, rest(15).let { it.windowLeftSecs to it.windowLengthSecs })
        assertEquals(15L to 30L, rest(75).let { it.windowLeftSecs to it.windowLengthSecs })
        assertEquals(0L to 0L, rest(100).let { it.windowLeftSecs to it.windowLengthSecs })
    }

    @Test
    fun overtimeCountsUpPastTheMaximumRest() {
        assertEquals(0L, rest(75).overtimeSecs)
        assertEquals(10L, rest(100).overtimeSecs)
    }
}
