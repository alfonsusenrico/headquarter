package com.alfonsusenrico.headquarter

import android.app.KeyguardManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import uniffi.hq_mobile.Phone
import uniffi.hq_mobile.newPhoneKey
import java.io.File

/**
 * The app's link to the Rust core: the [Phone], when it is connected (add-android-app D6),
 * the app lock (D9) and the update check (D10). Screens read [version] and take new
 * snapshots from [phone] when it moves.
 */
class Core(
    private val context: Context,
) : DefaultLifecycleObserver {
    val settings = Settings(context)
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private val handler = Handler(Looper.getMainLooper())

    /** True when the phone's key could not be read and a new one was made: pair again. */
    var keyReplaced = false
        private set

    val phone: Phone = openPhone()

    private val versionFlow = MutableStateFlow(0UL)
    val version: StateFlow<ULong> = versionFlow

    private val lockedFlow = MutableStateFlow(lockAvailable() && settings.appLock)
    val locked: StateFlow<Boolean> = lockedFlow

    private val newerFlow = MutableStateFlow(savedNewer())
    val newer: StateFlow<NewerRelease?> = newerFlow

    private var watcher: Job? = null
    private var leftAt = 0L
    private val disconnectLater = Runnable { phone.disconnect() }

    private fun openPhone(): Phone {
        val folder = context.filesDir
        val store = PhoneKey(File(folder, "phone.key"))
        val key =
            store.read() ?: newPhoneKey().also {
                // A key that cannot be read means the computers no longer know this phone.
                keyReplaced = File(folder, "phone.key").exists()
                File(folder, "computers.json").delete()
                store.write(it)
            }
        return Phone(key, folder.absolutePath)
    }

    /** Whether the phone has a screen lock, which the app lock needs (D9). */
    fun lockAvailable(): Boolean = context.getSystemService(KeyguardManager::class.java).isDeviceSecure

    fun unlocked() {
        lockedFlow.value = false
        phone.connect()
    }

    fun setAppLock(enabled: Boolean) {
        settings.appLock = enabled
    }

    override fun onStart(owner: LifecycleOwner) {
        handler.removeCallbacks(disconnectLater)
        val away = SystemClock.elapsedRealtime() - leftAt
        if (leftAt != 0L && away > LOCK_AFTER_MS && settings.appLock && lockAvailable()) {
            lockedFlow.value = true
        }
        if (!lockedFlow.value) phone.connect()
        watch()
        checkForUpdate()
    }

    override fun onStop(owner: LifecycleOwner) {
        leftAt = SystemClock.elapsedRealtime()
        // A quick switch to another app keeps the session (D6).
        handler.postDelayed(disconnectLater, DISCONNECT_AFTER_MS)
        watcher?.cancel()
    }

    /**
     * Follows the core's change counter while the app is on screen: it waits in the core until
     * something changes, then takes at most one change a frame. Polling every frame cost an idle
     * home 15% of a core on the owner's phone, through JNA.
     */
    private fun watch() {
        watcher?.cancel()
        watcher =
            scope.launch(Dispatchers.IO) {
                var seen = versionFlow.value
                while (isActive) {
                    val now = phone.waitChange(seen, WAIT_MS)
                    if (now != seen) {
                        seen = now
                        versionFlow.value = now
                        delay(FRAME_MS)
                    }
                }
            }
    }

    private fun checkForUpdate() {
        val installed = BuildConfig.VERSION_NAME
        val saved = newerFlow.value
        if (saved != null && !UpdateCheck.isNewer(saved.version, installed)) {
            settings.newerRelease = null
            newerFlow.value = null
        }
        val now = System.currentTimeMillis()
        if (!settings.updateCheck || now - settings.lastUpdateCheck < UpdateCheck.DAY_MS) return
        settings.lastUpdateCheck = now
        scope.launch(Dispatchers.IO) {
            val found =
                try {
                    UpdateCheck.latest(installed)
                } catch (error: java.io.IOException) {
                    android.util.Log.i("Headquarter", "update check failed: ${error.javaClass.simpleName}")
                    return@launch
                }
            settings.newerRelease = found?.let { "${it.version} ${it.page}" }
            newerFlow.value = found
        }
    }

    private fun savedNewer(): NewerRelease? {
        val saved = settings.newerRelease?.split(" ", limit = 2) ?: return null
        return NewerRelease(saved[0], saved.getOrElse(1) { "" })
    }

    /** Runs a blocking core call off the main thread. */
    fun io(block: () -> Unit) {
        scope.launch(Dispatchers.IO) { block() }
    }

    private companion object {
        const val DISCONNECT_AFTER_MS = 30_000L
        const val LOCK_AFTER_MS = 60_000L
        const val FRAME_MS = 16L
        const val WAIT_MS = 1_000u
    }
}
