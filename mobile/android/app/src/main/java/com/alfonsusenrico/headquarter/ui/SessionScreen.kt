package com.alfonsusenrico.headquarter.ui

import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.os.Build
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.foundation.text.input.InputTransformation
import androidx.compose.foundation.text.input.KeyboardActionHandler
import androidx.compose.foundation.text.input.TextFieldLineLimits
import androidx.compose.foundation.text.input.TextFieldState
import androidx.compose.foundation.text.input.clearText
import androidx.compose.foundation.text.input.placeCursorAtEnd
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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
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
import androidx.core.net.toUri
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import com.alfonsusenrico.headquarter.Core
import com.alfonsusenrico.headquarter.R
import com.alfonsusenrico.headquarter.Route
import kotlinx.coroutines.launch
import uniffi.hq_mobile.Key
import uniffi.hq_mobile.LinkStatus
import uniffi.hq_mobile.MobileException
import uniffi.hq_mobile.RowBeside
import uniffi.hq_mobile.RowStatus
import java.io.IOException
import androidx.compose.ui.input.key.Key as KeyboardKey

// Six a row, the arrows together on the second; the image key takes the last place
// (add-phone-cursor-movement D1).
private val SessionKeys =
    listOf(
        "1" to Key.ONE,
        "2" to Key.TWO,
        "3" to Key.THREE,
        "Esc" to Key.ESCAPE,
        "⇧Tab" to Key.SHIFT_TAB,
        "Tab" to Key.TAB,
        "←" to Key.LEFT,
        "↑" to Key.UP,
        "↓" to Key.DOWN,
        "→" to Key.RIGHT,
        "Ctrl-C" to Key.CTRL_C,
    )
private const val SESSION_COLUMNS = 6
private val ShellKeys =
    listOf(
        "Esc" to Key.ESCAPE,
        "Tab" to Key.TAB,
        "Ctrl-C" to Key.CTRL_C,
        "Ctrl-D" to Key.CTRL_D,
        "←" to Key.LEFT,
        "↑" to Key.UP,
        "↓" to Key.DOWN,
        "→" to Key.RIGHT,
    )
