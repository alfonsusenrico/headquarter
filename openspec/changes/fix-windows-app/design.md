# Design: the Windows app's window

## Context

`main.rs` already builds the app as a windowed program on Windows (`windows_subsystem =
"windows"`), so the app itself opens no console. Child processes started with
`std::process::Command` from a windowed program get a new console unless the creation flags say
otherwise. On Windows, the app starts `curl.exe` (shipped with Windows 10 and later) for the marks
and for the update check. `hostname` is not started there because `COMPUTERNAME` is set.

GPUI 0.3.9 lets an element declare itself the window's `Min`, `Max`, `Close` or `Drag` area
(`window_control_area`). Its Windows platform answers `WM_NCHITTEST` with `HTMINBUTTON`,
`HTMAXBUTTON`, `HTCLOSE` and `HTCAPTION` for them, so Windows treats them as real caption
buttons.

## Decisions

### D1. One helper for helper processes

`crate::process::quiet(program) -> Command` returns a `Command` with `CREATE_NO_WINDOW`
(`0x0800_0000`) on Windows and unchanged elsewhere. `marks.rs` and `update.rs` use it. A test fails
if `Command::new` appears in `hq-app` outside that helper, so a later call cannot bring the windows
back.

Rejected: `DETACHED_PROCESS`. It gives the child no console at all, and some console programs fail
to start without one. `CREATE_NO_WINDOW` gives a hidden one.

### D2. The icon is a build resource

`assets/icon/headquarter.ico` packs the existing renders in `assets/icon/png` (16, 32, 64, 128
and 256 px, PNG-compressed entries, which Windows Vista and later read) and is committed, so the
build needs no image tool. `crates/hq-app/build.rs`
embeds it with `winresource` as resource 1 only when the target is Windows. GPUI loads resource 1
for the window, so the window, the taskbar and Explorer show the same icon.

### D3. Windows 11 caption buttons

The owner chose the Windows 11 style (2026-11-10). On Windows only:
- Three buttons at the window's top right, each 46 px wide and as tall as the pane header (38 px),
  so they line up with it: minimize (─), maximize (☐), or restore (two squares) while maximized,
  and close (✕). They are drawn over everything, as the system's title bar would be.
- The glyphs use Segoe Fluent Icons where present, else Segoe MDL2 Assets, else plain shapes.
- Hover uses the theme's hover colour, and close turns `#C42B1C` with a white glyph.
- Each button is marked as its window control area. GPUI's Windows platform answers the hit test
  and acts on the click itself (minimize, maximize or restore, and close through the app's own
  close handling).
- The sidebar's top strip, which already moves the window, is the `Drag` area on Windows, so
  double-clicking it maximizes and dragging it to an edge snaps. The pane header is not: it holds
  clickable items (the context meter, the folder), which a drag area would swallow.
- The pane header leaves 138 px on the right for the buttons.

macOS keeps its traffic lights, and Linux is unchanged.

Rejected: turning the system title bar back on for Windows. It would add a second bar above the
app's own header and break the look each theme is designed for.

## Risks

- Fonts: Segoe Fluent Icons comes only with Windows 11. The fallbacks keep the buttons drawn on
  Windows 10.
- Checking on screen needs the owner's Windows PC. The Windows check proves it builds, not how it
  looks.
