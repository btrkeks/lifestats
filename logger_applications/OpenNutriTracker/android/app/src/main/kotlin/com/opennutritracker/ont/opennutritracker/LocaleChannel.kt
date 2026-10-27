package com.opennutritracker.ont.opennutritracker

import android.app.LocaleManager
import android.content.Context
import android.os.Build
import android.os.LocaleList
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

/**
 * Keeps the app's own language picker and Android's per-app language picker
 * (Settings -> Apps -> OpenNutriTracker -> Language) telling the same story.
 * Without this the OS picker is visible but inert for anyone who has ever
 * chosen a language in the app, because Flutter reads its locale from our
 * saved config rather than from the system.
 *
 * Registered on the engine, not on an activity: `main()` reads it during
 * boot, and a boot with no activity attached that found no handler would
 * read "no OS override" and push the saved locale over the user's OS choice.
 *
 * The framework LocaleManager is used rather than AppCompatDelegate: the app
 * has no AppCompatActivity, so AppCompatDelegate.setApplicationLocales would
 * silently do nothing. Below API 33 there is no OS picker to stay in step
 * with, so both calls answer harmlessly and the in-app picker remains the
 * only way to change language.
 */
object LocaleChannel {
    private const val CHANNEL = "com.opennutritracker/locale"

    fun register(app: Context, engine: FlutterEngine) {
        MethodChannel(engine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "getApplicationLocale" -> result.success(applicationLocaleTag(app))
                    "setApplicationLocale" -> {
                        setApplicationLocaleTag(app, call.argument<String?>("tag"))
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            }
    }

    /**
     * The language the user picked in Android's settings, or null when they
     * have not overridden it and the app should follow the system.
     */
    private fun applicationLocaleTag(app: Context): String? {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return null
        val locales = app.getSystemService(LocaleManager::class.java)?.applicationLocales
        if (locales == null || locales.isEmpty) return null
        return locales[0]?.toLanguageTag()
    }

    /**
     * Records [tag] as the app's language with the OS. A null tag clears the
     * override, which is what "System default" in our own picker means.
     */
    private fun setApplicationLocaleTag(app: Context, tag: String?) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
        val localeManager = app.getSystemService(LocaleManager::class.java) ?: return
        localeManager.applicationLocales =
            if (tag.isNullOrEmpty()) LocaleList.getEmptyLocaleList()
            else LocaleList.forLanguageTags(tag)
    }
}
