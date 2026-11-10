# Tasks

## 1. Build

- [x] 1.1 `packaging/windows/headquarter.iss` (D1). Evidence: compiled by `ISCC` in the job, on
  the `-rc.1` run (2.3).
- [x] 1.2 `build-windows-installer.yml`; Windows out of `dist`'s targets; `release.yml` regenerated
  with `dist generate` (D2, D3). Evidence: `release.yml` runs the job and `host` waits for it;
  `dist plan --tag v0.4.0-alpha.1` lists 20 artifacts, none for Windows; the `-rc.1` run (2.3).
- [x] 1.3 README's Windows section, `AGENTS.md` release kind and verify list (D4). Evidence: review
  of the diff.

Expected Result: a release carries a Windows setup that installs per user with a Start menu entry.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check`; the Windows check passes on the pull request.
- [ ] 2.2 After the owner accepts the result on review: archive, push
  `feat/windows-installer`, open the PR, report its URL.
- [ ] 2.3 After the owner merges (recorded in `PROJECT_STATE.md`): `v0.4.0-rc.1`, because the
  release pipeline changed. Evidence: every job succeeds; the setup and its checksum are on the
  release and in `sha256.sum`; the D2 and D3 outcomes are recorded.
- [ ] 2.4 The owner on the Windows PC, with the `-rc.1` setup: install, Start menu, open, update
  over an older setup, uninstall, data kept.
