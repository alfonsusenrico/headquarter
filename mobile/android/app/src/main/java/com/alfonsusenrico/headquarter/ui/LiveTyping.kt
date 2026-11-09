package com.alfonsusenrico.headquarter.ui

/**
 * What one keyboard edit does to the session's input: [erase] characters removed with
 * Backspace, then [text] typed (refine-phone-typing-and-links D1).
 */
data class Edit(
    val erase: Int,
    val text: String,
)

/**
 * Erases what differs at the end of [old], counted in characters as the agents' boxes delete
 * them (grapheme clusters), and types what [next] has in its place.
 */
fun mirrorEdit(
    old: String,
    next: String,
): Edit {
    val kept = commonPrefixOfCharacters(old, next)
    return Edit(erase = characterCount(old.substring(kept)), text = next.substring(kept))
}

/** Whether the keyboard's Enter sends, rather than starting a new line (D3). */
fun enterSends(
    agent: String?,
    shell: Boolean,
): Boolean = shell || agent !in NewLineAgents

/** Agents observed to take Ctrl+J as a new line (research/evidence/phone-typing-images). */
private val NewLineAgents = setOf("claude", "codex")

/** The length, in UTF-16 units, of the whole characters [a] and [b] start with alike. */
private fun commonPrefixOfCharacters(
    a: String,
    b: String,
): Int {
    val prefix = a.commonPrefixWith(b).length
    if (prefix == a.length && prefix == b.length) return prefix
    // Back off to a character boundary of both, so a changed emoji or accent is erased whole.
    val ofA = boundaries(a)
    val ofB = boundaries(b)
    return ofA.filter { it <= prefix && it in ofB }.maxOrNull() ?: 0
}

/** Where each character starts, and the end. */
private fun boundaries(text: String): Set<Int> = setOf(0) + OneCharacter.findAll(text).map { it.range.last + 1 }

/**
 * One user-perceived character (an extended grapheme cluster: an emoji sequence, a letter with
 * its accents). `java.text.BreakIterator` splits emoji sequences on the JVM before 20.
 */
private val OneCharacter = Regex("\\X")

private fun characterCount(text: String): Int = OneCharacter.findAll(text).count()
