package com.alfonsusenrico.headquarter.ui

import android.content.Intent
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.net.toUri
import com.alfonsusenrico.headquarter.Core
import com.alfonsusenrico.headquarter.Route
import uniffi.hq_mobile.ComputerInfo
import uniffi.hq_mobile.GroupRows
import uniffi.hq_mobile.LinkStatus
import uniffi.hq_mobile.Row
import uniffi.hq_mobile.RowStatus

@Composable
fun HomeScreen(
    core: Core,
    version: ULong,
    go: (Route) -> Unit,
) {
    val tokens = LocalTokens.current
    val phone = core.phone
    val status = remember(version) { phone.status() }
    val home = remember(version) { phone.home() }
    val computers = remember(version) { phone.computers() }
    val current = computers.firstOrNull { it.selected }
    val newer by core.newer.collectAsState()
    var sheet by remember { mutableStateOf(false) }
    // The owner's taps on group headers. Otherwise a group is open while active (D7).
    val folded = remember { mutableStateMapOf<Long, Boolean>() }
    val context = LocalContext.current

    DisposableEffect(Unit) {
        core.io { phone.setHomeVisible(true) }
        onDispose { core.io { phone.setHomeVisible(false) } }
    }

    val open: (Row) -> Unit = { row ->
        if (row.status == RowStatus.INACTIVE) go(Route.Inactive(row.id)) else go(Route.Session(row.id))
    }

    Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize().statusBarsPadding()) {
            Row(
                Modifier.fillMaxWidth().padding(start = 16.dp, end = 4.dp, top = 6.dp, bottom = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Headquarter", fontSize = 20.sp, fontWeight = FontWeight.Bold, modifier = Modifier.weight(1f))
                Switcher(current?.name ?: "", status) { sheet = true }
                Box(
                    Modifier
                        .size(48.dp)
                        .clickable(role = Role.Button) { go(Route.Settings) }
                        .semantics { contentDescription = "Settings" },
                    contentAlignment = Alignment.Center,
                ) { Text("⚙", fontSize = 20.sp, color = tokens.muted) }
            }
            newer?.let { release ->
                Banner("Headquarter v${release.version} is available", info = true, action = "Download") {
                    context.startActivity(Intent(Intent.ACTION_VIEW, release.page.toUri()))
                }
            }
            when (status) {
                is LinkStatus.ComputerTooOld -> {
                    Message(
                        "Update Headquarter on ${current?.name}",
                        "${current?.name} runs Headquarter ${status.version}, which is older than this app needs. " +
                            "Update it on the computer (Check for Updates…), then come back.",
                    )
                }

                is LinkStatus.AppTooOld -> {
                    Message(
                        "Update this app",
                        "${current?.name} runs Headquarter ${status.version}, which is newer than this app. " +
                            "Download the new version from the release page.",
                    ) {
                        HqButton("Open the release page", onClick = {
                            context.startActivity(Intent(Intent.ACTION_VIEW, LATEST_RELEASE.toUri()))
                        }, primary = true)
                    }
                }

                LinkStatus.NotKnown -> {
                    Message(
                        "${current?.name} no longer knows this phone",
                        "It was removed on the computer. Pair again to use it.",
                    ) {
                        HqButton("Pair again", onClick = { go(Route.Pair) }, primary = true)
                    }
                }

                else -> {
                    ConnectionBanner(core, status, version, current)
                    Sessions(home, status, folded, open)
                }
            }
        }
        if (status == LinkStatus.Connected || status is LinkStatus.Reconnecting) {
            Box(
                Modifier
                    .align(Alignment.BottomEnd)
                    .navigationBarsPadding()
                    .padding(18.dp)
                    .height(48.dp)
                    .clip(RoundedCornerShape(24.dp))
                    .background(tokens.accent)
                    .clickable(role = Role.Button) { go(Route.Add) }
                    .padding(horizontal = 20.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    "+ Add",
                    color = if (tokens.dark) tokens.terminalBackground else tokens.card,
                    fontWeight = FontWeight.SemiBold,
                )
            }
        }
        if (sheet) {
            ComputersSheet(
                computers = computers,
                onPick = {
                    sheet = false
                    if (!it.selected) core.io { phone.select(it.id) }
                },
                onPair = {
                    sheet = false
                    go(Route.Pair)
                },
                onDismiss = { sheet = false },
            )
        }
    }
}

