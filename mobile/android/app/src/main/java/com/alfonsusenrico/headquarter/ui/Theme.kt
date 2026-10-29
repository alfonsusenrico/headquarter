package com.alfonsusenrico.headquarter.ui

import android.app.Activity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.core.view.WindowCompat
import com.alfonsusenrico.headquarter.R
import com.alfonsusenrico.headquarter.ThemeChoice

/** The desktop's tokens (crates/hq-app/src/theme.rs), light and dark (add-android-app D11). */
@Immutable
data class Tokens(
    val dark: Boolean,
    val page: Color,
    val line: Color,
    val card: Color,
    val text: Color,
    val muted: Color,
    val group: Color,
    val amber: Color,
    val glyphAmber: Color,
    val track: Color,
    val accent: Color,
    val teal: Color,
    val violet: Color,
    val accentSoft: Color,
    val warnSoft: Color,
    val danger: Color,
    val ok: Color,
    val terminalForeground: Color,
    val terminalBackground: Color,
    val ansi: List<Color>,
    val groups: Map<String, Color>,
)

private fun rgb(hex: Long) = Color(0xFF000000 or hex)

val LightTokens =
    Tokens(
        dark = false,
        page = rgb(0xECF0F7),
        line = rgb(0xD5DBE6),
        card = rgb(0xFFFFFF),
        text = rgb(0x1D2433),
        muted = rgb(0x5B6475),
        group = rgb(0x4A5468),
        amber = rgb(0x9A4A06),
        glyphAmber = rgb(0xC26A10),
        track = rgb(0xC9CFDA),
        accent = rgb(0x2457C5),
        teal = rgb(0x0E6A5A),
        violet = rgb(0x6B3FC2),
        accentSoft = rgb(0xDCE6FA),
        warnSoft = rgb(0xF6E7D6),
        danger = rgb(0xB42318),
        ok = rgb(0x15703D),
        terminalForeground = rgb(0x1D2433),
        terminalBackground = rgb(0xFFFFFF),
        ansi =
            listOf(
                0x1D2433,
                0xB42318,
                0x15703D,
                0x855400,
                0x2457C5,
                0x8E3EA8,
                0x0E6A80,
                0xE4E8EF,
                0x4A5468,
                0xC0261B,
                0x167A43,
                0x7A4D00,
                0x1F4FB8,
                0x9B3FB8,
                0x0B6377,
                0xF4F6FA,
            ).map { rgb(it.toLong()) },
        groups =
            mapOf(
                "mauve" to rgb(0x8839EF),
                "green" to rgb(0x2E8B2B),
                "teal" to rgb(0x13868A),
                "peach" to rgb(0xC8580F),
                "blue" to rgb(0x1E66F5),
                "pink" to rgb(0xB8458F),
                "yellow" to rgb(0x9A6A0C),
                "red" to rgb(0xD20F39),
            ),
    )

val DarkTokens =
    Tokens(
        dark = true,
        page = rgb(0x171A21),
        line = rgb(0x262B35),
        card = rgb(0x252A34),
        text = rgb(0xE6E8EC),
        muted = rgb(0x9AA3B2),
        group = rgb(0xB4BCC9),
        amber = rgb(0xF5A524),
        glyphAmber = rgb(0xF5A524),
        track = rgb(0x3A404C),
        accent = rgb(0x8AB4FF),
        teal = rgb(0x67E8C9),
        violet = rgb(0xC4A7FF),
        accentSoft = rgb(0x1E2A44),
        warnSoft = rgb(0x3A2C14),
        danger = rgb(0xF87171),
        ok = rgb(0x4ADE80),
        terminalForeground = rgb(0xE6E8EC),
        terminalBackground = rgb(0x0F1115),
        ansi =
            listOf(
                0x2A2F3A,
                0xF87171,
                0x4ADE80,
                0xFACC15,
                0x8AB4FF,
                0xD8B4FE,
                0x67E8F9,
                0xD5DAE3,
                0x8A93A3,
                0xFCA5A5,
                0x86EFAC,
                0xFDE68A,
                0xB4CDFF,
                0xE9D5FF,
                0xA5F3FC,
                0xF5F7FA,
            ).map { rgb(it.toLong()) },
        groups =
            mapOf(
                "mauve" to rgb(0xCBA6F7),
                "green" to rgb(0xA6E3A1),
                "teal" to rgb(0x94E2D5),
                "peach" to rgb(0xFAB387),
                "blue" to rgb(0x89B4FA),
                "pink" to rgb(0xF5C2E7),
                "yellow" to rgb(0xF9E2AF),
                "red" to rgb(0xF38BA8),
            ),
    )

val LocalTokens = staticCompositionLocalOf { LightTokens }

/** JetBrains Mono without ligatures: a terminal draws `->` as two characters. */
val Mono =
    FontFamily(
        Font(R.font.jetbrains_mono_nl_regular, FontWeight.Normal),
        Font(R.font.jetbrains_mono_nl_bold, FontWeight.Bold),
        Font(R.font.jetbrains_mono_nl_italic, FontWeight.Normal, FontStyle.Italic),
        Font(R.font.jetbrains_mono_nl_bold_italic, FontWeight.Bold, FontStyle.Italic),
    )

@Composable
fun HeadquarterTheme(
    choice: ThemeChoice,
    content: @Composable () -> Unit,
) {
    val dark =
        when (choice) {
            ThemeChoice.System -> isSystemInDarkTheme()
            ThemeChoice.Light -> false
            ThemeChoice.Dark -> true
        }
    val tokens = if (dark) DarkTokens else LightTokens
    val scheme =
        if (dark) {
            darkColorScheme(
                primary = tokens.accent,
                onPrimary = tokens.terminalBackground,
                background = tokens.page,
                onBackground = tokens.text,
                surface = tokens.page,
                onSurface = tokens.text,
                surfaceVariant = tokens.card,
                onSurfaceVariant = tokens.muted,
                outline = tokens.line,
                error = tokens.danger,
            )
        } else {
            lightColorScheme(
                primary = tokens.accent,
                onPrimary = tokens.card,
                background = tokens.page,
                onBackground = tokens.text,
                surface = tokens.page,
                onSurface = tokens.text,
                surfaceVariant = tokens.card,
                onSurfaceVariant = tokens.muted,
                outline = tokens.line,
                error = tokens.danger,
            )
        }
    val view = LocalView.current
    if (!view.isInEditMode) {
        // The bars follow the app's choice, not the phone's, so their icons stay readable.
        SideEffect {
            val window = (view.context as Activity).window
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !dark
                isAppearanceLightNavigationBars = !dark
            }
        }
    }
    CompositionLocalProvider(LocalTokens provides tokens) {
        MaterialTheme(colorScheme = scheme) {
            // Only a Surface sets the text colour, and the screens draw on a plain Box.
            CompositionLocalProvider(LocalContentColor provides tokens.text, content = content)
        }
    }
}
