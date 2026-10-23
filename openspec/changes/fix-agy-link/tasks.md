# Tasks

## 1. Linking

- [x] 1.1 Read `--conversation` from agy's command line; link by folder and time otherwise; leave shared folders unlinked; stop reading the busy flag. Verify with `agy_names_its_conversation_on_the_command_line` (`hq-signals`) and `agy_links_to_the_conversation_its_folder_wrote_since_it_started` (`hq-daemon`). Done (2026-10-23).
- [x] 1.2 On screen: agy in another terminal waiting for approval shows once, tagged "In another terminal". Evidence: the owner's check. Done: the owner confirmed it on the test build (`test/combined-build`), 2026-10-23.

Expected Result: an agy session elsewhere shows once, as its conversation.

## 2. Delivery

- [ ] 2.1 `./scripts/check` passes; write `verification.md`; push and open the PR after fix-title-broadcast. Check passed (377 tests) and verification written (2026-10-23).
