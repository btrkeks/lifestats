package com.opennutritracker.ont.opennutritracker.quicklog

import android.content.Context
import android.content.Intent
import com.opennutritracker.ont.opennutritracker.quicklog.widget.logFood
import io.flutter.embedding.android.FlutterActivityLaunchConfigs.BackgroundMode
import io.flutter.embedding.android.FlutterFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

/**
 * The amount sheet over the launcher (s6, s7), opened by a tap on a row's
 * body.
 *
 * Runs `quickLogPickerMain` in an engine of its own, created and destroyed
 * with this activity. That isolate never opens Hive: it gets the row's
 * picker request from the published snapshot, and Add goes through
 * [logFood] to the process engine, exactly like a pill.
 *
 * The manifest gives it its own task (taskAffinity="") kept out of recents,
 * so the app's task, splash and back stack never come forward over the
 * launcher.
 */
class QuickLogPickerActivity : FlutterFragmentActivity() {

    override fun getDartEntrypointFunctionName(): String = "quickLogPickerMain"

    override fun getDartEntrypointLibraryUri(): String = "package:opennutritracker/main.dart"

    override fun getBackgroundMode(): BackgroundMode = BackgroundMode.transparent

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        val foodKey = intent.getStringExtra(EXTRA_FOOD_KEY)
        val profileId = intent.getStringExtra(EXTRA_PROFILE_ID)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "request" -> result.success(row(foodKey, profileId)?.pickerJson)
                    "submit" -> {
                        val baseAmount = call.argument<Number>("baseAmount")?.toDouble()
                        val unit = call.argument<String>("unit")
                        if (foodKey == null || profileId == null || baseAmount == null || unit == null) {
                            result.error("invalid_arguments", "submit needs baseAmount and unit", null)
                            return@setMethodCallHandler
                        }
                        // Outlives the activity: a write already sent must
                        // still show Added if the user leaves meanwhile.
                        submits.launch {
                            val reply = logFood(applicationContext, foodKey, profileId, baseAmount, unit)
                            // Unavailable is worth a retry; anything else is done.
                            val closing = reply !is QuickLogReply.Unavailable
                            result.success(closing)
                            if (closing) finish()
                        }
                    }
                    "dismiss" -> {
                        result.success(null)
                        finish()
                    }
                    else -> result.notImplemented()
                }
            }
    }

    /**
     * A sheet the user left is closed, not kept in the background: a later
     * row tap then opens a fresh one for that row.
     */
    override fun onStop() {
        super.onStop()
        if (!isChangingConfigurations && !isFinishing) finish()
    }

    private fun row(foodKey: String?, profileId: String?): Row? {
        val model = QuickLogStore.current(this)
        val snapshot = model.snapshot as? WidgetSnapshot.Ready ?: return null
        if (foodKey == null || snapshot.profileId != profileId) return null
        return model.row(foodKey)
    }

    companion object {
        private const val CHANNEL = "com.opennutritracker/quick_log_picker"
        private const val EXTRA_FOOD_KEY = "foodKey"
        private const val EXTRA_PROFILE_ID = "profileId"

        private val submits = CoroutineScope(SupervisorJob() + Dispatchers.Main)

        fun intentFor(context: Context, foodKey: String, profileId: String): Intent =
            Intent(context, QuickLogPickerActivity::class.java)
                .putExtra(EXTRA_FOOD_KEY, foodKey)
                .putExtra(EXTRA_PROFILE_ID, profileId)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    }
}
