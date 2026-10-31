# Scrolling full-screen agents on the phone

Status: approved by the owner 2026-10-31 ("ok let's do it", with fix-claude-usage-folder and fix-false-done).

## Why

The owner (2026-10-29, phone app 0.1.20): the session's terminal does not scroll on the phone.
Claude Code 2.1.312 draws on the alternate screen and asks for mouse reports, so the terminal keeps
no history; the phone's drag only moves its own history, while the desktop sends the wheel to the
agent (`research/evidence/phone-scroll/findings.md`).

## Expected Outcome

- Dragging the terminal on the phone scrolls Claude Code's and Codex's conversation, as the mouse
  wheel does on the desktop.
- Agents that print inline keep scrolling the phone's own history, with the fade and Live button.

## What Changes

- Phone core: a scroll goes the desktop's way: wheel reports when the agent asked for mouse
  reports, arrow keys on the alternate screen without them, its own history otherwise (D1).
- Android app: the drag passes where it is, so the wheel report names that cell (D2).
- `android-app` spec: scrolling a full-screen agent.

## Non-goals

- A Live button for full-screen agents: they keep their own place and follow new output themselves.
- Kinetic (fling) scrolling.
- The desktop: it already does this.

## Capabilities

No new project capability. Modified: `android-app`.

## Impact

`hq-mobile` (`terminal.rs`, `lib.rs` and its UniFFI binding), `TerminalView.kt`,
`SessionScreen.kt`. No protocol or host change. Checked against Claude Code 2.1.312.
