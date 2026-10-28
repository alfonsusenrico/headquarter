package com.alfonsusenrico.headquarter

import android.os.Build
import android.os.Bundle
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.biometric.BiometricManager.Authenticators
import androidx.biometric.BiometricPrompt
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.alfonsusenrico.headquarter.ui.AddScreen
import com.alfonsusenrico.headquarter.ui.HeadquarterTheme
import com.alfonsusenrico.headquarter.ui.HomeScreen
import com.alfonsusenrico.headquarter.ui.InactiveScreen
import com.alfonsusenrico.headquarter.ui.LocalTokens
import com.alfonsusenrico.headquarter.ui.LockScreen
import com.alfonsusenrico.headquarter.ui.PairScreen
import com.alfonsusenrico.headquarter.ui.SessionScreen
import com.alfonsusenrico.headquarter.ui.SettingsScreen
import uniffi.hq_mobile.LinkStatus

/** Where the owner is in the app. */
sealed interface Route {
    data object Home : Route

    data object Pair : Route

    data class Session(
        val id: String,
    ) : Route

    data class Shell(
        val id: String,
    ) : Route

    data class Inactive(
        val id: String,
    ) : Route

    data object Add : Route

    data object Settings : Route
}

class MainActivity : FragmentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val core = (application as HeadquarterApp).core
        setContent { App(core, ::unlock) }
    }

    /** Asks for a fingerprint, face or the phone's PIN (add-android-app D9). */
    private fun unlock(core: Core) {
        val allowed =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                Authenticators.BIOMETRIC_STRONG or Authenticators.DEVICE_CREDENTIAL
            } else {
                // Android 10 pairs the device credential only with weak biometrics.
                Authenticators.BIOMETRIC_WEAK or Authenticators.DEVICE_CREDENTIAL
            }
        val prompt =
            BiometricPrompt(
                this,
                ContextCompat.getMainExecutor(this),
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                        core.unlocked()
                    }
                },
            )
        prompt.authenticate(
            BiometricPrompt.PromptInfo
                .Builder()
                .setTitle("Unlock Headquarter")
                .setAllowedAuthenticators(allowed)
                .build(),
        )
    }
}

@Composable
private fun App(
    core: Core,
    unlock: (Core) -> Unit,
) {
    var theme by remember { mutableStateOf(core.settings.theme) }
    val version by core.version.collectAsStateWithLifecycle()
    val locked by core.locked.collectAsStateWithLifecycle()
    HeadquarterTheme(theme) {
        Box(Modifier.fillMaxSize().background(LocalTokens.current.page)) {
            if (locked) {
                LockScreen(onUnlock = { unlock(core) })
                return@Box
            }
            var stack by remember { mutableStateOf(listOf<Route>(Route.Home)) }
            val status = remember(version) { core.phone.status() }
            val go: (Route) -> Unit = { stack = stack + it }
            val back: () -> Unit = { if (stack.size > 1) stack = stack.dropLast(1) }
            val home: () -> Unit = { stack = listOf(Route.Home) }
            BackHandler(enabled = stack.size > 1, onBack = back)
            // With no computer paired, pairing is all there is (first launch).
            val route = if (status is LinkStatus.NoComputer && stack.last() != Route.Pair) Route.Pair else stack.last()
            when (route) {
                Route.Home -> {
                    HomeScreen(core, version, go)
                }

                Route.Pair -> {
                    PairScreen(core, canGoBack = status !is LinkStatus.NoComputer, onPaired = home, onBack = back)
                }

                is Route.Session -> {
                    SessionScreen(core, version, route.id, shell = false, go = go, onBack = back, onTaken = home)
                }

                is Route.Shell -> {
                    SessionScreen(core, version, route.id, shell = true, go = go, onBack = back, onTaken = home)
                }

                is Route.Inactive -> {
                    InactiveScreen(core, version, route.id, onResumed = {
                        stack =
                            listOf(Route.Home, Route.Session(route.id))
                    }, onBack = back)
                }

                Route.Add -> {
                    AddScreen(core, version, onBack = back)
                }

                Route.Settings -> {
                    SettingsScreen(
                        core = core,
                        version = version,
                        theme = theme,
                        onTheme = {
                            theme = it
                            core.settings.theme = it
                        },
                        go = go,
                        onBack = back,
                    )
                }
            }
        }
    }
}
