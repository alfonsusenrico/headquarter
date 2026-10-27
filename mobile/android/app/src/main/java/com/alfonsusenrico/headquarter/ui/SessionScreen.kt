package com.alfonsusenrico.headquarter.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.isImeVisible
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.TextAutoSize
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import com.alfonsusenrico.headquarter.Core
import com.alfonsusenrico.headquarter.R
import com.alfonsusenrico.headquarter.Route
import kotlinx.coroutines.launch
import uniffi.hq_mobile.Key
import uniffi.hq_mobile.LinkStatus
import uniffi.hq_mobile.MobileException
import uniffi.hq_mobile.RowStatus

private val SessionKeys =
    listOf(
        "1" to Key.ONE,
        "2" to Key.TWO,
        "3" to Key.THREE,
        "Esc" to Key.ESCAPE,
        "⇧Tab" to Key.SHIFT_TAB,
        "↑" to Key.UP,
        "↓" to Key.DOWN,
        "Tab" to Key.TAB,
        "Ctrl-C" to Key.CTRL_C,
    )
private val ShellKeys =
    listOf(
        "Esc" to Key.ESCAPE,
        "Tab" to Key.TAB,
        "↑" to Key.UP,
        "↓" to Key.DOWN,
        "Ctrl-C" to Key.CTRL_C,
        "Ctrl-D" to Key.CTRL_D,
    )

/** A running session, or its shell, at the phone's width (add-android-app D8). */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SessionScreen(
    core: Core,
    version: ULong,
    id: String,
    shell: Boolean,
    go: (Route) -> Unit,
    onBack: () -> Unit,
) {
    val phone = core.phone
    val row = remember(version) { findRow(phone.home(), id) }
    val status = remember(version) { phone.status() }
    val screen = remember(version) { phone.screen() }
    val ended = remember(version) { phone.terminalEnded() }
    var textSize by remember { mutableFloatStateOf(core.settings.textSize) }
    var grid by remember { mutableStateOf<Pair<Int, Int>?>(null) }
    var opened by remember { mutableStateOf(false) }
    var problem by remember { mutableStateOf<String?>(null) }
    var draft by remember { mutableStateOf("") }

    DisposableEffect(id, shell) {
        onDispose {
            opened = false
            core.io { phone.closeTerminal() }
        }
    }
    // Back on this screen: the phone is the device in use again (phase 1, D6).
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        core.io {
            phone.setWatching(true)
            phone.focus()
        }
    }
    LifecycleEventEffect(Lifecycle.Event.ON_PAUSE) { core.io { phone.setWatching(false) } }

    fun open(
        cols: Int,
        rows: Int,
    ) {
        core.io {
            try {
                if (shell) {
                    phone.openShell(id, cols.toUShort(), rows.toUShort())
                } else {
                    phone.openSession(id, cols.toUShort(), rows.toUShort())
                }
                opened = true
                problem = null
            } catch (error: MobileException) {
                problem = error.message
            }
        }
    }

    // Opens once the grid is known and the computer is connected. Reconnects reattach in
    // the core.
    val connected = status == LinkStatus.Connected
    LaunchedEffect(grid != null, connected, problem) {
        val size = grid
        if (size != null && !opened && connected && problem == null) {
            opened = true
            open(size.first, size.second)
        }
    }

    Column(Modifier.fillMaxSize().statusBarsPadding().imePadding()) {
        SessionHeader(row, shell, onBack, onShell = { go(Route.Shell(id)) })
        if (status is LinkStatus.Reconnecting) Banner("Reconnecting…", info = false)
        problem?.let { Banner(it, info = false, action = "Try again") { problem = null } }
        if (ended) Banner("The agent exited.", info = true)
        TerminalView(
            screen = screen,
            textSize = textSize,
            onTextSize = {
                textSize = it
                core.settings.textSize = it
            },
            onGrid = { cols, rows ->
                grid = cols to rows
                if (opened) core.io { phone.resize(cols.toUShort(), rows.toUShort()) }
            },
            onScroll = { lines -> core.io { phone.scroll(lines) } },
            modifier = Modifier.weight(1f).fillMaxWidth(),
        )
        val keys = if (shell) ShellKeys else SessionKeys
        val sendKey = { key: Key -> core.io { runCatching { phone.sendKey(key) } } }
        if (WindowInsets.isImeVisible) {
            FoldedKeys(keys, sendKey)
        } else {
            KeyGrid(keys, if (shell) keys.size else 5, sendKey)
        }
        Composer(
            draft = draft,
            onDraft = { draft = it },
            shell = shell,
            onSend = {
                val text = draft
                draft = ""
                core.io { runCatching { phone.typeText(text) } }
            },
        )
    }
}

/**
 * The title over the status, the agent and the folder, with the shell one tap away (D2). A
 * shell says so in its title and has no shell button.
 */