@Composable
private fun Switcher(
    name: String,
    status: LinkStatus,
    onClick: () -> Unit,
) {
    val tokens = LocalTokens.current
    val shape = RoundedCornerShape(10.dp)
    Row(
        Modifier
            .heightIn(min = 40.dp)
            .clip(shape)
            .background(tokens.card)
            .border(1.dp, tokens.line, shape)
            .clickable(role = Role.Button, onClick = onClick)
            .padding(horizontal = 10.dp, vertical = 6.dp)
            .semantics { contentDescription = "Computer: $name. Switch computers" },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Dot(status == LinkStatus.Connected, breathing = status != LinkStatus.Connected)
        Text(name, fontSize = 13.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
        Text("⌄", fontSize = 13.sp, color = tokens.muted)
    }
}

@Composable
fun Dot(
    on: Boolean,
    breathing: Boolean = false,
) {
    val tokens = LocalTokens.current
    val alpha =
        if (breathing) {
            val transition = rememberInfiniteTransition(label = "dot")
            transition
                .animateFloat(1f, 0.35f, infiniteRepeatable(tween(700), RepeatMode.Reverse), label = "dot")
                .value
        } else {
            1f
        }
    Box(
        Modifier
            .size(8.dp)
            .alpha(alpha)
            .clip(CircleShape)
            .background(
                if (on) {
                    tokens.ok
                } else if (breathing) {
                    tokens.glyphAmber
                } else {
                    tokens.track
                },
            ),
    )
}

@Composable
private fun ConnectionBanner(
    core: Core,
    status: LinkStatus,
    version: ULong,
    current: ComputerInfo?,
) {
    val tokens = LocalTokens.current
    if (status is LinkStatus.Reconnecting) {
        val since = remember(version) { core.phone.currentAt() }
        val now = System.currentTimeMillis()
        val waited = now - status.sinceMs.toLong()
        val age = since?.let { " The list shows what it knew ${ageText(now - it.toLong())} ago." } ?: ""
        val text =
            if (waited > CANT_REACH_MS) {
                "Can't reach ${current?.name}. Is Headquarter running there with remote control on?$age"
            } else {
                "Lost the connection to ${current?.name}. Reconnecting.$age"
            }
        Banner(text, info = false)
    }
    if (status == LinkStatus.Connecting) {
        Text(
            "Connecting to ${current?.name}…",
            color = tokens.muted,
            fontSize = 13.sp,
            modifier = Modifier.padding(horizontal = 16.dp),
        )
    }
}

@Composable
private fun Sessions(
    home: uniffi.hq_mobile.Home,
    status: LinkStatus,
    folded: MutableMap<Long, Boolean>,
    open: (Row) -> Unit,
) {
    val dim = if (status is LinkStatus.Reconnecting || status == LinkStatus.Off) 0.6f else 1f
    LazyColumn(
        Modifier.fillMaxSize().alpha(dim).padding(horizontal = 12.dp),
        contentPadding =
            androidx.compose.foundation.layout
                .PaddingValues(bottom = 96.dp),
    ) {
        if (home.usage.isNotEmpty()) item { UsageStrip(home.usage) }
        items(home.alerts, key = { "alert-${it.id}" }) { AlertRow(it) { open(it) } }
        home.groups.forEach { group ->
            val isOpen = !(folded[group.id] ?: !group.active)
            item(key = "group-${group.id}") {
                GroupHeader(group, isOpen) { folded[group.id] = isOpen }
            }
            if (isOpen) items(group.rows, key = { "${group.id}-${it.id}" }) { SessionRow(it) { open(it) } }
        }
        if (home.ungrouped.isNotEmpty() && home.groups.isNotEmpty()) item { SectionLabel("No group") }
        items(home.ungrouped, key = { "rest-${it.id}" }) { SessionRow(it) { open(it) } }
        if (home.alerts.isEmpty() && home.groups.isEmpty() && home.ungrouped.isEmpty() &&
            status == LinkStatus.Connected
        ) {
            item {
                Text(
                    "No sessions in the sidebar. + Add puts a past conversation here.",
                    color = LocalTokens.current.muted,
                    modifier = Modifier.padding(vertical = 24.dp, horizontal = 4.dp),
                )
            }
        }
    }
}

@Composable
private fun UsageStrip(usage: List<uniffi.hq_mobile.UsageMeter>) {
    val tokens = LocalTokens.current
    val shape = RoundedCornerShape(12.dp)
    // Meters wrap to a new line rather than squeeze, since three rarely fit a phone's width.
    FlowRow(
        Modifier
            .fillMaxWidth()
            .padding(bottom = 8.dp)
            .clip(shape)
            .background(tokens.card)
            .border(1.dp, tokens.line, shape)
            .padding(horizontal = 12.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        usage.forEach { meter ->
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(agentName(meter.agent), fontSize = 12.5.sp, fontWeight = FontWeight.SemiBold, softWrap = false)
                Text(
                    meter.window,
                    fontSize = 12.5.sp,
                    color = tokens.muted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.widthIn(max = 140.dp),
                )
                Box(
                    Modifier
                        .width(46.dp)
                        .height(5.dp)
                        .clip(RoundedCornerShape(3.dp))
                        .background(tokens.track),
                ) {
                    val near = meter.percent >= 80.0
                    Box(
                        Modifier
                            .fillMaxWidth((meter.percent / 100.0).toFloat())
                            .height(5.dp)
                            .background(if (near) tokens.ansi[1] else tokens.accent),
                    )
                }
                Text("${meter.percent.toInt()}%", fontSize = 12.5.sp, color = tokens.muted, softWrap = false)
            }
        }
    }
}

@Composable
private fun AlertRow(
    row: Row,
    onClick: () -> Unit,
) {
    val tokens = LocalTokens.current
    val edge = if (row.status == RowStatus.NEEDS_YOU) tokens.glyphAmber else tokens.accent
    val shape = RoundedCornerShape(12.dp)
    Row(
        Modifier
            .fillMaxWidth()
            .padding(bottom = 8.dp)
            .clip(shape)
            .background(tokens.card)
            .border(1.dp, tokens.line, shape)
            .clickable(role = Role.Button, onClick = onClick),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(4.dp).height(64.dp).background(edge))
        Spacer(Modifier.width(10.dp))
        StatusGlyph(row.status)
        Column(Modifier.weight(1f).padding(horizontal = 10.dp, vertical = 10.dp)) {
            Text(
                row.title,
                fontSize = 15.sp,
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                "${row.detail} · ${row.age}",
                fontSize = 12.5.sp,
                color = if (row.status == RowStatus.NEEDS_YOU) tokens.amber else tokens.muted,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        Text("›", color = tokens.muted, modifier = Modifier.padding(end = 12.dp))
    }
}

@Composable
private fun GroupHeader(
    group: GroupRows,
    open: Boolean,
    onToggle: () -> Unit,
) {
    val tokens = LocalTokens.current
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .clickable(role = Role.Button, onClick = onToggle)
            .padding(horizontal = 6.dp)
            .semantics {
                contentDescription =
                    "${group.name}, ${group.rows.size} sessions, ${if (open) "open" else "folded"}"
            },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Box(Modifier.size(9.dp).clip(CircleShape).background(tokens.groups[group.color] ?: tokens.muted))
        Text(group.name, fontWeight = FontWeight.SemiBold, color = tokens.group, modifier = Modifier.weight(1f))
        Text("${group.rows.size}", color = tokens.muted, fontSize = 12.5.sp)
        Text(if (open) "⌄" else "›", color = tokens.muted)
    }
}

@Composable
private fun SessionRow(
    row: Row,
    onClick: () -> Unit,
) {
    val tokens = LocalTokens.current
    val dim = row.status == RowStatus.INACTIVE || row.status == RowStatus.ELSEWHERE
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .clip(RoundedCornerShape(10.dp))
            .clickable(role = Role.Button, enabled = row.status != RowStatus.ELSEWHERE, onClick = onClick)
            .padding(horizontal = 8.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        AgentMark(row.agent)
        Text(
            row.title,
            fontSize = 15.sp,
            color = if (dim) tokens.muted else tokens.text,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        when (row.status) {
            RowStatus.WORKING -> {
                Breathing("working")
            }

            RowStatus.STARTING -> {
                Breathing("starting")
            }

            RowStatus.ELSEWHERE -> {
                Text("in a terminal", color = tokens.muted, fontSize = 12.5.sp)
            }

            else -> {
                Text(row.age, color = tokens.muted, fontSize = 12.5.sp)
                StatusGlyph(row.status)
            }
        }
    }
}

@Composable
private fun Breathing(word: String) {
    val transition = rememberInfiniteTransition(label = word)
    val alpha by transition.animateFloat(1f, 0.35f, infiniteRepeatable(tween(1000), RepeatMode.Reverse), label = word)
    Text(word, color = LocalTokens.current.muted, fontSize = 12.5.sp, modifier = Modifier.alpha(alpha))
}

private const val LATEST_RELEASE = "https://github.com/alfonsusenrico/headquarter/releases/latest"

@Composable
private fun Message(
    title: String,
    body: String,
    action: @Composable () -> Unit = {},
) {
    Column(
        Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp, Alignment.CenterVertically),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(title, fontSize = 17.sp, fontWeight = FontWeight.SemiBold)
        Text(body, color = LocalTokens.current.muted, textAlign = androidx.compose.ui.text.style.TextAlign.Center)
        action()
    }
}

@Composable
private fun ComputersSheet(
    computers: List<ComputerInfo>,
    onPick: (ComputerInfo) -> Unit,
    onPair: () -> Unit,
    onDismiss: () -> Unit,
) {
    val tokens = LocalTokens.current
    Box(
        Modifier
            .fillMaxSize()
            .background(Color.Black.copy(alpha = 0.35f))
            .clickable(onClick = onDismiss),
        contentAlignment = Alignment.BottomCenter,
    ) {
        Column(
            Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(topStart = 20.dp, topEnd = 20.dp))
                .background(tokens.page)
                .clickable(enabled = false) {}
                .navigationBarsPadding()
                .padding(horizontal = 12.dp, vertical = 10.dp),
        ) {
            Box(
                Modifier
                    .align(
                        Alignment.CenterHorizontally,
                    ).width(40.dp)
                    .height(4.dp)
                    .clip(CircleShape)
                    .background(tokens.track),
            )
            SectionLabel("Computers")
            Card {
                computers.forEach { computer ->
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .heightIn(min = 56.dp)
                            .clickable(role = Role.RadioButton) { onPick(computer) }
                            .padding(horizontal = 14.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        Radio(computer.selected)
                        Column(Modifier.weight(1f)) {
                            Text(computer.name, fontSize = 15.sp)
                            Text(
                                if (computer.selected) "Shown now" else seenText(computer.lastSeenAt),
                                color = tokens.muted,
                                fontSize = 12.5.sp,
                            )
                        }
                    }
                }
            }
            Box(Modifier.fillMaxWidth().padding(bottom = 8.dp), contentAlignment = Alignment.Center) {
                HqButton("+ Pair another computer", onClick = onPair)
            }
        }
    }
}

@Composable
fun Radio(on: Boolean) {
    val tokens = LocalTokens.current
    Box(
        Modifier
            .size(18.dp)
            .clip(CircleShape)
            .border(2.dp, if (on) tokens.accent else tokens.track, CircleShape)
            .padding(4.dp)
            .clip(CircleShape)
            .background(if (on) tokens.accent else Color.Transparent),
    )
}

fun seenText(lastSeenAt: ULong?): String =
    lastSeenAt?.let { "Seen ${ageText(System.currentTimeMillis() - it.toLong())} ago" } ?: "Never connected"

/** Ages in the desktop's style: 0m, 12m, 3h, 2d, 5w. */
fun ageText(elapsedMs: Long): String {
    val secs = maxOf(elapsedMs, 0) / 1000
    return when {
        secs < 3_600 -> "${secs / 60}m"
        secs < 86_400 -> "${secs / 3_600}h"
        secs < 1_209_600 -> "${secs / 86_400}d"
        else -> "${secs / 604_800}w"
    }
}

private const val CANT_REACH_MS = 120_000L
