package com.alfonsusenrico.headquarter.ui

import android.util.LruCache
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.alfonsusenrico.headquarter.R
import com.alfonsusenrico.headquarter.Settings
import uniffi.hq_mobile.Paint
import uniffi.hq_mobile.Screen
import uniffi.hq_mobile.Span
import kotlin.math.floor

private val Padding = 8.dp

/**
 * The open terminal, drawn from the core's snapshot (add-android-app D8). Reports the grid
 * that fits through [onGrid], scrolls with a vertical drag at the dragged cell (the core
 * decides whether that is history or the agent's own scrolling, fix-phone-scroll D2), and
 * changes the text size with a pinch.
 */
@Composable
fun TerminalView(
    screen: Screen?,
    textSize: Float,
    onTextSize: (Float) -> Unit,
    onGrid: (cols: Int, rows: Int) -> Unit,
    onScroll: (lines: Int, row: Int, col: Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val tokens = LocalTokens.current
    val density = LocalDensity.current
    val measurer = rememberTextMeasurer(cacheSize = 0)
    val style = TextStyle(fontFamily = Mono, fontSize = textSize.sp)
    val cell = remember(textSize, density) { measurer.measure("M", style).size }
    // The font's own advance, so a row laid out as one text lines up with the grid.
    val advance = remember(textSize, density) { measurer.measure("M".repeat(100), style).size.width / 100f }
    val rowLayouts = remember(textSize, density, tokens) { RowLayouts(measurer, style, tokens) }
    val currentTextSize by rememberUpdatedState(textSize)
    val currentCell by rememberUpdatedState(cell)
    val currentAdvance by rememberUpdatedState(advance)
    var pending by remember { mutableFloatStateOf(0f) }

    BoxWithConstraints(modifier.background(tokens.terminalBackground)) {
        val padding = with(density) { Padding.toPx() }
        val cols = floor((constraints.maxWidth - 2 * padding) / advance).toInt().coerceAtLeast(10)
        val rows = floor((constraints.maxHeight - 2 * padding) / cell.height).toInt().coerceAtLeast(4)
        LaunchedEffect(cols, rows) { onGrid(cols, rows) }
        Canvas(
            Modifier
                .fillMaxSize()
                .semantics { contentDescription = "Terminal" }
                .pointerInput(Unit) {
                    detectTransformGestures { centroid, pan, zoom, _ ->
                        if (zoom != 1f) {
                            val size = (currentTextSize * zoom).coerceIn(Settings.MIN_TEXT_SIZE, Settings.MAX_TEXT_SIZE)
                            if (size != currentTextSize) onTextSize(size)
                        } else {
                            // Dragging down reveals earlier output.
                            pending += pan.y / currentCell.height
                            val lines = pending.toInt()
                            if (lines != 0) {
                                pending -= lines
                                val inset = Padding.toPx()
                                val row = ((centroid.y - inset) / currentCell.height).toInt().coerceAtLeast(0)
                                val col = ((centroid.x - inset) / currentAdvance).toInt().coerceAtLeast(0)
                                onScroll(lines, row, col)
                            }
                        }
                    }
                },
        ) {
            val shown = screen ?: return@Canvas
            val cw = advance
            val ch = cell.height.toFloat()
            shown.lines.forEachIndexed { row, spans ->
                val y = padding + row * ch
                spans.forEach { span ->
                    val bg = color(span.bg, tokens)
                    if (bg != tokens.terminalBackground) {
                        drawRect(bg, Offset(padding + span.col.toInt() * cw, y), Size(span.cells.toInt() * cw, ch))
                    }
                }
                val layout = rowLayouts.layout(spans)
                if (layout != null) {
                    drawText(layout, topLeft = Offset(padding, y))
                } else {
                    spans.forEach { span ->
                        if (span.text.isNotBlank()) {
                            drawText(
                                rowLayouts.spanLayout(span),
                                topLeft = Offset(padding + span.col.toInt() * cw, y),
                            )
                        }
                    }
                }
            }
            if (shown.cursorVisible) {
                drawRect(
                    tokens.terminalForeground.copy(alpha = 0.5f),
                    Offset(padding + shown.cursorCol.toInt() * cw, padding + shown.cursorRow.toInt() * ch),
                    Size(cw, ch),
                )
            }
        }
        val scrolledBack = screen?.scrolledBack?.toInt() ?: 0
        // Soft edges say there is more output above, or below while scrolled back (D1).
        if (screen != null && screen.history.toInt() > scrolledBack) {
            EdgeFade(Modifier.align(Alignment.TopCenter), tokens.terminalBackground, fromTop = true)
        }
        if (scrolledBack > 0) {
            EdgeFade(Modifier.align(Alignment.BottomCenter), tokens.terminalBackground, fromTop = false)
            LiveButton(Modifier.align(Alignment.BottomEnd).padding(12.dp)) { onScroll(-scrolledBack, 0, 0) }
        }
    }
}

@Composable
private fun EdgeFade(
    modifier: Modifier,
    color: Color,
    fromTop: Boolean,
) {
    val colors = if (fromTop) listOf(color, Color.Transparent) else listOf(Color.Transparent, color)
    Box(modifier.fillMaxWidth().height(16.dp).background(Brush.verticalGradient(colors)))
}

/** Back to the newest output, shown while scrolled back. */
@Composable
private fun LiveButton(
    modifier: Modifier,
    onClick: () -> Unit,
) {
    val tokens = LocalTokens.current
    Row(
        modifier
            .heightIn(min = 40.dp)
            .clip(RoundedCornerShape(20.dp))
            .background(tokens.accentSoft)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = "Back to live output" }
            .padding(start = 10.dp, end = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Icon(painterResource(R.drawable.ic_arrow_down), null, tint = tokens.accent, modifier = Modifier.size(20.dp))
        Text("Live", color = tokens.accent, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
    }
}

/**
 * Text layouts of terminal rows, kept while the rows stay the same so output that only moves
 * (scrolling) is not laid out again. A row whose characters each fill one cell is one layout.
 * A row with wide characters falls back to one layout per span, placed at its column.
 */
private class RowLayouts(
    private val measurer: TextMeasurer,
    private val style: TextStyle,
    private val tokens: Tokens,
) {
    private val rows = LruCache<List<Span>, TextLayoutResult>(ROWS_KEPT)
    private val spans = LruCache<Span, TextLayoutResult>(SPANS_KEPT)

    fun layout(row: List<Span>): TextLayoutResult? {
        rows.get(row)?.let { return it }
        val text = rowText(row) { spanStyle(it, tokens) } ?: return null
        val laidOut = measurer.measure(text, style, softWrap = false, maxLines = 1)
        rows.put(row, laidOut)
        return laidOut
    }

    fun spanLayout(span: Span): TextLayoutResult =
        spans.get(span)
            ?: measurer
                .measure(
                    AnnotatedString(span.text, spanStyle(span, tokens)),
                    style,
                    softWrap = false,
                    maxLines = 1,
                ).also { spans.put(span, it) }

    private companion object {
        // A few screens of rows, so scrolling back and forth reuses them.
        const val ROWS_KEPT = 400
        const val SPANS_KEPT = 400
    }
}

/**
 * A row as one text: each span from its column, with spaces before it, in its own style. Null
 * when a span's characters do not each fill one cell (wide or combining characters), since one
 * text would then drift from the grid.
 */
fun rowText(
    row: List<Span>,
    styleOf: (Span) -> SpanStyle,
): AnnotatedString? {
    if (row.any { it.text.codePointCount(0, it.text.length) != it.cells.toInt() }) return null
    return buildAnnotatedString {
        row.forEach { span ->
            while (length < span.col.toInt()) append(' ')
            withStyle(styleOf(span)) { append(span.text) }
        }
    }
}

fun spanStyle(
    span: Span,
    tokens: Tokens,
): SpanStyle {
    val fg = color(span.fg, tokens)
    return SpanStyle(
        color = if (span.dim) fg.copy(alpha = 0.6f) else fg,
        fontWeight = if (span.bold) FontWeight.Bold else FontWeight.Normal,
        fontStyle = if (span.italic) FontStyle.Italic else FontStyle.Normal,
        textDecoration = if (span.underline) TextDecoration.Underline else null,
    )
}

fun color(
    paint: Paint,
    tokens: Tokens,
): Color =
    when (paint) {
        Paint.Foreground -> tokens.terminalForeground
        Paint.Background -> tokens.terminalBackground
        is Paint.Ansi -> tokens.ansi.getOrElse(paint.index.toInt()) { tokens.terminalForeground }
        is Paint.Rgb -> Color(0xFF000000 or paint.rgb.toLong())
    }
