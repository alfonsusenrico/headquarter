package com.alfonsusenrico.headquarter.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.alfonsusenrico.headquarter.Settings
import uniffi.hq_mobile.Paint
import uniffi.hq_mobile.Screen
import kotlin.math.floor

private val Padding = 8.dp

/**
 * The open terminal, drawn from the core's snapshot (add-android-app D8). Reports the grid
 * that fits through [onGrid], scrolls history with a vertical drag, and changes the text
 * size with a pinch.
 */
@Composable
fun TerminalView(
    screen: Screen?,
    textSize: Float,
    onTextSize: (Float) -> Unit,
    onGrid: (cols: Int, rows: Int) -> Unit,
    onScroll: (lines: Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val tokens = LocalTokens.current
    val density = LocalDensity.current
    val measurer = rememberTextMeasurer(cacheSize = 512)
    val style = TextStyle(fontFamily = Mono, fontSize = textSize.sp)
    val cell = remember(textSize, density) { measurer.measure("M", style).size }
    val currentTextSize by rememberUpdatedState(textSize)
    val currentCell by rememberUpdatedState(cell)
    var pending by remember { mutableFloatStateOf(0f) }

    BoxWithConstraints(modifier.background(tokens.terminalBackground)) {
        val padding = with(density) { Padding.toPx() }
        val cols = floor((constraints.maxWidth - 2 * padding) / cell.width).toInt().coerceAtLeast(10)
        val rows = floor((constraints.maxHeight - 2 * padding) / cell.height).toInt().coerceAtLeast(4)
        LaunchedEffect(cols, rows) { onGrid(cols, rows) }
        Canvas(
            Modifier
                .fillMaxSize()
                .semantics { contentDescription = "Terminal" }
                .pointerInput(Unit) {
                    detectTransformGestures { _, pan, zoom, _ ->
                        if (zoom != 1f) {
                            val size = (currentTextSize * zoom).coerceIn(Settings.MIN_TEXT_SIZE, Settings.MAX_TEXT_SIZE)
                            if (size != currentTextSize) onTextSize(size)
                        } else {
                            // Dragging down reveals earlier output.
                            pending += pan.y / currentCell.height
                            val lines = pending.toInt()
                            if (lines != 0) {
                                pending -= lines
                                onScroll(lines)
                            }
                        }
                    }
                },
        ) {
            val shown = screen ?: return@Canvas
            val cw = cell.width.toFloat()
            val ch = cell.height.toFloat()
            shown.lines.forEachIndexed { row, spans ->
                val y = padding + row * ch
                spans.forEach { span ->
                    val x = padding + span.col.toInt() * cw
                    val bg = color(span.bg, tokens)
                    if (bg != tokens.terminalBackground) {
                        drawRect(bg, Offset(x, y), Size(span.cells.toInt() * cw, ch))
                    }
                    val text = span.text
                    if (text.isNotBlank()) {
                        val fg = color(span.fg, tokens)
                        drawText(
                            measurer,
                            text,
                            topLeft = Offset(x, y),
                            style =
                                style.copy(
                                    color = if (span.dim) fg.copy(alpha = 0.6f) else fg,
                                    fontWeight = if (span.bold) FontWeight.Bold else FontWeight.Normal,
                                    fontStyle = if (span.italic) FontStyle.Italic else FontStyle.Normal,
                                    textDecoration = if (span.underline) TextDecoration.Underline else null,
                                ),
                            softWrap = false,
                        )
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
    }
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
