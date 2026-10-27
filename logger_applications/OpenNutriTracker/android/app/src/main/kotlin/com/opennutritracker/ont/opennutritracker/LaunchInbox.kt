package com.opennutritracker.ont.opennutritracker

import android.content.Intent
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

/**
 * "The OS opened us to do X", held until Dart asks for it.
 *
 * A slot that Dart pulls, rather than a call pushed at Dart, because the
 * intent is known in onCreate while the widget tree that has to react to it
 * is not ready until later. Pushing would mean racing engine startup for the
 * launch that most needs to work.
 *
 * Process-scoped rather than a field on MainActivity: the engine outlives the
 * activity, so a field on it would be read through a dead activity. One slot,
 * last writer wins: two launches before Dart reads mean the user only wants
 * the latest.
 */
object LaunchInbox {
    private const val CHANNEL = "com.opennutritracker/launch"

    /**
     * Both Health Connect actions mean the same thing to a user: explain the
     * health import. ACTION_SHOW_PERMISSIONS_RATIONALE comes from the grant
     * dialog, VIEW_PERMISSION_USAGE from Health Connect's own settings for an
     * app that already holds permissions. Matched as literals rather than
     * through the Health Connect client's constants, so this file can be read
     * against the manifest side by side.
     */
    private const val ACTION_SHOW_PERMISSIONS_RATIONALE = "androidx.health.ACTION_SHOW_PERMISSIONS_RATIONALE"
    private const val ACTION_VIEW_PERMISSION_USAGE = "android.intent.action.VIEW_PERMISSION_USAGE"

    /** Wire values match Dart's `LaunchRequest` enum names. */
    enum class Request(val wire: String) {
        HealthRationale("healthRationale"),
    }

    private var pending: Request? = null

    /** Records what [intent] asks for; leaves the slot alone for any other intent. */
    fun offer(intent: Intent?) {
        requestFor(intent)?.let { pending = it }
    }

    fun register(engine: FlutterEngine) {
        MethodChannel(engine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    // Reading clears the slot: the request is a one-shot
                    // event, and leaving it set would reopen the screen on
                    // every resume for the rest of the session.
                    "consume" -> {
                        result.success(pending?.wire)
                        pending = null
                    }
                    else -> result.notImplemented()
                }
            }
    }

    private fun requestFor(intent: Intent?): Request? =
        when (intent?.action) {
            ACTION_SHOW_PERMISSIONS_RATIONALE, ACTION_VIEW_PERMISSION_USAGE -> Request.HealthRationale
            else -> null
        }
}
