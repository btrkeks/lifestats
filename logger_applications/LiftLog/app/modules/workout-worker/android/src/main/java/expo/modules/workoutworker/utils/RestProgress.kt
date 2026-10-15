package expo.modules.workoutworker.utils

data class RestProgress(val elapsedSecs: Long, val minRestSecs: Long, val maxRestSecs: Long) {
    val window: RestWindow get() = RestWindow.of(elapsedSecs, minRestSecs, maxRestSecs)

    val windowLeftSecs: Long
        get() = when (window) {
            RestWindow.RESTING -> minRestSecs - elapsedSecs
            RestWindow.READY -> maxRestSecs - elapsedSecs
            RestWindow.DONE -> 0L
        }

    val windowLengthSecs: Long
        get() = when (window) {
            RestWindow.RESTING -> minRestSecs
            RestWindow.READY -> maxRestSecs - minRestSecs
            RestWindow.DONE -> 0L
        }

    val overtimeSecs: Long get() = (elapsedSecs - maxRestSecs).coerceAtLeast(0L)

    // The bar drains, but a ProgressStyle only fills from its start. So it shows the time left, and
    // the minimum-rest mark is mirrored to the far end, the stretch that empties first.
    val barMax: Int get() = maxRestSecs.toInt()
    val barProgress: Int get() = (maxRestSecs - elapsedSecs).coerceIn(0L, maxRestSecs).toInt()
    val barMinRestMark: Int? get() = (maxRestSecs - minRestSecs).toInt().takeIf { it > 0 }
}
