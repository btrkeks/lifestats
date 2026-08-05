package dev.lifestats.screentimelogger

import android.app.Application

class ScreenTimeLoggerApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        SyncScheduler.ensurePeriodicWork(this)
    }
}
