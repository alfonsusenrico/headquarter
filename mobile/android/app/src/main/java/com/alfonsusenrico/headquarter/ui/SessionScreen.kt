package com.alfonsusenrico.headquarter.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import com.alfonsusenrico.headquarter.Core
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
@Composable
fun SessionScreen(
    core: Core,
    version: ULong,
    id: String,
    shell: Boolean,
    go: (Route) -> Unit,
    onBack: () -> Unit,
) {
    val tokens = LocalTokens.current
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
        val title = row?.title ?: "Session"
        val subtitle =
            if (shell) {
                "${row?.folder ?: ""} · runs on your computer"
            } else {
                listOfNotNull(
                    row?.folder,
                    row?.let { agentName(it.agent) },
                    row?.let { statusWord(it.status) },
                ).joinToString(" · ")
            }
        Crumb(if (shell) "Shell · $title" else title, subtitle, onBack) {
            if (!shell) {
                KeyButton(">_ Shell") { go(Route.Shell(id)) }
            }
        }
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
        // Enter stays in reach at the end while the other keys scroll, since they rarely fit.
        Row(
            Modifier
                .fillMaxWidth()
                .background(tokens.page)
                .padding(8.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                Modifier.weight(1f).horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                (if (shell) ShellKeys else SessionKeys).forEach { (label, key) ->
                    KeyButton(label) { core.io { runCatching { phone.sendKey(key) } } }
                }
            }
            KeyButton("⏎") { core.io { runCatching { phone.sendKey(Key.ENTER) } } }
        }
        Row(
            Modifier
                .fillMaxWidth()
                .background(tokens.page)
                .navigationBarsPadding()
                .padding(start = 8.dp, end = 8.dp, bottom = 12.dp, top = 2.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            val send = {
                val text = draft
                draft = ""
                core.io { runCatching { phone.typeText(text) } }
            }
            val shape = RoundedCornerShape(12.dp)
            Box(
                Modifier
                    .weight(1f)
                    .heightIn(min = 48.dp)
                    .clip(shape)
                    .background(tokens.card)
                    .border(1.dp, tokens.line, shape)
                    .padding(horizontal = 12.dp, vertical = 12.dp),
                contentAlignment = Alignment.CenterStart,
            ) {
                if (draft.isEmpty()) {
                    Text(if (shell) "Type a command…" else "Type to the agent…", color = tokens.muted, fontSize = 15.sp)
                }
                BasicTextField(
                    value = draft,
                    onValueChange = { draft = it },
                    textStyle = TextStyle(color = tokens.text, fontSize = 15.sp),
                    cursorBrush = SolidColor(tokens.accent),
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
                    keyboardActions = KeyboardActions(onSend = { send() }),
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .semantics { contentDescription = if (shell) "Type a command" else "Type to the agent" },
                )
            }
            HqButton("Send", onClick = send, primary = true, width = 72.dp)
        }
    }
}

@Composable
fun KeyButton(
    label: String,
    onClick: () -> Unit,
) {
    val tokens = LocalTokens.current
    val shape = RoundedCornerShape(9.dp)
    Box(
        Modifier
            .heightIn(min = 48.dp)
            .clip(shape)
            .background(tokens.card)
            .border(1.dp, tokens.line, shape)
            .clickable(role = Role.Button, onClick = onClick)
            .padding(horizontal = 12.dp),
        contentAlignment = Alignment.Center,
    ) { Text(label, fontSize = 13.sp, color = tokens.text) }
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
