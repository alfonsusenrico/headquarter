package com.alfonsusenrico.headquarter.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** Shown until the phone confirms the owner (add-android-app D9). Nothing connects meanwhile. */
@Composable
fun LockScreen(onUnlock: () -> Unit) {
    LaunchedEffect(Unit) { onUnlock() }
    Column(
        Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp, Alignment.CenterVertically),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("🔒", fontSize = 44.sp)
        Text("Headquarter is locked", fontSize = 19.sp, fontWeight = FontWeight.SemiBold)
        Text(
            "Unlock with your fingerprint, face or phone PIN. A paired phone can type into your agents, so the app locks when you leave it.",
            color = LocalTokens.current.muted,
            textAlign = TextAlign.Center,
        )
        HqButton("Unlock", onClick = onUnlock, primary = true)
    }
}
