package expo.modules.workoutworker.utils

import org.junit.Assert.assertEquals
import org.junit.Test

class RestCountdownTest {
    @Test
    fun crossingThreeSecondsLeft() {
        assertEquals(listOf(3), countdownSecondsCrossed(4000, 2900))
    }

    @Test
    fun aStalledTickCrossesEveryMarkItSkipped() {
        assertEquals(listOf(3, 2, 1), countdownSecondsCrossed(3100, 900))
    }

    @Test
    fun landingExactlyOnAMarkCrossesItOnce() {
        assertEquals(listOf(2), countdownSecondsCrossed(2400, 2000))
        assertEquals(emptyList<Int>(), countdownSecondsCrossed(2000, 1600))
    }

    @Test
    fun noTimePassingCrossesNothing() {
        assertEquals(emptyList<Int>(), countdownSecondsCrossed(2000, 2000))
        assertEquals(listOf(2), countdownSecondsCrossed(2001, 2000))
    }

    @Test
    fun restRunningOutIsTheLastMark() {
        assertEquals(listOf(0), countdownSecondsCrossed(500, -200))
        assertEquals(listOf(1, 0), countdownSecondsCrossed(1100, -200))
    }

    @Test
    fun aRestTimerThatIsAlreadyOverCrossesNothing() {
        assertEquals(emptyList<Int>(), countdownSecondsCrossed(-200, -700))
        assertEquals(emptyList<Int>(), countdownSecondsCrossed(0, -500))
    }

    @Test
    fun aTimerStartedWithUnderThreeSecondsLeftOnlyCrossesTheMarksAhead() {
        assertEquals(emptyList<Int>(), countdownSecondsCrossed(2500, 2500))
        assertEquals(listOf(2), countdownSecondsCrossed(2500, 2000))
    }
}
