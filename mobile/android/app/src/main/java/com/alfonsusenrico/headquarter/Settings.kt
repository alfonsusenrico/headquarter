package com.alfonsusenrico.headquarter

import android.content.Context
import androidx.core.content.edit

/** The look the owner picked, or the phone's own. */
enum class ThemeChoice { System, Light, Dark }

/** The app's settings (add-android-app D9, D10, D11), kept in the app's private storage. */
class Settings(
    context: Context,
) {
    private val prefs = context.getSharedPreferences("settings", Context.MODE_PRIVATE)

    var appLock: Boolean
        get() = prefs.getBoolean("app_lock", false)
        set(value) = prefs.edit { putBoolean("app_lock", value) }

    var updateCheck: Boolean
        get() = prefs.getBoolean("update_check", true)
        set(value) = prefs.edit { putBoolean("update_check", value) }

    var theme: ThemeChoice
        get() = ThemeChoice.entries.firstOrNull { it.name == prefs.getString("theme", null) } ?: ThemeChoice.System
        set(value) = prefs.edit { putString("theme", value.name) }

    /** The terminal's text size in sp. */
    var textSize: Float
        get() = prefs.getFloat("text_size", DEFAULT_TEXT_SIZE)
        set(value) = prefs.edit { putFloat("text_size", value.coerceIn(MIN_TEXT_SIZE, MAX_TEXT_SIZE)) }

    /** When GitHub was last asked for a newer release (Unix milliseconds). */
    var lastUpdateCheck: Long
        get() = prefs.getLong("last_update_check", 0)
        set(value) = prefs.edit { putLong("last_update_check", value) }

    /** The newer release found, as "version page-url", until the app is updated. */
    var newerRelease: String?
        get() = prefs.getString("newer_release", null)
        set(value) = prefs.edit { putString("newer_release", value) }

    companion object {
        const val DEFAULT_TEXT_SIZE = 12f
        const val MIN_TEXT_SIZE = 9f
        const val MAX_TEXT_SIZE = 20f
    }
}
