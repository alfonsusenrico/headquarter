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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
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
import uniffi.hq_mobile.RowStatus

/** Buttons shown together share this width, so a group reads as one choice. */
val ButtonWidth = 168.dp

/** A screen's header: back, the title and its subtitle, and an action. */
@Composable
fun Crumb(
    title: String,
    subtitle: String?,
    onBack: (() -> Unit)?,
    action: @Composable RowScope.() -> Unit = {},
) {
    val tokens = LocalTokens.current
    Row(
        Modifier.fillMaxWidth().padding(start = 4.dp, end = 12.dp, top = 4.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (onBack != null) {
            Box(
                Modifier
                    .size(48.dp)
                    .clickable(role = Role.Button, onClick = onBack)
                    .semantics { contentDescription = "Back" },
                contentAlignment = Alignment.Center,
            ) { Text("‹", color = tokens.accent, fontSize = 28.sp) }
        } else {
            Spacer(Modifier.width(12.dp))
        }
        Column(Modifier.weight(1f)) {
            Text(
                title,
                fontWeight = FontWeight.SemiBold,
                fontSize = 16.sp,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (subtitle != null) {
                Text(subtitle, color = tokens.muted, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        action()
    }
}

@Composable
fun HqButton(
    label: String,
    onClick: () -> Unit,
    primary: Boolean = false,
    danger: Boolean = false,
    enabled: Boolean = true,
    width: Dp = ButtonWidth,
) {
    val tokens = LocalTokens.current
    val shape = RoundedCornerShape(12.dp)
    val background = if (primary) tokens.accent else tokens.card
    val foreground =
        when {
            primary -> onAccent(tokens)
            danger -> tokens.danger
            else -> tokens.text
        }
    Box(
        Modifier
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
    val shape = RoundedCornerShape(14.dp)
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
        fontSize = 11.5.sp,
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