private const val SHELL_COLUMNS = 4

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
    onTaken: () -> Unit,
) {
    val phone = core.phone
    val row = remember(version) { findRow(phone.home(), id) }
    val status = remember(version) { phone.status() }
    val screen = remember(version) { phone.screen() }
    val ended = remember(version) { phone.terminalEnded() }
    val elsewhere = remember(version) { phone.inUseElsewhere() }
    val context = LocalContext.current
    var textSize by remember { mutableFloatStateOf(core.settings.textSize) }
    var grid by remember { mutableStateOf<Pair<Int, Int>?>(null) }
    var opened by remember { mutableStateOf(false) }
    var problem by remember { mutableStateOf<String?>(null) }
    // What was typed since the last send or key; its edits reach the agent's box as they
    // happen (refine-phone-typing-and-links D1).
    val typed = remember(id, shell) { TextFieldState() }
    val typing = remember { FocusRequester() }
    val keyboard = LocalSoftwareKeyboardController.current
    val sending = remember(version) { phone.sending() }
    var imageProblem by remember { mutableStateOf<String?>(null) }
    // Sends an image, then its path lands in the agent's box (add-image-upload D6).
    val sendImage = { load: () -> ByteArray ->
        imageProblem = null
        core.io {
            try {
                phone.sendImage(load())
                core.scope.launch { typed.clearText() }
            } catch (error: ImageProblem) {
                imageProblem = error.message
            } catch (error: MobileException) {
                imageProblem = error.ownerText()
            } catch (_: IOException) {
                imageProblem = "The image couldn't be opened"
            } catch (_: SecurityException) {
                imageProblem = "The image couldn't be opened"
            }
        }
    }
    val picker =
        rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
            uri?.let { sendImage { prepareImage(context, it) } }
        }

    DisposableEffect(id, shell) {
        onDispose {
            opened = false
            core.io { phone.closeTerminal() }
        }
    }
    // Another device took the session: the phone gives way and goes home, saying where it
    // went (refine-in-use-elsewhere D4).
    LaunchedEffect(elsewhere) {
        val holder = elsewhere ?: return@LaunchedEffect
        val name = row?.title ?: "The session"
        Toast.makeText(context, "$name is open on $holder", Toast.LENGTH_LONG).show()
        onTaken()
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
                problem = error.ownerText()
            }
        }
    }

    // Opens once the grid is known and the computer is connected. Reconnects reattach in
    // the core.
    val connected = status == LinkStatus.Connected
    // The agent's box may have changed while the link was down (D2).
    LaunchedEffect(connected) { typed.clearText() }
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
        sending?.let {
            val percent = if (it.total > 0uL) (it.sent * 100uL / it.total).toInt() else 0
            Banner("Sending image… $percent%", info = true)
        }
        imageProblem?.let { Banner(it, info = false, action = "OK") { imageProblem = null } }
        // Every key starts the next edit afresh: it may change or clear the agent's box (D2).
        val sendKey = { key: Key ->
            typed.clearText()
            core.input { runCatching { phone.sendKey(key) } }
        }
        val linkAt = { row: Int, col: Int -> phone.linkAt(row.toUShort(), col.toUShort()) }
        Box(Modifier.weight(1f).fillMaxWidth()) {
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
                onScroll = { lines, row, col -> core.io { phone.scroll(lines, row.toUShort(), col.toUShort()) } },
                onTap = { row, col ->
                    val link = linkAt(row, col)
                    if (link != null) {
                        openLink(context, link)
                    } else {
                        // On the agent's input line, the cursor goes where the owner tapped
                        // (add-phone-cursor-movement D2); typing then starts there afresh.
                        typed.clearText()
                        core.input { runCatching { phone.moveCursorTo(row.toUShort(), col.toUShort()) } }
                        typing.requestFocus()
                        keyboard?.show()
                    }
                },
                onLongPress = { row, col -> linkAt(row, col)?.let { copyLink(context, it) } },
                modifier = Modifier.fillMaxSize(),
            )
            LiveInput(
                state = typed,
                shell = shell,
                enterSends = enterSends(row?.agent, shell),
                onEdit = { edit -> core.input { runCatching { phone.edit(edit.erase.toUInt(), edit.text) } } },
                onEmptyBackspace = { core.input { runCatching { phone.edit(1u, "") } } },
                onEnter = { sendKey(Key.ENTER) },
                modifier = Modifier.align(Alignment.BottomStart).focusRequester(typing),
            )
        }
        val keys = if (shell) ShellKeys else SessionKeys
        Column(Modifier.background(LocalTokens.current.page).navigationBarsPadding().padding(bottom = 10.dp)) {
            val pickImage =
                if (shell) {
                    null
                } else {
                    { picker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)) }
                }
            if (WindowInsets.isImeVisible) {
                FoldedKeys(keys, sendKey, pickImage)
            } else {
                KeyGrid(keys, if (shell) SHELL_COLUMNS else SESSION_COLUMNS, sendKey, pickImage)
            }
        }
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
                        row.beside?.let(::besideWord) ?: statusWord(row.status),
                        color = row.beside?.let { besideColor(it, tokens) } ?: statusColor(row.status, tokens),
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
            // What runs in the background or waits for review (add-background-work-status D5).
            if (!shell) {
                row?.besideDetail?.let { detail ->
                    Text(detail, color = tokens.muted, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
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
    onImage: (() -> Unit)?,
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
            val lines = keys.chunked(columns)
            lines.forEachIndexed { index, line ->
                // The image key takes the last row's free place (add-image-upload D6).
                val image = onImage.takeIf { index == lines.lastIndex && line.size < columns }
                Row(horizontalArrangement = Arrangement.spacedBy(KeyGap)) {
                    line.forEach { (label, key) -> KeyButton(label, Modifier.weight(1f)) { onKey(key) } }
                    image?.let { ImageKey(Modifier.weight(1f), it) }
                    repeat(columns - line.size - (if (image != null) 1 else 0)) { Spacer(Modifier.weight(1f)) }
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
    onImage: (() -> Unit)?,
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
        // Pinned beside Enter, so it never scrolls out of sight (add-image-upload D6).
        onImage?.let { ImageKey(Modifier.width(56.dp), it) }
        EnterKey(Modifier.width(56.dp)) { onKey(Key.ENTER) }
    }
}

/**
 * The field the keyboard types into, never seen: each edit reaches the agent's own box as it
 * happens (refine-phone-typing-and-links D1), and the cursor stays at the end. Where Enter
 * starts a new line it holds several lines; where Enter sends, it is one line and Enter is its
 * action (D3). A shell gets exact keys: no autocorrect, no capitals.
 */
@Composable
private fun LiveInput(
    state: TextFieldState,
    shell: Boolean,
    enterSends: Boolean,
    onEdit: (Edit) -> Unit,
    onEmptyBackspace: () -> Unit,
    onEnter: () -> Unit,
    modifier: Modifier,
) {
    val options =
        when {
            shell -> {
                // Gboard keeps correcting a plain text field without autocorrect, but never a URI field.
                KeyboardOptions(
                    capitalization = KeyboardCapitalization.None,
                    autoCorrectEnabled = false,
                    keyboardType = KeyboardType.Uri,
                    imeAction = ImeAction.Go,
                )
            }

            enterSends -> {
                KeyboardOptions(capitalization = KeyboardCapitalization.Sentences, imeAction = ImeAction.Send)
            }

            else -> {
                KeyboardOptions(capitalization = KeyboardCapitalization.Sentences, imeAction = ImeAction.Default)
            }
        }
    // Each edit, in order, as the keyboard makes it: the text before and after it.
    val mirror =
        InputTransformation {
            val edit = mirrorEdit(originalText.toString(), asCharSequence().toString())
            if (edit.erase > 0 || edit.text.isNotEmpty()) onEdit(edit)
            placeCursorAtEnd()
        }
    BasicTextField(
        state = state,
        inputTransformation = mirror,
        lineLimits = if (enterSends) TextFieldLineLimits.SingleLine else TextFieldLineLimits.MultiLine(),
        keyboardOptions = options,
        onKeyboardAction = if (enterSends) KeyboardActionHandler { onEnter() } else null,
        modifier =
            modifier
                .size(1.dp)
                .alpha(0f)
                // Backspace with nothing typed here still erases in the agent's box, such as a
                // message recalled with ↑ (D2).
                .onPreviewKeyEvent { event ->
                    val erase =
                        event.type == KeyEventType.KeyDown &&
                            event.key == KeyboardKey.Backspace &&
                            state.text.isEmpty()
                    if (erase) onEmptyBackspace()
                    erase
                }.semantics { contentDescription = if (shell) "Type a command" else "Type to the agent" },
    )
}

/** Opens the photo picker to send an image to the session (add-image-upload D6). */
@Composable
private fun ImageKey(
    modifier: Modifier,
    onClick: () -> Unit,
) {
    val tokens = LocalTokens.current
    Box(
        modifier
            .heightIn(min = 48.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(tokens.card)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = "Send an image" },
        contentAlignment = Alignment.Center,
    ) {
        Icon(painterResource(R.drawable.ic_image), null, tint = tokens.text, modifier = Modifier.size(20.dp))
    }
}

/** Opens a web link in the phone's browser (D4). */
private fun openLink(
    context: Context,
    link: String,
) {
    try {
        context.startActivity(Intent(Intent.ACTION_VIEW, link.toUri()))
    } catch (_: ActivityNotFoundException) {
        Toast.makeText(context, "No app can open this link", Toast.LENGTH_SHORT).show()
    }
}

/** Copies a link. Android 13 and later confirm a copy themselves (D4). */
private fun copyLink(
    context: Context,
    link: String,
) {
    context.getSystemService(ClipboardManager::class.java)?.setPrimaryClip(ClipData.newPlainText("Link", link))
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
        Toast.makeText(context, "Link copied", Toast.LENGTH_SHORT).show()
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
                        problem = error.ownerText()
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

/** "background" or "review", where a session at its prompt shows its age (add-background-work-status D4). */
fun besideWord(beside: RowBeside): String =
    when (beside) {
        RowBeside.BACKGROUND -> "background"
        RowBeside.REVIEW -> "review"
    }

/** Teal for background work, violet for a plan to review, as on the desktop (D8). */
fun besideColor(
    beside: RowBeside,
    tokens: Tokens,
): Color =
    when (beside) {
        RowBeside.BACKGROUND -> tokens.teal
        RowBeside.REVIEW -> tokens.violet
    }

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