@Composable
private fun SessionHeader(
    row: uniffi.hq_mobile.Row?,
    shell: Boolean,
    onBack: () -> Unit,
    onShell: () -> Unit,
) {
    val tokens = LocalTokens.current
    val title = row?.title ?: "Session"
    Row(
        Modifier.fillMaxWidth().heightIn(min = 56.dp).padding(start = 4.dp, end = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        IconAction(R.drawable.ic_back, "Back", tint = tokens.text, onClick = onBack)
        Column(Modifier.weight(1f).padding(start = 2.dp)) {
            Text(
                if (shell) "Shell · $title" else title,
                fontWeight = FontWeight.SemiBold,
                fontSize = 16.sp,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                if (!shell && row != null) {
                    StatusGlyph(row.status, Modifier.size(14.dp))
                    Text(
                        statusWord(row.status),
                        color = statusColor(row.status, tokens),
                        fontSize = 13.sp,
                        fontWeight = FontWeight.Medium,
                    )
                }
                val rest =
                    if (shell) {
                        listOfNotNull(row?.folder, "runs on your computer")
                    } else {
                        listOfNotNull(row?.let { agentName(it.agent) }, row?.folder)
                    }
                Text(
                    (if (!shell && row != null) "· " else "") + rest.joinToString(" · "),
                    color = tokens.muted,
                    fontSize = 13.sp,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        if (!shell) {
            IconAction(R.drawable.ic_terminal, "Open the shell", tint = tokens.text, tonal = true, onClick = onShell)
        }
    }
}

private fun statusColor(
    status: RowStatus,
    tokens: Tokens,
) = when (status) {
    RowStatus.NEEDS_YOU -> tokens.amber
    RowStatus.WORKING, RowStatus.STARTING, RowStatus.UNREAD -> tokens.accent
    else -> tokens.muted
}

/**
 * Every key at once while the keyboard is closed (D3): [columns] keys a row, and Enter at the end,
 * as tall as the rows, since answering a prompt needs it most.
 */
@Composable
private fun KeyGrid(
    keys: List<Pair<String, Key>>,
    columns: Int,
    onKey: (Key) -> Unit,
) {
    val tokens = LocalTokens.current
    Row(
        Modifier
            .fillMaxWidth()
            .height(IntrinsicSize.Min)
            .background(tokens.page)
            .padding(start = 8.dp, end = 8.dp, top = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(KeyGap),
    ) {
        Column(Modifier.weight(columns.toFloat()), verticalArrangement = Arrangement.spacedBy(KeyGap)) {
            keys.chunked(columns).forEach { line ->
                Row(horizontalArrangement = Arrangement.spacedBy(KeyGap)) {
                    line.forEach { (label, key) -> KeyButton(label, Modifier.weight(1f)) { onKey(key) } }
                    repeat(columns - line.size) { Spacer(Modifier.weight(1f)) }
                }
            }
        }
        EnterKey(Modifier.weight(1f).fillMaxHeight()) { onKey(Key.ENTER) }
    }
}

/** The keys in one row while typing, so the terminal keeps its rows, Enter pinned at the end (D3). */
@Composable
private fun FoldedKeys(
    keys: List<Pair<String, Key>>,
    onKey: (Key) -> Unit,
) {
    val tokens = LocalTokens.current
    Row(
        Modifier
            .fillMaxWidth()
            .background(tokens.page)
            .padding(start = 8.dp, end = 8.dp, top = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(KeyGap),
    ) {
        Row(
            Modifier.weight(1f).horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(KeyGap),
        ) {
            keys.forEach { (label, key) -> KeyButton(label, Modifier.widthIn(min = 48.dp)) { onKey(key) } }
        }
        EnterKey(Modifier.width(56.dp)) { onKey(Key.ENTER) }
    }
}

/** The text box as a pill, with Send inside it only while it holds text (D4). */
@Composable
private fun Composer(
    draft: String,
    onDraft: (String) -> Unit,
    shell: Boolean,
    onSend: () -> Unit,
) {
    val tokens = LocalTokens.current
    Row(
        Modifier
            .fillMaxWidth()
            .background(tokens.page)
            .navigationBarsPadding()
            .padding(start = 8.dp, end = 8.dp, top = 8.dp, bottom = 10.dp)
            .heightIn(min = 48.dp)
            .clip(RoundedCornerShape(24.dp))
            .background(tokens.card)
            .padding(start = 18.dp, end = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.weight(1f).padding(vertical = 13.dp), contentAlignment = Alignment.CenterStart) {
            if (draft.isEmpty()) {
                Text(if (shell) "Type a command…" else "Type to the agent…", color = tokens.muted, fontSize = 15.sp)
            }
            BasicTextField(
                value = draft,
                onValueChange = onDraft,
                textStyle = TextStyle(color = tokens.text, fontSize = 15.sp),
                cursorBrush = SolidColor(tokens.accent),
                maxLines = 4,
                // A command goes to the shell exactly as typed. Gboard keeps correcting a
                // plain text field without autocorrect, but never a URI field.
                keyboardOptions =
                    if (shell) {
                        KeyboardOptions(
                            capitalization = KeyboardCapitalization.None,
                            autoCorrectEnabled = false,
                            keyboardType = KeyboardType.Uri,
                            imeAction = ImeAction.Send,
                        )
                    } else {
                        KeyboardOptions(imeAction = ImeAction.Send)
                    },
                keyboardActions = KeyboardActions(onSend = { if (draft.isNotEmpty()) onSend() }),
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .semantics { contentDescription = if (shell) "Type a command" else "Type to the agent" },
            )
        }
        if (draft.isNotEmpty()) {
            Box(
                Modifier
                    .padding(start = 6.dp)
                    .size(36.dp)
                    .clip(CircleShape)
                    .background(tokens.accent)
                    .clickable(role = Role.Button, onClick = onSend)
                    .semantics { contentDescription = "Send" },
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    painterResource(R.drawable.ic_send),
                    null,
                    tint = onAccent(tokens),
                    modifier = Modifier.size(18.dp),
                )
            }
        } else {
            Spacer(Modifier.width(6.dp))
        }
    }
}

private val KeyGap = 6.dp

@Composable
fun KeyButton(
    label: String,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    val tokens = LocalTokens.current
    Box(
        modifier
            .heightIn(min = 48.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(tokens.card)
            .clickable(role = Role.Button, onClick = onClick)
            .padding(horizontal = 4.dp),
        contentAlignment = Alignment.Center,
    ) {
        // Seven keys share a narrow phone's row in a shell, so a long label shrinks rather than cut.
        BasicText(
            label,
            style = TextStyle(color = tokens.text, fontSize = 13.sp),
            maxLines = 1,
            autoSize = TextAutoSize.StepBased(minFontSize = 10.sp, maxFontSize = 13.sp),
        )
    }
}

@Composable
private fun EnterKey(
    modifier: Modifier,
    onClick: () -> Unit,
) {
    val tokens = LocalTokens.current
    Box(
        modifier
            .heightIn(min = 48.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(tokens.accent)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = "Enter" },
        contentAlignment = Alignment.Center,
    ) { Text("⏎", fontSize = 20.sp, color = onAccent(tokens)) }
}

/** An inactive session: nothing starts until the owner presses Resume Session. */
@Composable
fun InactiveScreen(
    core: Core,
    version: ULong,
    id: String,
    onResumed: () -> Unit,
    onBack: () -> Unit,
) {
    val tokens = LocalTokens.current
    val row = remember(version) { findRow(core.phone.home(), id) }
    var busy by remember { mutableStateOf(false) }
    var problem by remember { mutableStateOf<String?>(null) }
    Column(Modifier.fillMaxSize().statusBarsPadding()) {
        Crumb(
            row?.title ?: "Session",
            listOfNotNull(
                row?.folder,
                row?.let {
                    agentName(it.agent)
                },
                "not running",
            ).joinToString(" · "),
            onBack,
        )
        Column(
            Modifier.fillMaxSize().padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp, Alignment.CenterVertically),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            StatusGlyph(RowStatus.INACTIVE, Modifier.size(40.dp))
            Text(row?.title ?: "", fontSize = 17.sp, fontWeight = FontWeight.SemiBold)
            Text(
                "Not running. Resuming starts ${row?.let {
                    agentName(
                        it.agent,
                    )
                } ?: "the agent"} on your computer, on this conversation.",
                color = tokens.muted,
                textAlign = androidx.compose.ui.text.style.TextAlign.Center,
            )
            problem?.let { Text(it, color = tokens.danger) }
            HqButton(if (busy) "Resuming…" else "Resume Session", primary = true, enabled = !busy, onClick = {
                busy = true
                core.io {
                    try {
                        core.phone.resume(id)
                        core.scope.launch { onResumed() }
                    } catch (error: MobileException) {
                        problem = error.message
                        busy = false
                    }
                }
            })
        }
    }
}

fun findRow(
    home: uniffi.hq_mobile.Home,
    id: String,
): uniffi.hq_mobile.Row? = (home.alerts + home.groups.flatMap { it.rows } + home.ungrouped).firstOrNull { it.id == id }

fun statusWord(status: RowStatus): String =
    when (status) {
        RowStatus.WORKING -> "working"
        RowStatus.STARTING -> "starting"
        RowStatus.NEEDS_YOU -> "needs you"
        RowStatus.UNREAD -> "idle, not read yet"
        RowStatus.IDLE -> "idle"
        RowStatus.INACTIVE -> "not running"
        RowStatus.ELSEWHERE -> "in another terminal"
    }
