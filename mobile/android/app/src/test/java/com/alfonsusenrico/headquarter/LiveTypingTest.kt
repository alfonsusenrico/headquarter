package com.alfonsusenrico.headquarter

import com.alfonsusenrico.headquarter.ui.Edit
import com.alfonsusenrico.headquarter.ui.LiveDraft
import com.alfonsusenrico.headquarter.ui.enterSends
import com.alfonsusenrico.headquarter.ui.mirrorEdit
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LiveTypingTest {
    @Test
    fun typedLettersAreTypedAsTheyCome() {
        assertEquals(Edit(0, "f"), mirrorEdit("", "f"))
        assertEquals(Edit(0, "ix"), mirrorEdit("f", "fix"))
    }

    @Test
    fun aCorrectedWordIsErasedAndRetyped() {
        assertEquals(Edit(3, "he "), mirrorEdit("teh ", "the "))
        assertEquals(Edit(2, "he "), mirrorEdit("fix teh", "fix the "))
    }

    @Test
    fun deletingErasesOnly() {
        assertEquals(Edit(1, ""), mirrorEdit("fixx", "fix"))
        assertEquals(Edit(3, ""), mirrorEdit("fix", ""))
    }

    @Test
    fun anEmojiIsErasedAsOneCharacter() {
        val family = "👨‍👩‍👧"
        assertEquals(Edit(1, ""), mirrorEdit("ok $family", "ok "))
        assertEquals(Edit(1, ""), mirrorEdit("ok 👍", "ok "))
        // A skin tone added to a thumb replaces the whole character.
        assertEquals(Edit(1, "👍🏽"), mirrorEdit("👍", "👍🏽"))
    }

    @Test
    fun aNewLineIsTypedAsItIs() {
        assertEquals(Edit(0, "\nsecond"), mirrorEdit("first", "first\nsecond"))
    }

    @Test
    fun theDraftStartsAfreshAfterAKey() {
        val draft = LiveDraft()
        assertEquals(Edit(0, "abc"), draft.change("abc"))
        assertNull("no change, nothing to send", draft.change("abc"))
        draft.reset()
        assertEquals("", draft.text)
        // What was typed before the key is not erased again: the agent's box owns it now.
        assertEquals(Edit(0, "d"), draft.change("d"))
    }

    @Test
    fun enterStartsANewLineOnlyForAgentsKnownToTakeCtrlJ() {
        assertFalse(enterSends("claude", shell = false))
        assertFalse(enterSends("codex", shell = false))
        assertTrue("agy is not checked yet", enterSends("agy", shell = false))
        assertTrue(enterSends(null, shell = false))
        assertTrue("a shell runs the command", enterSends("claude", shell = true))
    }
}
