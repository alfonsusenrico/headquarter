package com.alfonsusenrico.headquarter

import com.alfonsusenrico.headquarter.ui.ownerText
import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.hq_mobile.MobileException

class ErrorTextTest {
    @Test
    fun errorsReadAsTheirSentence() {
        val unreachable = "Couldn't reach the computer: the computer did not answer"
        assertEquals(unreachable, MobileException.Failed(unreachable).ownerText())
        val refused = "the pairing code is not valid"
        assertEquals(refused, MobileException.Refused(refused).ownerText())
        assertEquals("Not connected to the computer.", MobileException.NotConnected().ownerText())
    }
}
