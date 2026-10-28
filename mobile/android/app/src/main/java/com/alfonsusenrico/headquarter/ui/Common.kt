package com.alfonsusenrico.headquarter.ui

import androidx.annotation.DrawableRes
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.alfonsusenrico.headquarter.R
import kotlinx.coroutines.delay
import uniffi.hq_mobile.RowStatus

/** Buttons shown together share this width, so a group reads as one choice. */
val ButtonWidth = 168.dp

/** A screen's header: back, the title and its subtitle, and an action (refresh-phone-screens D1). */
@Composable
fun Crumb(
    title: String,
    subtitle: String?,
    onBack: (() -> Unit)?,
    action: @Composable RowScope.() -> Unit = {},
) {
    val tokens = LocalTokens.current
    Row(
        Modifier.fillMaxWidth().heightIn(min = 56.dp).padding(start = 4.dp, end = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (onBack != null) {
            IconAction(R.drawable.ic_back, "Back", tint = tokens.accent, onClick = onBack)
        } else {
            Spacer(Modifier.width(12.dp))
        }
        Column(Modifier.weight(1f)) {
            Text(
                title,
                fontWeight = FontWeight.SemiBold,
                fontSize = 17.sp,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (subtitle != null) {
                Text(subtitle, color = tokens.muted, fontSize = 12.5.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        action()
    }
}

@Composable
fun HqButton(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    primary: Boolean = false,
    danger: Boolean = false,
    enabled: Boolean = true,
    width: Dp = ButtonWidth,
) {
    val tokens = LocalTokens.current
    val shape = CircleShape
    val background = if (primary) tokens.accent else tokens.card
    val foreground =
        when {
            primary -> onAccent(tokens)
            danger -> tokens.danger
            else -> tokens.text
        }
    Box(
        modifier
            .widthIn(min = width)
            .heightIn(min = 48.dp)
            .clip(shape)
            .background(if (enabled) background else tokens.track)
            .border(1.dp, if (primary) tokens.accent else tokens.line, shape)
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .padding(horizontal = 18.dp, vertical = 12.dp),
        contentAlignment = Alignment.Center,
    ) { Text(label, color = foreground, fontWeight = FontWeight.SemiBold, fontSize = 15.sp) }
}

/** Text and icons drawn on the accent colour. */
fun onAccent(tokens: Tokens): Color = if (tokens.dark) tokens.terminalBackground else tokens.card

/** A 48 dp icon button, plain or on a tonal square. */
@Composable
fun IconAction(
    @DrawableRes icon: Int,
    description: String,
    tint: Color,
    tonal: Boolean = false,
    onClick: () -> Unit,
) {
    val tokens = LocalTokens.current
    Box(
        Modifier
            .size(48.dp)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = description },
        contentAlignment = Alignment.Center,
    ) {
        Box(
            Modifier
                .size(40.dp)
                .clip(RoundedCornerShape(12.dp))
                .background(if (tonal) tokens.card else Color.Transparent),
            contentAlignment = Alignment.Center,
        ) { Icon(painterResource(icon), null, tint = tint, modifier = Modifier.size(22.dp)) }
    }
}

/** A rounded group of rows, as the desktop's cards. */
@Composable
fun Card(content: @Composable ColumnScope.() -> Unit) {
    val tokens = LocalTokens.current
    val shape = RoundedCornerShape(16.dp)
    Column(
        Modifier
            .fillMaxWidth()
            .padding(bottom = 12.dp)
            .clip(shape)
            .background(tokens.card)
            .border(1.dp, tokens.line, shape),
        content = content,
    )
}

@Composable
fun SectionLabel(text: String) {
    Text(
        text.uppercase(),
        color = LocalTokens.current.muted,
        fontSize = 12.sp,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = 0.5.sp,
        modifier = Modifier.padding(start = 4.dp, top = 14.dp, bottom = 6.dp),
    )
}

@Composable
fun Banner(
    text: String,
    info: Boolean,
    action: String? = null,
    onAction: () -> Unit = {},
) {
    val tokens = LocalTokens.current
    Row(
        Modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp, vertical = 4.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(if (info) tokens.accentSoft else tokens.warnSoft)
            .padding(horizontal = 12.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        val color = if (info) tokens.accent else tokens.amber
        Text(text, color = color, fontSize = 13.sp, modifier = Modifier.weight(1f))
        if (action != null) {
            Text(
                action,
                color = color,
                fontWeight = FontWeight.SemiBold,
                fontSize = 13.sp,
                modifier = Modifier.clickable(role = Role.Button, onClick = onAction).padding(8.dp),
            )
        }
    }
}

/** A small pill for an action inside a row, such as Add or Unpair. */
@Composable
fun RowPill(
    label: String,
    color: Color,
    outline: Color,
    enabled: Boolean = true,
    onClick: () -> Unit,
) {
    Box(
        Modifier
            .heightIn(min = 48.dp)
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Box(
            Modifier
                .heightIn(min = 36.dp)
                .clip(CircleShape)
                .border(1.dp, outline, CircleShape)
                .padding(horizontal = 14.dp),
            contentAlignment = Alignment.Center,
        ) { Text(label, color = color, fontSize = 13.sp, fontWeight = FontWeight.Medium) }
    }
}

/**
 * "working" or "starting" with three dots that light in turn, then rest for a step, four steps a
 * second, as the desktop's sidebar (refresh-pane-and-status D1).
 */
@Composable
fun WorkingDots(
    word: String,
    color: Color,
) {
    var step by remember { mutableIntStateOf(0) }
    LaunchedEffect(Unit) {
        while (true) {
            delay(DOTS_STEP_MS)
            step = (step + 1) % 4
        }
    }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(3.dp)) {
        Text(word, color = color, fontSize = 12.5.sp, modifier = Modifier.padding(end = 2.dp))
        repeat(3) { index ->
            Box(
                Modifier
                    .size(4.dp)
                    .alpha(if (index == step) 1f else 0.3f)
                    .clip(CircleShape)
                    .background(color),
            )
        }
    }
}

private const val DOTS_STEP_MS = 250L

/** A status glyph as the desktop paints it: needs you, unread, idle, inactive. */
@Composable
fun StatusGlyph(
    status: RowStatus,
    modifier: Modifier = Modifier,
) {
    val tokens = LocalTokens.current
    val label =
        when (status) {
            RowStatus.NEEDS_YOU -> "needs you"
            RowStatus.UNREAD -> "reply not read yet"
            RowStatus.IDLE -> "idle"
            RowStatus.INACTIVE -> "not running"
            RowStatus.WORKING -> "working"
            RowStatus.STARTING -> "starting"
            RowStatus.ELSEWHERE -> "in another terminal"
        }
    Canvas(modifier.size(20.dp).semantics { contentDescription = label }) {
        val unit = size.minDimension / 24f
        when (status) {
            RowStatus.NEEDS_YOU -> {
                drawCircle(tokens.glyphAmber, radius = 8.5f * unit, style = Stroke(2.4f * unit))
                drawCircle(tokens.glyphAmber, radius = 4f * unit)
            }

            RowStatus.UNREAD, RowStatus.IDLE -> {
                val bubble =
                    Path().apply {
                        moveTo(5f * unit, 5.5f * unit)
                        lineTo(19f * unit, 5.5f * unit)
                        lineTo(20.5f * unit, 7f * unit)
                        lineTo(20.5f * unit, 15f * unit)
                        lineTo(19f * unit, 16.5f * unit)
                        lineTo(11f * unit, 16.5f * unit)
                        lineTo(7f * unit, 19.5f * unit)
                        lineTo(7f * unit, 16.5f * unit)
                        lineTo(5f * unit, 16.5f * unit)
                        lineTo(3.5f * unit, 15f * unit)
                        lineTo(3.5f * unit, 7f * unit)
                        close()
                    }
                if (status == RowStatus.UNREAD) {
                    drawPath(bubble, tokens.text)
                    drawCircle(tokens.accent, radius = 3.4f * unit, center = Offset(20f * unit, 4.5f * unit))
                } else {
                    drawPath(bubble, tokens.muted, style = Stroke(2f * unit))
                }
            }

            RowStatus.INACTIVE -> {
                val moon =
                    Path().apply {
                        moveTo(15f * unit, 4.6f * unit)
                        cubicTo(8f * unit, 4.6f * unit, 4f * unit, 9f * unit, 4f * unit, 12f * unit)
                        cubicTo(4f * unit, 16f * unit, 8f * unit, 19.4f * unit, 15f * unit, 19.4f * unit)
                        cubicTo(10f * unit, 17f * unit, 10f * unit, 7f * unit, 15f * unit, 4.6f * unit)
                        close()
                    }
                drawPath(moon, tokens.muted)
            }

            RowStatus.WORKING, RowStatus.STARTING, RowStatus.ELSEWHERE -> {}
        }
    }
}

/** The agent's mark: Claude's asterisk, Codex's hexagon, anything else a circle. */
@Composable
fun AgentMark(
    agent: String,
    color: Color = LocalTokens.current.muted,
) {
    Canvas(Modifier.size(20.dp).semantics { contentDescription = agent }) {
        val unit = size.minDimension / 24f
        val stroke = Stroke(2.2f * unit)
        when (agent) {
            "claude" -> {
                drawLine(color, Offset(12f * unit, 3f * unit), Offset(12f * unit, 21f * unit), 2.2f * unit)
                drawLine(color, Offset(4.2f * unit, 7.5f * unit), Offset(19.8f * unit, 16.5f * unit), 2.2f * unit)
                drawLine(color, Offset(4.2f * unit, 16.5f * unit), Offset(19.8f * unit, 7.5f * unit), 2.2f * unit)
            }

            "codex" -> {
                val hexagon =
                    Path().apply {
                        moveTo(12f * unit, 3f * unit)
                        lineTo(20f * unit, 7.5f * unit)
                        lineTo(20f * unit, 16.5f * unit)
                        lineTo(12f * unit, 21f * unit)
                        lineTo(4f * unit, 16.5f * unit)
                        lineTo(4f * unit, 7.5f * unit)
                        close()
                    }
                drawPath(hexagon, color, style = stroke)
            }

            else -> {
                drawCircle(color, radius = 8f * unit, style = stroke)
            }
        }
    }
}

fun agentName(agent: String): String =
    when (agent) {
        "claude" -> "Claude"
        "codex" -> "Codex"
        else -> "Antigravity"
    }
