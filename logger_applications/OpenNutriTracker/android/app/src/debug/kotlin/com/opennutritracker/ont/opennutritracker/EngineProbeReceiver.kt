package com.opennutritracker.ont.opennutritracker

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Process
import android.os.SystemClock
import android.util.Log
import io.flutter.plugin.common.MethodChannel
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.coroutines.resume

/**
 * Debug builds only: drives one intake write through [OntEngine] with no
 * activity, so the shared-engine design can be checked on a device before
 * the widget exists.
 *
 *   adb shell am broadcast -n <pkg>/com.opennutritracker.ont.opennutritracker.EngineProbeReceiver \
 *       [--el delay_ms 3000] [--es find_id <intakeId>]
 *
 * `delay_ms` holds the write in Dart that long, so the activity can be
 * detached and reattached while a command is in flight. `find_id` writes
 * nothing and looks that intake up instead. Results go to logcat
 * under the EngineProbe tag. Remove once the widget's own actions exercise
 * this path (phase 2, U8).
 */
class EngineProbeReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val pending = goAsync()
        val triggeredAt = System.currentTimeMillis()
        val processAgeMs = SystemClock.uptimeMillis() - Process.getStartUptimeMillis()
        val wasRunning = OntEngine.isRunning
        val engine = OntEngine.obtain(context)
        Log.i(
            TAG,
            "trigger at=$triggeredAt processAgeMs=$processAgeMs engineWasRunning=$wasRunning " +
                "engine=${System.identityHashCode(engine)}",
        )
        scope.launch {
            try {
                val readiness = withTimeoutOrNull(TIMEOUT_MS) { OntEngine.awaitReadiness() }
                val readyAt = System.currentTimeMillis()
                Log.i(TAG, "readiness=$readiness at=$readyAt (+${readyAt - triggeredAt} ms)")
                if (readiness != OntEngine.Readiness.Ready) return@launch
                val reply = withTimeoutOrNull(TIMEOUT_MS) {
                    suspendCancellableCoroutine { cont ->
                        val findId = intent.getStringExtra("find_id")
                        MethodChannel(engine.dartExecutor.binaryMessenger, CHANNEL).invokeMethod(
                            if (findId == null) "logIntake" else "findIntake",
                            mapOf("delayMs" to intent.getLongExtra("delay_ms", 0), "id" to findId),
                            object : MethodChannel.Result {
                                override fun success(result: Any?) = cont.resume("ok $result")
                                override fun error(code: String, message: String?, details: Any?) =
                                    cont.resume("error $code $message")
                                override fun notImplemented() = cont.resume("notImplemented")
                            },
                        )
                    }
                }
                val committedAt = System.currentTimeMillis()
                Log.i(
                    TAG,
                    "commit at=$committedAt (+${committedAt - triggeredAt} ms, " +
                        "+${committedAt - readyAt} ms after ready) reply=${reply ?: "timeout"}",
                )
            } finally {
                pending.finish()
            }
        }
    }

    private companion object {
        const val TAG = "EngineProbe"
        const val CHANNEL = "com.opennutritracker/debug_probe"
        const val TIMEOUT_MS = 20_000L
        val scope = MainScope()
    }
}
