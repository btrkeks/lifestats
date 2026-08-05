package dev.lifestats.screentimelogger

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.graphics.Typeface
import android.net.Uri
import android.os.Bundle
import android.provider.Settings
import android.text.InputType
import android.view.View
import android.view.WindowInsets
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import java.util.Locale

class MainActivity : Activity() {
    private lateinit var baseUrlInput: EditText
    private lateinit var statisticIdInput: EditText
    private lateinit var usageAccessStatus: TextView
    private lateinit var saveStatus: TextView

    private val settingsRepository by lazy { SettingsRepository(this) }
    private val usageAccessChecker by lazy { AndroidUsageAccessChecker(this) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(createContentView())
        populateSavedSettings()
    }

    override fun onResume() {
        super.onResume()
        usageAccessStatus.setText(
            if (usageAccessChecker.hasAccess()) {
                R.string.usage_access_granted
            } else {
                R.string.usage_access_missing
            },
        )
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

        content.addView(fieldLabel(R.string.statistic_id_label).apply {
            setPadding(0, dp(16), 0, 0)
        })
        statisticIdInput = EditText(this).apply {
            hint = getString(R.string.statistic_id_hint)
            inputType = InputType.TYPE_CLASS_NUMBER
            setSingleLine(true)
        }
        content.addView(statisticIdInput, matchWidth())

        usageAccessStatus = TextView(this).apply {
            textSize = 16f
            setPadding(0, dp(24), 0, dp(8))
        }
        content.addView(usageAccessStatus)

        content.addView(Button(this).apply {
            setText(R.string.open_usage_access)
            setOnClickListener { openUsageAccessSettings() }
        }, matchWidth())

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
            statisticIdInput.setText(String.format(Locale.ROOT, "%d", settings.statisticId))
        }
    }

    private fun saveAndSync() {
        val settings = try {
            SettingsValidator.parse(baseUrlInput.text.toString(), statisticIdInput.text.toString())
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
}
