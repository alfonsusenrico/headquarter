package com.alfonsusenrico.headquarter

import com.alfonsusenrico.headquarter.ui.MAX_IMAGE_BYTES
import com.alfonsusenrico.headquarter.ui.agentsRead
import com.alfonsusenrico.headquarter.ui.needsConversion
import com.alfonsusenrico.headquarter.ui.scaleFor
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SendImageTest {
    private fun bytes(vararg head: Int) = ByteArray(16) { if (it < head.size) head[it].toByte() else 0 }

    private val png = bytes(0x89, 'P'.code, 'N'.code, 'G'.code, 0x0D, 0x0A, 0x1A, 0x0A)

    @Test
    fun theAgentsReadPngJpegGifAndWebp() {
        assertTrue(agentsRead(png))
        assertTrue(agentsRead(bytes(0xFF, 0xD8, 0xFF, 0xE0)))
        assertTrue(agentsRead("GIF89a".toByteArray() + ByteArray(8)))
        assertTrue(agentsRead("RIFF\u0000\u0000\u0000\u0000WEBPVP8 ".toByteArray(Charsets.ISO_8859_1)))
        // A HEIC photo from the camera: "ftypheic" after the box size.
        assertFalse(agentsRead("\u0000\u0000\u0000\u0018ftypheic".toByteArray(Charsets.ISO_8859_1)))
        assertFalse(agentsRead("%PDF-1.7".toByteArray()))
        assertFalse(agentsRead(ByteArray(0)))
    }

    @Test
    fun anImageIsConvertedWhenTheAgentsCannotReadItOrItIsTooLarge() {
        assertFalse(needsConversion(png))
        assertTrue(needsConversion("\u0000\u0000\u0000\u0018ftypheic".toByteArray(Charsets.ISO_8859_1)))
        val large = png.copyOf(MAX_IMAGE_BYTES.toInt() + 1)
        assertTrue(needsConversion(large))
    }

    @Test
    fun theLongerSideIsScaledTo4096AtMost() {
        assertEquals(1f, scaleFor(1080, 2400), 0f)
        assertEquals(0.5f, scaleFor(8192, 6144), 0f)
        assertEquals(0.5f, scaleFor(3000, 8192), 0f)
    }
}
