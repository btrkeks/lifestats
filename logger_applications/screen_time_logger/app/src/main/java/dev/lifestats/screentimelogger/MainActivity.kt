package dev.lifestats.screentimelogger

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Typeface
import android.net.Uri
import android.os.Bundle
import android.provider.Settings
import android.text.InputType
import android.view.View
import android.view.WindowInsets
import android.widget.Button
import android.widget.CheckBox
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import java.util.Locale

class MainActivity : Activity() {
    private lateinit var baseUrlInput: EditText
    private lateinit var totalStatisticIdInput: EditText
    private lateinit var goalStatisticIdInput: EditText
    private lateinit var usageAccessStatus: TextView
    private lateinit var goalExemptAppsStatus: TextView
    private lateinit var goalExemptAppsContainer: LinearLayout
    private lateinit var saveStatus: TextView

    private val settingsRepository by lazy { SettingsRepository(this) }
    private val usageAccessChecker by lazy { AndroidUsageAccessChecker(this) }
    private val selectedGoalExemptPackages = mutableSetOf<String>()
    private var recentAppsLoading = false
    private var recentAppsLoaded = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(createContentView())
        populateSavedSettings()
    }

    override fun onResume() {
        super.onResume()
        val hasUsageAccess = usageAccessChecker.hasAccess()
        usageAccessStatus.setText(
            if (hasUsageAccess) {
                R.string.usage_access_granted
            } else {
                R.string.usage_access_missing
            },
        )
        if (hasUsageAccess) {
            loadRecentAppsIfNeeded()
        } else {
            goalExemptAppsStatus.setText(R.string.goal_exempt_apps_require_access)
        }
    }

    private fun createContentView(): View {
        val content = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            val spacing = dp(24)
            setPadding(spacing, spacing, spacing, spacing)
        }
        applySystemBarInsets(content)

        content.addView(TextView(this).apply {
            setText(R.string.screen_title)
            textSize = 28f
            setTypeface(typeface, Typeface.BOLD)
        })
        content.addView(TextView(this).apply {
            setText(R.string.screen_description)
            textSize = 16f
            setPadding(0, dp(8), 0, dp(24))
        })

        content.addView(fieldLabel(R.string.base_url_label))
        baseUrlInput = EditText(this).apply {
            hint = getString(R.string.base_url_hint)
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
            setSingleLine(true)
        }
        content.addView(baseUrlInput, matchWidth())

        content.addView(fieldLabel(R.string.total_statistic_id_label).apply {
            setPadding(0, dp(16), 0, 0)
        })
        totalStatisticIdInput = EditText(this).apply {
            hint = getString(R.string.statistic_id_hint)
            inputType = InputType.TYPE_CLASS_NUMBER
            setSingleLine(true)
        }
        content.addView(totalStatisticIdInput, matchWidth())

        content.addView(fieldLabel(R.string.goal_statistic_id_label).apply {
            setPadding(0, dp(16), 0, 0)
        })
        goalStatisticIdInput = EditText(this).apply {
            hint = getString(R.string.goal_statistic_id_hint)
            inputType = InputType.TYPE_CLASS_NUMBER
            setSingleLine(true)
        }
        content.addView(goalStatisticIdInput, matchWidth())
        content.addView(TextView(this).apply {
            setText(R.string.goal_statistic_description)
            textSize = 14f
            setPadding(0, dp(6), 0, 0)
        })

        usageAccessStatus = TextView(this).apply {
            textSize = 16f
            setPadding(0, dp(24), 0, dp(8))
        }
        content.addView(usageAccessStatus)

        content.addView(Button(this).apply {
            setText(R.string.open_usage_access)
            setOnClickListener { openUsageAccessSettings() }
        }, matchWidth())

        content.addView(fieldLabel(R.string.goal_exempt_apps_label).apply {
            setPadding(0, dp(24), 0, 0)
        })
        content.addView(TextView(this).apply {
            setText(R.string.goal_exempt_apps_description)
            textSize = 14f
            setPadding(0, dp(6), 0, dp(8))
        })
        content.addView(Button(this).apply {
            setText(R.string.refresh_recent_apps)
            setOnClickListener {
                recentAppsLoaded = false
                if (usageAccessChecker.hasAccess()) {
                    loadRecentAppsIfNeeded()
                } else {
                    goalExemptAppsStatus.setText(R.string.goal_exempt_apps_require_access)
                }
            }
        }, matchWidth())

        goalExemptAppsStatus = TextView(this).apply {
            textSize = 14f
            setPadding(0, dp(8), 0, dp(4))
        }
        content.addView(goalExemptAppsStatus)

        goalExemptAppsContainer = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
        }
        content.addView(goalExemptAppsContainer, matchWidth())

        content.addView(Button(this).apply {
            setText(R.string.save_and_sync)
            setOnClickListener { saveAndSync() }
        }, matchWidth(topMargin = dp(12)))

        saveStatus = TextView(this).apply {
            textSize = 15f
            setPadding(0, dp(12), 0, 0)
        }
        content.addView(saveStatus)

        return ScrollView(this).apply { addView(content) }
    }

    private fun populateSavedSettings() {
        settingsRepository.loadOrNull()?.let { settings ->
            baseUrlInput.setText(settings.baseUrl)
            totalStatisticIdInput.setText(
                String.format(Locale.ROOT, "%d", settings.totalStatisticId),
            )
            settings.goalStatisticId?.let {
                goalStatisticIdInput.setText(String.format(Locale.ROOT, "%d", it))
            }
            selectedGoalExemptPackages += settings.goalExemptPackages
            renderAppChoices(settings.goalExemptPackages.map { AppChoice(it, it) })
        }
    }

    private fun saveAndSync() {
        val settings = try {
            SettingsValidator.parse(
                baseUrlInput = baseUrlInput.text.toString(),
                totalStatisticIdInput = totalStatisticIdInput.text.toString(),
                goalStatisticIdInput = goalStatisticIdInput.text.toString(),
                goalExemptPackages = selectedGoalExemptPackages,
            )
        } catch (error: IllegalArgumentException) {
            saveStatus.text = error.message
            return
        }

        settingsRepository.save(settings)
        SyncScheduler.ensurePeriodicWork(this)
        SyncScheduler.enqueueImmediate(this)
        saveStatus.setText(R.string.configuration_saved)
        Toast.makeText(this, R.string.configuration_saved, Toast.LENGTH_SHORT).show()
    }

    private fun loadRecentAppsIfNeeded() {
        if (recentAppsLoaded || recentAppsLoading) return

        recentAppsLoading = true
        val savedGoalExemptPackages = selectedGoalExemptPackages.toSet()
        goalExemptAppsStatus.setText(R.string.goal_exempt_apps_loading)
        Thread {
            try {
                val now = System.currentTimeMillis()
                val packageNames = AndroidUsageEventSource(applicationContext)
                    .query(now - RECENT_APP_WINDOW_MILLIS, now)
                    .asSequence()
                    .filter { it.type == ForegroundEventType.RESUMED }
                    .map(ForegroundEvent::packageName)
                    .filter(String::isNotBlank)
                    .toSet() + savedGoalExemptPackages
                val choices = packageNames.map(::resolveAppChoice)
                    .sortedWith(
                        compareBy<AppChoice> { it.label.lowercase(Locale.getDefault()) }
                            .thenBy(AppChoice::packageName),
                    )
                runOnUiThread {
                    if (isDestroyed) return@runOnUiThread
                    recentAppsLoading = false
                    recentAppsLoaded = true
                    renderAppChoices(choices)
                }
            } catch (_: RuntimeException) {
                runOnUiThread {
                    if (isDestroyed) return@runOnUiThread
                    recentAppsLoading = false
                    renderAppChoices(selectedGoalExemptPackages.map { AppChoice(it, it) })
                    goalExemptAppsStatus.setText(R.string.goal_exempt_apps_load_failed)
                }
            }
        }.apply {
            name = "recent-screen-time-apps"
            start()
        }
    }

    @Suppress("DEPRECATION")
    private fun resolveAppChoice(packageName: String): AppChoice {
        val label = try {
            val applicationInfo = packageManager.getApplicationInfo(packageName, 0)
            packageManager.getApplicationLabel(applicationInfo).toString()
        } catch (_: PackageManager.NameNotFoundException) {
            packageName
        }
        return AppChoice(packageName = packageName, label = label)
    }

    private fun renderAppChoices(choices: Collection<AppChoice>) {
        goalExemptAppsContainer.removeAllViews()
        if (choices.isEmpty()) {
            goalExemptAppsStatus.setText(R.string.goal_exempt_apps_empty)
            return
        }

        goalExemptAppsStatus.text = resources.getQuantityString(
            R.plurals.goal_exempt_apps_found,
            choices.size,
            choices.size,
        )
        choices.forEach { choice ->
            goalExemptAppsContainer.addView(CheckBox(this).apply {
                text = if (choice.label == choice.packageName) {
                    choice.packageName
                } else {
                    "${choice.label}\n${choice.packageName}"
                }
                isChecked = choice.packageName in selectedGoalExemptPackages
                setPadding(0, dp(4), 0, dp(4))
                setOnCheckedChangeListener { _, checked ->
                    if (checked) {
                        selectedGoalExemptPackages += choice.packageName
                    } else {
                        selectedGoalExemptPackages -= choice.packageName
                    }
                }
            }, matchWidth())
        }
    }

    private fun openUsageAccessSettings() {
        val packageUri = Uri.parse("package:$packageName")
        try {
            startActivity(Intent(Settings.ACTION_USAGE_ACCESS_SETTINGS, packageUri))
        } catch (_: ActivityNotFoundException) {
            startActivity(Intent(Settings.ACTION_USAGE_ACCESS_SETTINGS))
        }
    }

    private fun fieldLabel(textResource: Int) = TextView(this).apply {
        setText(textResource)
        textSize = 14f
        setTypeface(typeface, Typeface.BOLD)
    }

    private fun matchWidth(topMargin: Int = 0) = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.MATCH_PARENT,
        LinearLayout.LayoutParams.WRAP_CONTENT,
    ).apply {
        this.topMargin = topMargin
    }

    private fun applySystemBarInsets(view: View) {
        val basePadding = dp(24)
        view.setOnApplyWindowInsetsListener { target, insets ->
            if (android.os.Build.VERSION.SDK_INT >= 30) {
                val bars = insets.getInsets(WindowInsets.Type.systemBars())
                target.setPadding(
                    basePadding + bars.left,
                    basePadding + bars.top,
                    basePadding + bars.right,
                    basePadding + bars.bottom,
                )
            } else {
                @Suppress("DEPRECATION")
                target.setPadding(
                    basePadding + insets.systemWindowInsetLeft,
                    basePadding + insets.systemWindowInsetTop,
                    basePadding + insets.systemWindowInsetRight,
                    basePadding + insets.systemWindowInsetBottom,
                )
            }
            insets
        }
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    private data class AppChoice(
        val packageName: String,
        val label: String,
    )

    companion object {
        private const val RECENT_APP_WINDOW_MILLIS = 30L * 24 * 60 * 60 * 1000
    }
}
