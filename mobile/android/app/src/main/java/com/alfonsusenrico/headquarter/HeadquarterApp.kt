package com.alfonsusenrico.headquarter

import android.app.Application
import androidx.lifecycle.ProcessLifecycleOwner

class HeadquarterApp : Application() {
    lateinit var core: Core
        private set

    override fun onCreate() {
        super.onCreate()
        core = Core(this)
        // The whole app on screen or not, whichever activity shows (D6).
        ProcessLifecycleOwner.get().lifecycle.addObserver(core)
    }
}
