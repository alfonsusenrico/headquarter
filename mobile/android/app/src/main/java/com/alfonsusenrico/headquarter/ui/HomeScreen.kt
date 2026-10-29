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
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
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
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.net.toUri
import com.alfonsusenrico.headquarter.Core
import com.alfonsusenrico.headquarter.R
import com.alfonsusenrico.headquarter.Route
import uniffi.hq_mobile.ComputerInfo
import uniffi.hq_mobile.GroupRows
import uniffi.hq_mobile.LinkStatus
import uniffi.hq_mobile.Row
import uniffi.hq_mobile.RowStatus

@OptIn(ExperimentalMaterial3Api::class)
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
    // The title row folds away as the list scrolls down and returns at the top (D2).
    val folding = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()
    val titleHeight = with(LocalDensity.current) { TITLE_HEIGHT.toPx() }
    SideEffect { folding.state.heightOffsetLimit = -titleHeight }
    val list = rememberLazyListState()
    // Add shows its label at the top and while scrolling up, and shrinks while scrolling down (D5).
    var scrollingUp by remember { mutableStateOf(true) }
    val followScroll =
        remember {
            object : NestedScrollConnection {
                override fun onPreScroll(
                    available: Offset,
                    source: NestedScrollSource,
                ): Offset {
                    if (available.y < 0f) scrollingUp = false
                    if (available.y > 0f) scrollingUp = true
                    return Offset.Zero
                }
            }
        }
    val addExpanded = scrollingUp || !list.canScrollBackward

    Box(Modifier.fillMaxSize().nestedScroll(folding.nestedScrollConnection).nestedScroll(followScroll)) {
        Column(Modifier.fillMaxSize().statusBarsPadding()) {
            val shown = 1f - folding.state.collapsedFraction
            Box(
                Modifier
                    .fillMaxWidth()
                    .height(TITLE_HEIGHT * shown)
                    .clipToBounds()
                    .alpha(shown),
                contentAlignment = Alignment.CenterStart,
            ) {
                Text(
                    "Headquarter",
                    fontSize = 26.sp,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    modifier = Modifier.padding(start = 20.dp),
                )
            }
            ComputerRow(current?.name ?: "", status, onSwitch = { sheet = true }, onSettings = { go(Route.Settings) })
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
                    Sessions(home, status, folded, list, open)
                }
            }
        }
        if (status == LinkStatus.Connected || status is LinkStatus.Reconnecting) {
            ExtendedFloatingActionButton(
                text = { Text("Add", fontWeight = FontWeight.SemiBold) },
                icon = {
                    Icon(
                        painterResource(R.drawable.ic_add),
                        contentDescription = if (addExpanded) null else "Add",
                    )
                },
                onClick = { go(Route.Add) },
                expanded = addExpanded,
                shape = RoundedCornerShape(16.dp),
                containerColor = tokens.accent,
                contentColor = onAccent(tokens),
                modifier =
                    Modifier
                        .align(Alignment.BottomEnd)
                        .navigationBarsPadding()
                        .padding(18.dp),
            )
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

/** The computer on screen, its link and the way to the switcher, with Settings beside it (D2). */
@Composable
private fun ComputerRow(
    name: String,
    status: LinkStatus,
    onSwitch: () -> Unit,
    onSettings: () -> Unit,
) {
    val tokens = LocalTokens.current
    val shape = RoundedCornerShape(14.dp)
    val state = linkWord(status)
    Row(
        Modifier.fillMaxWidth().padding(start = 16.dp, end = 4.dp, top = 2.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(
            Modifier
                .weight(1f)
                .heightIn(min = 48.dp)
                .clip(shape)
                .background(tokens.card)
                .border(1.dp, tokens.line, shape)
                .clickable(role = Role.Button, onClick = onSwitch)
                .padding(horizontal = 14.dp)
                .semantics { contentDescription = "Computer: $name, $state. Switch computers" },
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Dot(status == LinkStatus.Connected, breathing = status != LinkStatus.Connected)
            Text(
                name,
                fontSize = 14.5.sp,
                fontWeight = FontWeight.Medium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
            Text(state, fontSize = 12.5.sp, color = tokens.muted, maxLines = 1)
            Icon(
                painterResource(R.drawable.ic_chevron_down),
                null,
                tint = tokens.muted,
                modifier = Modifier.size(20.dp),
            )
        }
        IconAction(R.drawable.ic_settings, "Settings", tint = tokens.muted, onClick = onSettings)
    }
}

private fun linkWord(status: LinkStatus): String =
    when (status) {
        LinkStatus.Connected -> "connected"
        LinkStatus.Connecting -> "connecting…"
        is LinkStatus.Reconnecting -> "reconnecting…"
        LinkStatus.NotKnown -> "no longer knows this phone"
        is LinkStatus.ComputerTooOld -> "needs an update"
        is LinkStatus.AppTooOld -> "newer than this app"
        LinkStatus.Off, LinkStatus.NoComputer -> "not connected"
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
    list: LazyListState,
    open: (Row) -> Unit,
) {
    val dim = if (status is LinkStatus.Reconnecting || status == LinkStatus.Off) 0.6f else 1f
    // Rows hold their places while a finger is on the list or it still moves, and take their
    // new places once it rests (add-working-sessions-first D3).
    var touching by remember { mutableStateOf(false) }
    val held = touching || list.isScrollInProgress
    val shownOrder = remember { mutableMapOf<String, List<String>>() }
    val ordered = { section: String, rows: List<Row> ->
        val shown = if (held) heldOrder(rows, shownOrder[section].orEmpty()) { it.id } else rows
        shownOrder[section] = shown.map { it.id }
        shown
    }
    LazyColumn(
        Modifier
            .fillMaxSize()
            .alpha(dim)
            .padding(horizontal = 12.dp)
            // Watches touches without taking them: taps and scrolls work as before.
            .pointerInput(Unit) {
                awaitPointerEventScope {
                    while (true) {
                        val event = awaitPointerEvent(PointerEventPass.Initial)
                        touching = event.changes.any { it.pressed }
                    }
                }
            },
        state = list,
        contentPadding =
            androidx.compose.foundation.layout
                .PaddingValues(bottom = 96.dp),
    ) {
        if (home.usage.isNotEmpty()) item { UsageTiles(home.usage) }
        items(ordered("alerts", home.alerts), key = { "alert-${it.id}" }) { AlertRow(it) { open(it) } }
        home.groups.forEach { group ->
            val isOpen = !(folded[group.id] ?: !group.active)
            item(key = "group-${group.id}") {
                GroupHeader(group, isOpen) { folded[group.id] = isOpen }
            }
            if (isOpen) {
                items(ordered("group-${group.id}", group.rows), key = { "${group.id}-${it.id}" }) {
                    SessionRow(it) { open(it) }
                }
            }
        }
        if (home.ungrouped.isNotEmpty() && home.groups.isNotEmpty()) item { SectionLabel("No group") }
        items(ordered("rest", home.ungrouped), key = { "rest-${it.id}" }) { SessionRow(it) { open(it) } }
        if (home.alerts.isEmpty() && home.groups.isEmpty() && home.ungrouped.isEmpty() &&
            status == LinkStatus.Connected
        ) {
            item {
                Text(
                    "No sessions in the sidebar. Add puts a past conversation here.",
                    color = LocalTokens.current.muted,
                    modifier = Modifier.padding(vertical = 24.dp, horizontal = 4.dp),
                )
            }
        }
    }
}

/** One tile per agent's tightest window, three to a row (D3). */
@Composable
private fun UsageTiles(usage: List<uniffi.hq_mobile.UsageMeter>) {
    Column(Modifier.fillMaxWidth().padding(bottom = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        usage.chunked(TILES_PER_ROW).forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { meter -> UsageTile(meter, Modifier.weight(1f)) }
            }
        }
    }
}

@Composable
private fun UsageTile(
    meter: uniffi.hq_mobile.UsageMeter,
    modifier: Modifier,
) {
    val tokens = LocalTokens.current
    val shape = RoundedCornerShape(14.dp)
    Column(
        modifier
            .clip(shape)
            .background(tokens.card)
            .border(1.dp, tokens.line, shape)
            .padding(start = 10.dp, end = 10.dp, top = 9.dp, bottom = 10.dp),
    ) {
        // The percentage stays whole beside the name; a long window name (agy's model pools) is cut.
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                agentName(meter.agent),
                fontSize = 12.5.sp,
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
            Text("${meter.percent.toInt()}%", fontSize = 12.5.sp, fontWeight = FontWeight.SemiBold, maxLines = 1)
        }
        Text(meter.window, fontSize = 12.sp, color = tokens.muted, maxLines = 1, overflow = TextOverflow.Ellipsis)
        Box(
            Modifier
                .padding(top = 6.dp)
                .fillMaxWidth()
                .height(5.dp)
                .clip(RoundedCornerShape(3.dp))
                .background(tokens.track),
        ) {
            val near = meter.percent >= 80.0
            Box(
                Modifier
                    .fillMaxWidth((meter.percent / 100.0).coerceIn(0.0, 1.0).toFloat())
                    .height(5.dp)
                    .background(if (near) tokens.ansi[1] else tokens.accent),
            )
        }
    }
}

private const val TILES_PER_ROW = 3

@Composable
private fun AlertRow(
    row: Row,
    onClick: () -> Unit,
) {
    val tokens = LocalTokens.current
    val edge = if (row.status == RowStatus.NEEDS_YOU) tokens.glyphAmber else tokens.accent
    val shape = RoundedCornerShape(16.dp)
    Row(
        Modifier
            .fillMaxWidth()
            .padding(bottom = 8.dp)
            .heightIn(min = 68.dp)
            .height(IntrinsicSize.Min)
            .clip(shape)
            .background(tokens.card)
            .border(1.dp, tokens.line, shape)
            .clickable(role = Role.Button, onClick = onClick),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(4.dp).fillMaxHeight().background(edge))
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
        Icon(
            painterResource(R.drawable.ic_chevron_right),
            null,
            tint = tokens.muted,
            modifier = Modifier.padding(end = 10.dp).size(20.dp),
        )
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
        Icon(
            painterResource(if (open) R.drawable.ic_chevron_down else R.drawable.ic_chevron_right),
            null,
            tint = tokens.muted,
            modifier = Modifier.size(20.dp),
        )
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
            .heightIn(min = 52.dp)
            .clip(RoundedCornerShape(12.dp))
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
        val beside = row.beside
        when {
            // "background" or "review" ends the row in place of the age, as on the desktop
            // (add-background-work-status D4).
            beside != null -> {
                Text(beside, color = tokens.accent, fontSize = 12.5.sp, fontWeight = FontWeight.SemiBold)
            }

            row.status == RowStatus.WORKING -> {
                WorkingDots("working", tokens.accent)
            }

            row.status == RowStatus.STARTING -> {
                WorkingDots("starting", tokens.muted)
            }

            row.status == RowStatus.ELSEWHERE -> {
                Text("in a terminal", color = tokens.muted, fontSize = 12.5.sp)
            }

            else -> {
                Text(row.age, color = tokens.muted, fontSize = 12.5.sp)
                StatusGlyph(row.status)
            }
        }
    }
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
            Card {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .heightIn(min = 56.dp)
                        .clickable(role = Role.Button, onClick = onPair)
                        .padding(horizontal = 14.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Icon(
                        painterResource(R.drawable.ic_add),
                        null,
                        tint = tokens.accent,
                        modifier = Modifier.size(20.dp),
                    )
                    Text(
                        "Pair another computer",
                        color = tokens.accent,
                        fontWeight = FontWeight.SemiBold,
                        fontSize = 15.sp,
                    )
                }
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

/** Home's title row, folded away while the list is scrolled (D2). */
private val TITLE_HEIGHT = 56.dp
