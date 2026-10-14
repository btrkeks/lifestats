package dev.lifestats.bedtime

import android.app.Activity
import android.content.SharedPreferences
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.TypedValue
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

/**
 * Opened from the tile it logs right away and closes itself; opened from the app list it
 * shows the last bedtime and a button, so a stray launch doesn't log anything.
 */
class LogBedtimeActivity : Activity() {

    private lateinit var store: BedtimeStore
    private lateinit var timeView: TextView
    private lateinit var statusView: TextView
    private val handler = Handler(Looper.getMainLooper())
    private val onStoreChanged = SharedPreferences.OnSharedPreferenceChangeListener { _, _ -> render() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        store = BedtimeStore(this)

        timeView = TextView(this).apply {
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 40f)
            gravity = Gravity.CENTER
        }
        statusView = TextView(this).apply { gravity = Gravity.CENTER }
        val logButton = Button(this).apply {
            text = "Log bedtime now"
            setOnClickListener { logAndClose() }
        }
        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(24, 24, 24, 24)
            addView(TextView(this@LogBedtimeActivity).apply {
                text = "Bedtime"
                gravity = Gravity.CENTER
            })
            addView(timeView)
            addView(statusView)
            addView(logButton)
        })

        if (savedInstanceState == null && intent.getBooleanExtra(EXTRA_LOG_NOW, false)) {
            logButton.visibility = Button.GONE
            logAndClose()
        }
        render()
    }

    override fun onStart() {
        super.onStart()
        store.prefs.registerOnSharedPreferenceChangeListener(onStoreChanged)
        render()
    }

    override fun onStop() {
        store.prefs.unregisterOnSharedPreferenceChangeListener(onStoreChanged)
        super.onStop()
    }

    private fun logAndClose() {
        logBedtime(this)
        render()
        handler.postDelayed({ finish() }, CLOSE_AFTER_MS)
    }

    private fun render() {
        val last = store.last()
        timeView.text = last?.bedtime?.time ?: "--:--"
        statusView.text = last?.let { statusLabel(it) } ?: "Nothing logged yet"
    }

    companion object {
        const val EXTRA_LOG_NOW = "log_now"
        private const val CLOSE_AFTER_MS = 3_000L
    }
}
