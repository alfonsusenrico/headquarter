# Tasks

## 1. Build

- [ ] 1.1 `process::quiet` with `CREATE_NO_WINDOW` on Windows; `marks.rs` and `update.rs` use it (D1).
  Evidence: a unit test that no `Command::new` is left in `hq-app` outside the helper; the Windows
  check builds it.
- [ ] 1.2 `headquarter.ico` from the SVG and `build.rs` with `winresource` on Windows (D2). Evidence:
  the Windows check builds; the built executable carries an icon resource (checked in the
  workflow's log or the artifact).
- [ ] 1.3 Caption buttons and drag area on Windows (D3). Evidence: the Windows check builds; macOS
  on screen unchanged.

Expected Result: on Windows the app opens alone, with its icon and its window buttons.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check`; the Windows check passes on the pull request.
- [ ] 2.2 The owner on the Windows PC: no console windows at first open, the icon in Explorer,
  taskbar and Alt+Tab, the three buttons with snap layouts, dragging and double-click maximize.
- [ ] 2.3 After the owner accepts: archive, push `fix/windows-app`, open the PR, report its URL.
