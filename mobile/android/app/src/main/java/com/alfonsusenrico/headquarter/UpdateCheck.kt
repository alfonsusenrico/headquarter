package com.alfonsusenrico.headquarter

import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

/** A release newer than the app that carries an Android APK (add-android-app D10). */
data class NewerRelease(
    val version: String,
    val page: String,
)

object UpdateCheck {
    private const val LATEST = "https://api.github.com/repos/alfonsusenrico/headquarter/releases/latest"
    const val DAY_MS = 24L * 60 * 60 * 1000

    /** Asks GitHub for the latest release. Null when it is not newer or has no APK. */
    fun latest(installed: String): NewerRelease? {
        val connection = URL(LATEST).openConnection() as HttpURLConnection
        connection.connectTimeout = 10_000
        connection.readTimeout = 10_000
        connection.setRequestProperty("Accept", "application/vnd.github+json")
        return try {
            if (connection.responseCode != HttpURLConnection.HTTP_OK) return null
            newer(connection.inputStream.bufferedReader().readText(), installed)
        } finally {
            connection.disconnect()
        }
    }

    /** Reads GitHub's answer: the release when it is newer than [installed] and carries an APK. */
    fun newer(
        json: String,
        installed: String,
    ): NewerRelease? {
        val release = JSONObject(json)
        val version = release.optString("tag_name").removePrefix("v")
        val assets = release.optJSONArray("assets") ?: return null
        val hasApk = (0 until assets.length()).any { assets.getJSONObject(it).optString("name").endsWith(".apk") }
        if (!hasApk || !isNewer(version, installed)) return null
        return NewerRelease(version, release.optString("html_url"))
    }

    /** Whether version `a` (such as 0.2.0) comes after `b`. Anything unreadable is not newer. */
    fun isNewer(
        a: String,
        b: String,
    ): Boolean {
        val left = a.split(".").map { it.toIntOrNull() ?: return false }
        val right = b.split(".").map { it.toIntOrNull() ?: return false }
        for (index in 0 until maxOf(left.size, right.size)) {
            val difference = left.getOrElse(index) { 0 } - right.getOrElse(index) { 0 }
            if (difference != 0) return difference > 0
        }
        return false
    }
}
