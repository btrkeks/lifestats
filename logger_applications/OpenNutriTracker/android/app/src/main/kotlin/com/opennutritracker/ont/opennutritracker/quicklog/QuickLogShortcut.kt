package com.opennutritracker.ont.opennutritracker.quicklog

import android.content.Context
import android.util.Log
import androidx.core.content.pm.ShortcutInfoCompat
import androidx.core.content.pm.ShortcutManagerCompat
import androidx.core.graphics.drawable.IconCompat
import com.opennutritracker.ont.opennutritracker.LaunchInbox
import com.opennutritracker.ont.opennutritracker.R

/**
 * The "Scan Product" entry on the app icon's long-press menu.
 *
 * Dynamic rather than a static shortcuts.xml: a static one names its target
 * package, which differs under the develop flavour, and its label would have
 * to be an Android string rather than the translated one the snapshot
 * carries.
 */
object QuickLogShortcut {
    private const val TAG = "QuickLogShortcut"
    private const val ID = "scan_product"

    /** The label last pushed in this process, to skip the system call when it has not changed. */
    private var pushedLabel: String? = null

    fun update(context: Context, label: String) {
        if (label == pushedLabel) return
        val shortcut = ShortcutInfoCompat.Builder(context, ID)
            .setShortLabel(label)
            .setIcon(IconCompat.createWithResource(context, R.drawable.ql_shortcut_scan))
            .setIntent(LaunchInbox.intentFor(context, LaunchInbox.Request.ScanProduct))
            .build()
        // The launcher menu is a convenience: failing it must not fail the
        // snapshot publish that called this.
        try {
            ShortcutManagerCompat.pushDynamicShortcut(context, shortcut)
            pushedLabel = label
        } catch (e: RuntimeException) {
            Log.w(TAG, "scan shortcut not updated", e)
        }
    }
}
