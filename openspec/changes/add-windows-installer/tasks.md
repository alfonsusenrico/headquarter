# Tasks

## 1. Build

- [ ] 1.1 `packaging/windows/headquarter.iss` (D1). Evidence: compiled by `ISCC` in the job (1.2).
- [ ] 1.2 `build-windows-installer.yml` and `dist-workspace.toml`, `release.yml` regenerated with
  `dist generate` (D2, D3). Evidence: `dist plan` lists the job; the `-rc.1` run (2.2).
- [ ] 1.3 README's Windows section, `AGENTS.md` release kind and verify list (D4). Evidence: review
  of the diff.

Expected Result: a release carries a Windows setup that installs per user with a Start menu entry.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check`; the Windows check passes on the pull request.
- [ ] 2.2 After the owner accepts the plan's result on review: archive, push
  `feat/windows-installer`, open the PR, report its URL.
- [ ] 2.3 After the owner merges (recorded in `PROJECT_STATE.md`): `v0.4.0-rc.1`, because the
  release pipeline changed. Evidence: every job succeeds; the setup and its checksum are on the
  release and in `sha256.sum`; the D2 and D3 outcomes are recorded.
- [ ] 2.4 The owner on the Windows PC, with the `-rc.1` setup: install, Start menu, open, update
  over an older setup, uninstall, data kept.
