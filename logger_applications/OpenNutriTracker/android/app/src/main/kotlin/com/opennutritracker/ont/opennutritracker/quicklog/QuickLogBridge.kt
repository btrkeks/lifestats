package com.opennutritracker.ont.opennutritracker.quicklog

import android.content.Context
import android.util.Log
import com.opennutritracker.ont.opennutritracker.OntEngine
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.coroutines.resume

/** What native code asks Dart to do. Mirrors Dart's `QuickLogCommand`. */
sealed interface QuickLogCommand {
    /** [opId] becomes the intake id: replaying the same log writes nothing new. */
    data class Log(
        val opId: String,
        val foodKey: String,
        val profileId: String,
        val baseAmount: Double,
        val unit: String,
        val tappedAtMs: Long,
    ) : QuickLogCommand

    data class Undo(val opId: String, val profileId: String) : QuickLogCommand

    data object Refresh : QuickLogCommand
}

sealed interface QuickLogReply {
    /** [addedText] is "Added · +30 g" for a log, null otherwise. */
    data class Ok(val addedText: String?) : QuickLogReply

    /** Dart understood and refused: stale profile, unknown food, invalid, not set up. */
    data class Rejected(val reason: String) : QuickLogReply

    /** Dart did not answer in time, storage failed to open, or the channel errored. */
    data class Unavailable(val cause: String) : QuickLogReply
}

/**
 * Kotlin end of `com.opennutritracker/quick_log` on [OntEngine]'s engine, and
 * the one way native code reaches Dart's storage.
 *
 *   Kotlin -> Dart   run(Map command) -> Map result
 *   Dart -> Kotlin   publish(String json)
 */
object QuickLogBridge {
    private const val TAG = "QuickLogBridge"
    private const val CHANNEL = "com.opennutritracker/quick_log"

    /**
     * Covers a cold boot, which runs all of `main()` up to storage, and stays
     * under the background-broadcast budget a widget action runs in.
     */
    const val SEND_TIMEOUT_MS = 12_000L

    private var channel: MethodChannel? = null

    /** Called once by [OntEngine] before Dart starts. */
    fun register(appContext: Context, engine: FlutterEngine) {
        channel = MethodChannel(engine.dartExecutor.binaryMessenger, CHANNEL).apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "publish" -> {
                        val json = call.arguments as? String
                        if (json == null) {
                            result.error("invalid_arguments", "publish needs a JSON string", null)
                        } else {
                            result.success(QuickLogStore.publish(appContext, json))
                        }
                    }
                    "isPlaced" -> result.success(QuickLogPresence.isPlaced(appContext))
                    else -> result.notImplemented()
                }
            }
        }
    }

    fun presenceChanged() {
        channel?.invokeMethod("presenceChanged", null)
    }

    /**
     * Starts the process engine if needed, waits until Dart has opened
     * storage, and runs [command]. Never throws.
     *
     * A timeout can land after Dart committed. The write is still published,
     * so an uncertain caller retries the same command and operation id.
     */
    suspend fun send(context: Context, command: QuickLogCommand): QuickLogReply {
        val startedAt = System.currentTimeMillis()
        val reply = withTimeoutOrNull(SEND_TIMEOUT_MS) {
            withContext(Dispatchers.Main) { OntEngine.obtain(context) }
            when (val readiness = OntEngine.awaitReadiness()) {
                is OntEngine.Readiness.Unavailable -> QuickLogReply.Unavailable(readiness.reason)
                else -> withContext(Dispatchers.Main) { runInDart(command) }
            }
        } ?: QuickLogReply.Unavailable("timeout")
        Log.i(TAG, "${command.op} -> $reply in ${System.currentTimeMillis() - startedAt} ms")
        return reply
    }

    private suspend fun runInDart(command: QuickLogCommand): QuickLogReply =
        suspendCancellableCoroutine { continuation ->
            val channel = channel ?: run {
                continuation.resume(QuickLogReply.Unavailable("no_channel"))
                return@suspendCancellableCoroutine
            }
            channel.invokeMethod("run", command.toWire(), object : MethodChannel.Result {
                override fun success(result: Any?) {
                    continuation.resume(parseReply(result))
                }

                override fun error(code: String, message: String?, details: Any?) {
                    continuation.resume(QuickLogReply.Unavailable("$code: $message"))
                }

                override fun notImplemented() {
                    continuation.resume(QuickLogReply.Unavailable("not_implemented"))
                }
            })
        }

    private fun parseReply(result: Any?): QuickLogReply {
        val map = result as? Map<*, *> ?: return QuickLogReply.Unavailable("malformed_reply")
        return when (map["status"]) {
            "ok" -> QuickLogReply.Ok(map["addedText"] as? String)
            "rejected" -> QuickLogReply.Rejected(map["reason"] as? String ?: "unknown")
            else -> QuickLogReply.Unavailable("malformed_reply")
        }
    }
}

private val QuickLogCommand.op: String
    get() = when (this) {
        is QuickLogCommand.Log -> "log"
        is QuickLogCommand.Undo -> "undo"
        QuickLogCommand.Refresh -> "refresh"
    }

internal fun QuickLogCommand.toWire(): Map<String, Any> = when (this) {
    is QuickLogCommand.Log -> mapOf(
        "op" to op,
        "opId" to opId,
        "foodKey" to foodKey,
        "profileId" to profileId,
        "baseAmount" to baseAmount,
        "unit" to unit,
        "tappedAtMs" to tappedAtMs,
    )
    is QuickLogCommand.Undo -> mapOf("op" to op, "opId" to opId, "profileId" to profileId)
    QuickLogCommand.Refresh -> mapOf("op" to op)
}
