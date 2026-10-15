package expo.modules.workoutworker.utils

// A running rest as the notification shows it, in whole seconds since the rest timer started.
data class RestProgress(val elapsed: Long, val minRest: Long, val maxRest: Long) {
    val window: RestWindow get() = RestWindow.of(elapsed, minRest, maxRest)

    // What the pill and the subtext count down: first to the minimum rest, then to the maximum.
    val windowLeft: Long
        get() = when (window) {
            RestWindow.RESTING -> minRest - elapsed
            RestWindow.READY -> maxRest - elapsed
            RestWindow.DONE -> 0L
        }

    val windowLength: Long
        get() = when (window) {
            RestWindow.RESTING -> minRest
            RestWindow.READY -> maxRest - minRest
            RestWindow.DONE -> 0L
        }

    val overtime: Long get() = (elapsed - maxRest).coerceAtLeast(0L)

    // The bar drains, but a ProgressStyle only fills from its start. So it shows the time left, and
    // the minimum-rest mark is mirrored to the far end, the stretch that empties first.
    val barMax: Int get() = maxRest.toInt()
    val barProgress: Int get() = (maxRest - elapsed).coerceIn(0L, maxRest).toInt()
    val barMinRestMark: Int? get() = (maxRest - minRest).toInt().takeIf { it > 0 }
}
