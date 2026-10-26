package com.alfonsusenrico.headquarter

import com.alfonsusenrico.headquarter.ui.ageText
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class UpdateCheckTest {
    @Test
    fun versionsCompareByNumber() {
        assertTrue(UpdateCheck.isNewer("0.1.14", "0.1.13"))
        assertTrue(UpdateCheck.isNewer("0.2.0", "0.1.99"))
        assertTrue(UpdateCheck.isNewer("0.10.0", "0.9.0"))
        assertFalse(UpdateCheck.isNewer("0.1.13", "0.1.13"))
        assertFalse(UpdateCheck.isNewer("0.1.12", "0.1.13"))
        assertFalse(UpdateCheck.isNewer("next", "0.1.13"))
    }

    @Test
    fun onlyANewerReleaseWithAnApkIsOffered() {
        val withApk =
            """{"tag_name":"v0.2.0","html_url":"https://example.com/r","assets":[{"name":"headquarter-0.2.0.apk"}]}"""
        assertEquals(NewerRelease("0.2.0", "https://example.com/r"), UpdateCheck.newer(withApk, "0.1.14"))
        assertNull(UpdateCheck.newer(withApk, "0.2.0"))
        val desktopOnly = """{"tag_name":"v0.2.0","html_url":"https://example.com/r","assets":[{"name":"hq.tar.xz"}]}"""
        assertNull(UpdateCheck.newer(desktopOnly, "0.1.14"))
    }

    @Test
    fun agesReadAsOnTheDesktop() {
        assertEquals("0m", ageText(30_000))
        assertEquals("59m", ageText(3_599_000))
        assertEquals("3h", ageText(3 * 3_600_000L))
        assertEquals("2d", ageText(2 * 86_400_000L))
        assertEquals("5w", ageText(35 * 86_400_000L))
    }
}
