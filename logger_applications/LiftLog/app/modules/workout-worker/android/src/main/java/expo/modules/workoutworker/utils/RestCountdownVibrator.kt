package expo.modules.workoutworker.utils

import android.content.Context
import android.media.AudioAttributes
import android.os.Build
import android.os.VibrationAttributes
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager

private val COUNTDOWN_SECONDS = listOf(3, 2, 1, 0)

// The seconds-left marks the rest countdown passed between two ticks, given the milliseconds left
// until rest is over at each. Zero is a mark too: on silent the "rest over" notification does not
// vibrate, so the countdown has to end with its own buzz.
fun countdownSecondsCrossed(previousRemainingMs: Long, remainingMs: Long): List<Int> =
    COUNTDOWN_SECONDS.filter { previousRemainingMs > it * 1000L && remainingMs <= it * 1000L }

class RestCountdownVibrator(context: Context) {
    private val vibrator: Vibrator =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            (context.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as VibratorManager).defaultVibrator
        } else {
            @Suppress("DEPRECATION")
            context.getSystemService(Context.VIBRATOR_SERVICE) as Vibrator
        }

    // Alarm usage so the pulse still comes through with the phone on silent, like a timer would.
    fun pulse(durationMs: Long) {
        val audioAttributes = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_ALARM).build()
        when {
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU -> vibrator.vibrate(
                VibrationEffect.createOneShot(durationMs, VibrationEffect.DEFAULT_AMPLITUDE),
                VibrationAttributes.createForUsage(VibrationAttributes.USAGE_ALARM),
            )

            Build.VERSION.SDK_INT >= Build.VERSION_CODES.O -> @Suppress("DEPRECATION") vibrator.vibrate(
                VibrationEffect.createOneShot(durationMs, VibrationEffect.DEFAULT_AMPLITUDE),
                audioAttributes,
            )

            else -> @Suppress("DEPRECATION") vibrator.vibrate(durationMs, audioAttributes)
        }
    }
}
