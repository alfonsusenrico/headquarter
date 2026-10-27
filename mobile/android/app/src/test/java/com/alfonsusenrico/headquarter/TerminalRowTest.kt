package com.alfonsusenrico.headquarter

import androidx.compose.ui.text.font.FontWeight
import com.alfonsusenrico.headquarter.ui.DarkTokens
import com.alfonsusenrico.headquarter.ui.rowText
import com.alfonsusenrico.headquarter.ui.spanStyle
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.hq_mobile.Paint
import uniffi.hq_mobile.Span

class TerminalRowTest {
    private fun span(
        col: Int,
        text: String,
        cells: Int = text.length,
        fg: Paint = Paint.Foreground,
        bold: Boolean = false,
    ) = Span(col.toUShort(), cells.toUShort(), text, fg, Paint.Background, bold, false, false, false)

    private fun style(span: Span) = spanStyle(span, DarkTokens)

    @Test
    fun spansStartAtTheirColumnsInTheirOwnStyles() {
        val row = listOf(span(0, "18333"), span(6, "build", fg = Paint.Ansi(4u), bold = true), span(14, "test"))
        val text = rowText(row, ::style)!!

        assertEquals("18333 build   test", text.text)
        val styles = text.spanStyles
        assertEquals(3, styles.size)
        assertEquals(6, styles[1].start)
        assertEquals(11, styles[1].end)
        assertEquals(DarkTokens.ansi[4], styles[1].item.color)
        assertEquals(FontWeight.Bold, styles[1].item.fontWeight)
        assertEquals(14, styles[2].start)
    }

    @Test
    fun aRowWithWideCharactersIsDrawnSpanBySpan() {
        val row = listOf(span(0, "ok "), span(3, "中文", cells = 4), span(7, " done"))

        assertNull(rowText(row, ::style))
    }

    @Test
    fun anEmptyRowIsEmptyText() {
        assertEquals("", rowText(emptyList(), ::style)!!.text)
    }
}
