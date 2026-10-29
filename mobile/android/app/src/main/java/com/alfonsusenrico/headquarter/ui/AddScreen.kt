package com.alfonsusenrico.headquarter.ui

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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.alfonsusenrico.headquarter.Core
import com.alfonsusenrico.headquarter.R
import uniffi.hq_mobile.MobileException

/** Past conversations the computer knows, to put in the sidebar. New sessions start only there. */
@Composable
fun AddScreen(
    core: Core,
    version: ULong,
    onBack: () -> Unit,
) {
    val tokens = LocalTokens.current
    val past = remember(version) { core.phone.pastConversations() }
    var query by remember { mutableStateOf("") }
    val added = remember { mutableStateListOf<String>() }
    var problem by remember { mutableStateOf<String?>(null) }
    val shown =
        past.filter {
            query.isBlank() || it.title.contains(query, ignoreCase = true) ||
                it.folder.contains(query, ignoreCase = true)
        }
    Column(Modifier.fillMaxSize().statusBarsPadding().navigationBarsPadding()) {
        Crumb("Add sessions", "Past conversations on your computer", onBack)
        Row(
            Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp)
                .heightIn(min = 48.dp)
                .clip(CircleShape)
                .background(tokens.card)
                .border(1.dp, tokens.line, CircleShape)
                .padding(horizontal = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Icon(painterResource(R.drawable.ic_search), null, tint = tokens.muted, modifier = Modifier.size(20.dp))
            Box(Modifier.weight(1f)) {
                if (query.isEmpty()) Text("Find a conversation or folder…", color = tokens.muted)
                BasicTextField(
                    value = query,
                    onValueChange = { query = it },
                    singleLine = true,
                    textStyle = TextStyle(color = tokens.text, fontSize = 15.sp),
                    cursorBrush = SolidColor(tokens.accent),
                    modifier =
                        Modifier.fillMaxWidth().semantics {
                            contentDescription = "Find a conversation or folder"
                        },
                )
            }
        }
        problem?.let { Banner(it, info = false) }
        LazyColumn(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
            shown.groupBy { it.folder }.forEach { (folder, conversations) ->
                item(key = "folder-$folder") { SectionLabel(folder) }
                item(key = "card-$folder") {
                    Card {
                        conversations.forEach { conversation ->
                            Row(
                                Modifier
                                    .fillMaxWidth()
                                    .heightIn(
                                        min = 56.dp,
                                    ).padding(horizontal = 14.dp, vertical = 8.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(10.dp),
                            ) {
                                AgentMark(conversation.agent)
                                Column(Modifier.weight(1f)) {
                                    Text(
                                        conversation.title,
                                        fontSize = 15.sp,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                    Text(
                                        "${agentName(conversation.agent)} · ${conversation.age}",
                                        color = tokens.muted,
                                        fontSize = 12.5.sp,
                                    )
                                }
                                val done = conversation.id in added
                                RowPill(
                                    if (done) "Added" else "Add",
                                    color = if (done) tokens.muted else tokens.accent,
                                    outline = if (done) tokens.line else tokens.accent,
                                    enabled = !done,
                                ) {
                                    added += conversation.id
                                    core.io {
                                        try {
                                            core.phone.add(conversation.id)
                                        } catch (error: MobileException) {
                                            added -= conversation.id
                                            problem = error.ownerText()
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            item {
                Text(
                    if (past.isEmpty()) "No past conversations to add." else "New sessions start only on the computer.",
                    color = tokens.muted,
                    fontSize = 12.5.sp,
                    modifier = Modifier.padding(4.dp),
                )
            }
        }
    }
}
