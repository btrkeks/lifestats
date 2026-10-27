package com.opennutritracker.ont.opennutritracker

import android.content.Context
import android.util.Log
import androidx.annotation.MainThread
import com.opennutritracker.ont.opennutritracker.quicklog.QuickLogBridge
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.embedding.engine.FlutterEngineCache
import io.flutter.embedding.engine.dart.DartExecutor
import io.flutter.plugin.common.MethodChannel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first

/**
 * The one Flutter engine in this process that runs the app's `main()`, and so
 * the only isolate that ever opens the Hive boxes.
 *
 * hive_ce keeps its cache, key allocator and write offset per isolate, and
 * every engine runs its own root isolate, so a second engine running `main()`
 * would be a second writer on the same files. MainActivity attaches to this
 * engine by its [CACHE_ID] instead of creating its own, and code that
 * runs with no activity reaches Dart through it too. The manifest declares no
 * android:process, so every component shares this object.
 *
 * Never destroyed: Dart outlives MainActivity on purpose, and destroying the
 * engine while a write is in flight would drop it. The OS reclaims it with
 * the process.
 */
object OntEngine {
    private const val TAG = "OntEngine"
    private const val CHANNEL = "com.opennutritracker/engine"

    /** The engine's id in FlutterEngineCache, which is how activities attach to it. */
    const val CACHE_ID = "ont_engine"

    sealed interface Readiness {
        /** `main()` has not finished opening storage yet. */
        data object Booting : Readiness

        /** Storage is open: Dart can take commands. */
        data object Ready : Readiness

        /** Storage failed to open; the user is on the recovery screen. */
        data class Unavailable(val reason: String) : Readiness
    }

    private var engine: FlutterEngine? = null
    private val readiness = MutableStateFlow<Readiness>(Readiness.Booting)

    /**
     * Returns the running engine, creating it and starting `main()` on first
     * use. Main thread only, which FlutterEngine construction requires anyway
     * and which makes creation race-free without a lock.
     */
    @MainThread
    fun obtain(context: Context): FlutterEngine {
        engine?.let { return it }
        val app = context.applicationContext
        val created = FlutterEngine(app)
        // Before Dart runs: a boot with no activity attached still calls the
        // locale, launch and quick-log channels from main().
        LocaleChannel.register(app, created)
        LaunchInbox.register(created)
        QuickLogBridge.register(app, created)
        registerReadiness(created)
        created.dartExecutor.executeDartEntrypoint(DartExecutor.DartEntrypoint.createDefault())
        engine = created
        FlutterEngineCache.getInstance().put(CACHE_ID, created)
        Log.i(TAG, "engine created id=${System.identityHashCode(created)}")
        return created
    }

    /** True once [obtain] has run in this process. For diagnostics only. */
    val isRunning: Boolean get() = engine != null

    /**
     * Suspends until Dart reports that storage opened or failed to open.
     * Returns at once when it already has. Callers bound the wait themselves.
     */
    suspend fun awaitReadiness(): Readiness = readiness.first { it != Readiness.Booting }

    private fun registerReadiness(engine: FlutterEngine) {
        MethodChannel(engine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "ready" -> {
                        readiness.value = Readiness.Ready
                        Log.i(TAG, "dart ready id=${System.identityHashCode(engine)}")
                        result.success(null)
                    }
                    // Not terminal: the recovery screen's retry re-runs the
                    // bootstrap and reports ready when it succeeds.
                    "unavailable" -> {
                        val reason = call.arguments as? String ?: "unknown"
                        readiness.value = Readiness.Unavailable(reason)
                        Log.w(TAG, "dart unavailable reason=$reason")
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            }
    }
}
