package com.alfonsusenrico.headquarter.ui

import androidx.annotation.DrawableRes
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import com.alfonsusenrico.headquarter.BuildConfig
import com.alfonsusenrico.headquarter.Core
import com.alfonsusenrico.headquarter.R
import com.alfonsusenrico.headquarter.Route
import com.alfonsusenrico.headquarter.ThemeChoice
import uniffi.hq_mobile.ComputerInfo
import uniffi.hq_mobile.LinkStatus
import uniffi.hq_mobile.MobileException

@Composable
fun SettingsScreen(
    core: Core,
    version: ULong,
    theme: ThemeChoice,
    onTheme: (ThemeChoice) -> Unit,
    go: (Route) -> Unit,
    onBack: () -> Unit,
) {
    val tokens = LocalTokens.current
    val phone = core.phone
    val computers = remember(version) { phone.computers() }
    val status = remember(version) { phone.status() }
    val path = remember(version) { phone.path() }
    var appLock by remember { mutableStateOf(core.settings.appLock && core.lockAvailable()) }
    var updateCheck by remember { mutableStateOf(core.settings.updateCheck) }
    var note by remember { mutableStateOf<String?>(null) }
    var confirming by remember { mutableStateOf<ComputerInfo?>(null) }

    fun unpair(computer: ComputerInfo) {
        core.io {
            note =
                try {
                    if (phone.unpair(computer.id)) {
                        "Unpaired ${computer.name}."
                    } else {
                        "Forgot ${computer.name} on this phone. It still lists this phone until you remove it there."
                    }
                } catch (error: MobileException) {
                    error.message
                }
        }
    }

    confirming?.let { computer ->
        // Pairing again needs the computer at hand, so a stray tap must not unpair.
        Dialog(onDismissRequest = { confirming = null }) {
            Column(
                Modifier.clip(RoundedCornerShape(16.dp)).background(tokens.card).padding(20.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("Unpair ${computer.name}?", fontSize = 17.sp, fontWeight = FontWeight.SemiBold)
                Text(
                    "This phone is removed from ${computer.name}. Pairing again needs a new code from the computer.",
                    color = tokens.muted,
                )
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End)) {
                    HqButton("Cancel", onClick = { confirming = null }, width = 112.dp)
                    HqButton("Unpair", onClick = {
                        confirming = null
                        unpair(computer)
                    }, danger = true, width = 112.dp)
                }
            }
        }
    }

    Column(Modifier.fillMaxSize().statusBarsPadding().navigationBarsPadding()) {
        Crumb("Settings", null, onBack)
        Column(
            Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 12.dp, vertical = 4.dp),
        ) {
            note?.let { Banner(it, info = true) }
            SectionLabel("Computers")
            Card {
                computers.forEach { computer ->
                    ComputerRow(computer, if (computer.selected) connection(status, path) else null) {
                        confirming = computer
                    }
                }
                SettingRow(
                    "Pair another computer",
                    null,
                    accent = true,
                    icon = R.drawable.ic_add,
                    onClick = { go(Route.Pair) },
                )
            }
            SectionLabel("Security")
            Card {
                val available = core.lockAvailable()
                SettingRow(
                    "Unlock with fingerprint or face",
                    if (available) {
                        "Asks when the app opens or comes back after a minute away."
                    } else {
                        "Set a screen lock on the phone first."
                    },
                ) {
                    Switch(
                        checked = appLock,
                        enabled = available,
                        onCheckedChange = {
                            appLock = it
                            core.setAppLock(it)
                        },
                    )
                }
            }
            SectionLabel("Look")
            Card {
                ThemeChoice.entries.forEach { choice ->
                    val label =
                        when (choice) {
                            ThemeChoice.System -> "As the phone"
                            ThemeChoice.Light -> "Light"
                            ThemeChoice.Dark -> "Dark"
                        }
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .heightIn(min = 52.dp)
                            .clickable(role = Role.RadioButton) { onTheme(choice) }
                            .padding(horizontal = 14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        Radio(choice == theme)
                        Text(label, fontSize = 15.sp)
                    }
                }
            }
            SectionLabel("Relay")
            Card {
                val relay = computers.firstOrNull { it.selected }?.relay
                SettingRow(
                    relay?.removePrefix("https://")?.trimEnd('/', '.') ?: "No relay",
                    "The computer's relay, used when the phone is on another network. Set on the computer.",
                )
            }
            SectionLabel("Connection")
            Card {
                SettingRow(
                    "Connected only while open",
                    "The app disconnects 30 seconds after you leave it, to save data and battery.",
                )
            }
            SectionLabel("Updates")
            Card {
                SettingRow(
                    "Tell me about new versions",
                    "Checks GitHub once a day while the app is open. Version ${BuildConfig.VERSION_NAME}.",
                ) {
                    Switch(
                        checked = updateCheck,
                        onCheckedChange = {
                            updateCheck = it
                            core.settings.updateCheck = it
                        },
                    )
                }
            }
        }
    }
}

private fun connection(
    status: LinkStatus,
    path: String?,
): String =
    when (status) {
        LinkStatus.Connected -> "Connected ${path ?: ""}".trim()
        LinkStatus.Connecting -> "Connecting…"
        is LinkStatus.Reconnecting -> "Reconnecting…"
        LinkStatus.NotKnown -> "No longer knows this phone"
        is LinkStatus.ComputerTooOld -> "Needs an update"
        is LinkStatus.AppTooOld -> "Newer than this app"
        LinkStatus.Off, LinkStatus.NoComputer -> "Not connected"
    }

@Composable
private fun ComputerRow(
    computer: ComputerInfo,
    connection: String?,
    onUnpair: () -> Unit,
) {
    val tokens = LocalTokens.current
    Row(
        Modifier.fillMaxWidth().heightIn(min = 60.dp).padding(horizontal = 14.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Dot(connection?.startsWith("Connected") == true)
        Column(Modifier.weight(1f)) {
            Text(computer.name, fontSize = 15.sp, fontWeight = FontWeight.Medium)
            Text(connection ?: seenText(computer.lastSeenAt), color = tokens.muted, fontSize = 12.5.sp)
        }
        RowPill("Unpair", color = tokens.danger, outline = tokens.line, onClick = onUnpair)
    }
}

@Composable
private fun SettingRow(
    title: String,
    detail: String?,
    accent: Boolean = false,
    @DrawableRes icon: Int? = null,
    onClick: (() -> Unit)? = null,
    trailing: @Composable () -> Unit = {},
) {
    val tokens = LocalTokens.current
    val base = Modifier.fillMaxWidth().heightIn(min = 52.dp)
    Row(
        (if (onClick != null) base.clickable(role = Role.Button, onClick = onClick) else base)
            .background(tokens.card)
            .padding(horizontal = 14.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        icon?.let { Icon(painterResource(it), null, tint = tokens.accent, modifier = Modifier.size(20.dp)) }
        Column(Modifier.weight(1f)) {
            Text(
                title,
                fontSize = 15.sp,
                color = if (accent) tokens.accent else tokens.text,
                fontWeight = if (accent) FontWeight.SemiBold else FontWeight.Medium,
            )
            if (detail != null) Text(detail, color = tokens.muted, fontSize = 12.5.sp)
        }
        trailing()
    }
}
