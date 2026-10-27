package com.opennutritracker.ont.opennutritracker

import android.content.Intent
import android.os.Bundle
import io.flutter.embedding.android.FlutterFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

// FlutterFragmentActivity (rather than FlutterActivity) is required by the
// health plugin: Health Connect permission requests go through
// registerForActivityResult, which needs a ComponentActivity host.
class MainActivity : FlutterFragmentActivity() {
    private val healthConnectChannelName = "com.opennutritracker/health_connect"

    /**
     * Scope for the Health Connect reads, which are suspend functions.
     *
     * Main dispatcher because a MethodChannel result has to be posted from the
     * platform thread; the client's own reads move themselves off it. A
     * SupervisorJob so one failed import cannot cancel the scope and leave
     * every later read dead, and cancelled in [onDestroy] so no read outlives
     * the activity it was started through.
     */
    private val healthConnectScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    override fun onCreate(savedInstanceState: Bundle?) {
        // Before super: super creates the fragment, which looks the engine up
        // by [getCachedEngineId]. The engine may already be running and
        // reading the inbox on resume.
        OntEngine.obtain(this)
        // A restored activity carries the intent that first created its task,
        // which is not a new request: re-offering it reopened the health
        // screen every time the app came back after process death.
        if (savedInstanceState == null) LaunchInbox.offer(intent)
        super.onCreate(savedInstanceState)
    }

    /**
     * Attaches to the process engine instead of creating one. A cached engine
     * rather than provideFlutterEngine: FlutterFragment.withNewEngine always
     * destroys the engine with the fragment, even one the host provided,
     * which killed Dart when the user backed out and crashed the next launch.
     */
    override fun getCachedEngineId(): String = OntEngine.CACHE_ID

    /**
     * Dart outlives this activity: the engine may still be needed after the
     * user backs out. Fixed rather than read from the launching intent's
     * extra, which any app could set since this activity is exported.
     */
    override fun shouldDestroyEngineWithHost(): Boolean = false

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        // Finished workouts, read straight from Health Connect rather than
        // through the health plugin. See [HealthConnectWorkoutReader] for why:
        // in short, the plugin's workout read also reads distance and steps,
        // and Play's Health Connect permissions policy refused those two
        // permissions for this app.
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, healthConnectChannelName)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "readExerciseSessions" -> {
                        val from = call.argument<Number>("fromMillis")?.toLong()
                        val to = call.argument<Number>("toMillis")?.toLong()
                        if (from == null || to == null) {
                            result.error(
                                "invalid_arguments",
                                "readExerciseSessions needs fromMillis and toMillis",
                                null,
                            )
                            return@setMethodCallHandler
                        }
                        healthConnectScope.launch {
                            try {
                                result.success(
                                    HealthConnectWorkoutReader.readExerciseSessions(
                                        applicationContext,
                                        from,
                                        to,
                                    ),
                                )
                            } catch (e: CancellationException) {
                                // The activity went away mid-read and
                                // [onDestroy] cancelled the scope. The engine
                                // outlives the activity, so Dart is still
                                // awaiting this read: answer it as failed, then
                                // rethrow so the coroutine actually ends.
                                result.error("health_connect_unavailable", "cancelled", null)
                                throw e
                            } catch (e: SecurityException) {
                                // A revoked grant. Distinguished from the rest
                                // so Dart can tell the user to re-grant instead
                                // of reporting a generic failure.
                                result.error("permission_denied", e.message, null)
                            } catch (e: Exception) {
                                // Health Connect uninstalled or updating, a
                                // client that will not initialise, an IO fault.
                                result.error("health_connect_unavailable", e.message, null)
                            }
                        }
                    }
                    else -> result.notImplemented()
                }
            }
    }

    /**
     * The engine survives this activity, and the handler above captures it
     * through [healthConnectScope] and [applicationContext]. Left in place it
     * would answer through a destroyed activity.
     */
    override fun cleanUpFlutterEngine(flutterEngine: FlutterEngine) {
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, healthConnectChannelName)
            .setMethodCallHandler(null)
        super.cleanUpFlutterEngine(flutterEngine)
    }

    /**
     * The activity is `launchMode="singleTop"`, so Health Connect reaching a
     * process that is already running delivers the intent here instead of
     * starting a new activity. Reading only the launching intent would work
     * exactly once per process and then quietly stop.
     */
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        LaunchInbox.offer(intent)
    }

    override fun onDestroy() {
        healthConnectScope.cancel()
        super.onDestroy()
    }
}
