# provider-quota Specification

## Purpose
Shows the owner how much of each agent provider's usage allowance is used in each window (for example five hours and one week) and when each window resets.

## Requirements

### Requirement: Quota windows per provider
The system SHALL report, for each enabled provider whose quota can be read, every usage window with its length, percentage used, and reset time, and the time the reading was taken.

#### Scenario: Codex quota read
- **WHEN** the owner requests quotas and Codex is installed and signed in
- **THEN** the system reports each Codex window labelled by its duration, with percentage used and reset time

#### Scenario: Claude quota from a running session
- **WHEN** a Claude Code session launched by the system has received a response that includes rate limits
- **THEN** the system reports the five-hour and seven-day windows with percentage used and reset time

### Requirement: Unavailable quota is explicit
The system SHALL report a provider's quota as unavailable, with the reason, when it cannot be read, and SHALL NOT show estimated or stale values as current.

#### Scenario: No Claude session yet
- **WHEN** quotas are requested and no Claude reading has been captured
- **THEN** Claude's quota is reported as unavailable with the reason that no session has reported limits yet

#### Scenario: Provider exposes no quota source
- **WHEN** quotas are requested for a provider Headquarter has no way to read, such as agy when it cannot be found
- **THEN** that provider's quota is reported as "<Name> usage not available", for example "Agy usage not available"

#### Scenario: Old reading
- **WHEN** the latest reading for a provider is older than a window's reset time
- **THEN** that window is reported as stale with the reading time, not as current

### Requirement: Quota reads use documented sources without credentials
The system SHALL read quotas only through the providers' documented local interfaces, including an agent's own usage command run in a hidden session, and SHALL NOT read, copy, or transmit the owner's provider credentials.

#### Scenario: Reading Codex quota
- **WHEN** the system reads Codex quota
- **THEN** it does so through the Codex CLI's own interface and does not open the Codex credential file

#### Scenario: Hidden Claude usage read
- **WHEN** the system reads Claude's usage with a hidden session
- **THEN** it runs Claude Code, opens its `/usage` screen and reads the numbers from the screen, without opening Claude's login or keychain entry and without saving a conversation

#### Scenario: Folder trust asked
- **WHEN** the hidden Claude session asks whether to trust its folder
- **THEN** the read stops without answering and counts as a failed update

### Requirement: Usage strip
The system SHALL show, under the sidebar, centred usage marks for the enabled agents nearest their limits, as many as fit on one line, and SHALL fold the rest into a "+N" chip that opens every agent's usage.

#### Scenario: Three agents
- **WHEN** Claude, Codex and agy are enabled and Codex has used 82% of its five-hour window and 30% of its week
- **THEN** three centred marks are shown, Codex's first with its inner ring at 82% and its outer ring at 30%, agy's dotted, and the label reads "codex 82% · 5h"

#### Scenario: Narrow sidebar
- **WHEN** the rings do not fit on one line
- **THEN** the agents furthest from their limits fold into a "+N" chip, and every agent stays reachable from it

### Requirement: Usage rings
The system SHALL give each usage mark an outer ring filled to the share used of the agent's longest current window (the week) and, when the agent has two, an inner ring for its shortest (5 hours), and SHALL leave per-model windows to the details.

#### Scenario: One window
- **WHEN** an agent reports a single current window
- **THEN** its mark has only the outer ring, filled to that window

### Requirement: Usage details
The system SHALL show, when the owner opens an agent's ring, each window with its percentage used and its reset time in local 12-hour time, including per-model windows the agent reports. Clicking the ring that opened the panel, pressing Escape, or clicking outside SHALL close it.

#### Scenario: Codex details
- **WHEN** the owner opens the Codex ring
- **THEN** the popover lists the five-hour and weekly windows with bars, percentages and reset times, and a Refresh button, with no reading age

#### Scenario: Unavailable agent
- **WHEN** the owner opens agy's ring
- **THEN** the popover states why its usage cannot be read and shows no percentage

#### Scenario: Closed by its own ring
- **WHEN** Claude's usage panel is open and the owner clicks Claude's ring again
- **THEN** the panel closes

#### Scenario: Per-model week
- **WHEN** Claude's `/usage` shows "Current week (Fable)" at 3%
- **THEN** Claude's details list "week · Fable" at 3% below the all-models week

#### Scenario: Long window name without a bar
- **WHEN** agy's details show "week · Gemini" as reset since the last reading, next to "week · Gemini 4 Argon" with a bar
- **THEN** both names stay on one line, and "reset since this reading" takes the remaining width

### Requirement: Refreshing usage
The system SHALL let the owner refresh an agent's reading from its details, keeping the details open and showing that the refresh is under way until the new readings are in.

#### Scenario: Refresh
- **WHEN** the owner presses Refresh
- **THEN** the popover stays open, the button shows "Refreshing…" with a spinner until the reading is taken again, and the popover then shows the new numbers without being reopened

#### Scenario: Refresh just after a reading
- **WHEN** the owner presses Refresh within the refresh floor after a reading
- **THEN** no new reading is taken and the button returns to Refresh at once

### Requirement: Reading cadence
The system SHALL read Codex usage 10 seconds after a hosted Codex session's turn ends, every five minutes while a client shows usage, and on refresh, and SHALL NOT read any agent's usage when no client is showing usage.

#### Scenario: App closed
- **WHEN** no app window is open
- **THEN** no Codex reading, no hidden Claude read and no agy read is started

#### Scenario: Codex turn ends
- **WHEN** a hosted Codex session finishes a turn while the app is open
- **THEN** Codex usage is read again within about 10 seconds

#### Scenario: agy turn ends
- **WHEN** a hosted agy session finishes a turn while the app is open
- **THEN** agy usage is read again within about 10 seconds

#### Scenario: Read fails
- **WHEN** a Codex reading fails after an earlier one succeeded
- **THEN** the earlier numbers stay and the details say "Couldn't update since" the earlier reading's time

### Requirement: Usage from the command line
The system SHALL show the same usage and reasons with `hq usage`, as text or JSON, with no reading age and with the same "Couldn't update since" line when an update failed.

#### Scenario: hq usage
- **WHEN** the owner runs `hq usage`
- **THEN** each enabled agent is listed with its windows, percentages and reset times, or the reason its usage cannot be read

### Requirement: Account behind the limits
The system SHALL show, with each agent's usage, whether its limits belong to a subscription and which plan, an API key, or a cloud provider, when the agent reports it, and SHALL keep no other account detail.

#### Scenario: Claude on a Max subscription
- **WHEN** `claude auth status` reports a claude.ai login with subscription type "max"
- **THEN** Claude's usage details, ring tooltip and `hq usage` say "Max plan"

#### Scenario: Codex with an API key
- **WHEN** Codex's `account/read` reports an API key account
- **THEN** Codex's usage says "API key"

#### Scenario: Agent reports no account
- **WHEN** an agent offers no account information
- **THEN** its usage shows no account words

#### Scenario: Personal details dropped
- **WHEN** an account answer includes the owner's email
- **THEN** the email is not stored, logged, sent to clients or shown

### Requirement: Freshness
The system SHALL show no reading age, SHALL show "Couldn't update since" the last good reading's local time when the latest update failed or a reading is more than 15 minutes old while updates are due, and SHALL show a window that has reset since its reading as reset rather than with the old percentage.

#### Scenario: Update failed
- **WHEN** the hidden Claude read times out and the last good reading was at 3:12 PM
- **THEN** Claude's details keep the 3:12 PM numbers and show "Couldn't update since 3:12 PM" in amber

#### Scenario: Window reset since the reading
- **WHEN** Claude's five-hour window reset after its last reading
- **THEN** the five-hour window shows as reset rather than with the old percentage

### Requirement: Usage failures point at the provider
The system SHALL tell the owner, when a provider does not report its usage, which provider it was and what to try, without internal detail, SHALL record the reason in the host log, and SHALL try a failed Codex read again after a minute.

#### Scenario: Codex does not report
- **WHEN** Codex's usage read fails
- **THEN** the details read "Codex didn't report its usage. Open Codex once to check it is signed in, then refresh." and Codex is read again a minute later

#### Scenario: agy does not report
- **WHEN** agy's `/usage` read fails or times out
- **THEN** the details read "agy didn't report its usage. Open agy once to check it is signed in, then refresh."

#### Scenario: An agent that does not share its usage
- **WHEN** agy is enabled but Headquarter cannot find it
- **THEN** its usage details read "Agy usage not available", the same line any agent without a usage source gets, with no further detail

#### Scenario: Claude Code asks to trust a folder
- **WHEN** Claude Code's hidden usage read stops at its folder-trust question
- **THEN** the details say Claude Code asked to trust a folder and that opening Claude Code once and answering fixes it

### Requirement: agy usage from its own usage command
The system SHALL read agy's usage with agy's headless `/usage` command and its JSON output, run in an empty folder of Headquarter's own, SHALL report each window of each pool of models agy lists with the pool's name, and SHALL NOT change agy's settings files.

#### Scenario: agy reports two pools
- **WHEN** agy's `/usage` lists "Gemini Models" with a 5-hour window 42% used and "Claude and GPT models" with a weekly window 10% used
- **THEN** agy's details list "5h · Gemini" at 42% and "week · Claude and GPT" at 10%, with their reset times

#### Scenario: Nothing of the owner's is touched
- **WHEN** agy's usage is read
- **THEN** agy runs in Headquarter's `quota/agy-workspace` folder, no agy conversation is started, and no file under agy's settings is written by Headquarter

### Requirement: Rings for pools of models
The system SHALL fill the rings of an agent that reports only pools of models from the pool nearest its limit, by the highest whole percentage used and, on a tie, the shorter window; between equal pools, the first one the agent reports.

#### Scenario: Claude and GPT pool nearer its limit
- **WHEN** agy's Gemini pool has used 11% of its week and its Claude and GPT pool 85% of its five hours and 40% of its week
- **THEN** agy's inner ring is filled to 85% and its outer ring to 40%, both from the Claude and GPT pool, and the label reads "agy 85% · 5h · Claude and GPT"

### Requirement: Usage near its limit
The system SHALL draw a ring red when its window has used 80% or more, and SHALL, from 80%, name the one agent near its limit with its nearest window (the shorter window when the whole percentages are equal) or, when several agents are, say how many.

#### Scenario: Exactly at the threshold
- **WHEN** Codex has used exactly 80% of its week and less of its five hours, and no other agent is near its limit
- **THEN** Codex's outer ring is red and the label reads "codex 80% · week"

#### Scenario: Several agents near their limits
- **WHEN** Claude has used 91% of its week and Codex 85% of its five hours
- **THEN** both rings are red and the label reads "2 agents near their limit"

#### Scenario: One agent, both windows near the limit
- **WHEN** only Claude is near its limit, with 85% of its five hours and 92% of its week used
- **THEN** both of Claude's rings are red and the label reads "claude 92% · week"

#### Scenario: Equal shares
- **WHEN** Claude's five hours and week both show 88%
- **THEN** the label reads "claude 88% · 5h"

#### Scenario: Below the threshold
- **WHEN** no agent has used 80% of any window
- **THEN** no label is shown and no ring is red

### Requirement: Claude reading cadence
The system SHALL take Claude usage from each report of a hosted Claude session and with a hidden `/usage` read on refresh, every ten minutes while a client shows usage and no hosted Claude session has reported, and after a Claude session in another terminal finishes a turn, at most every two minutes.

#### Scenario: Hosted Claude reports
- **WHEN** a hosted Claude session reports its usage while the app is open
- **THEN** the Claude ring shows that reading and no hidden `/usage` read is started for it

#### Scenario: Turns in another terminal
- **WHEN** a Claude session in another terminal finishes two turns a minute apart while the app is open
- **THEN** Claude usage is read after the first turn and not again before two minutes have passed

### Requirement: agy reading cadence
The system SHALL read agy usage 10 seconds after a hosted agy session's turn ends, every ten minutes while a client shows usage, and on refresh.

#### Scenario: agy while usage is shown
- **WHEN** the app shows usage for twenty minutes and no hosted agy session finishes a turn
- **THEN** agy usage is read about every ten minutes

### Requirement: The hidden Claude read picks a trusted folder
The system SHALL run the hidden Claude usage read in the most recently used session folder that Claude Code records as trusted, reading only that record and never changing it, and SHALL fall back to the most recently used existing folder when the record cannot be read.

#### Scenario: Working in the home folder
- **WHEN** the owner's most recent Claude Code session runs in the home folder, which Claude Code does not keep trusted, and an earlier one ran in a trusted project folder
- **THEN** the hidden read runs in that project folder and the usage updates

#### Scenario: No trusted folder
- **WHEN** none of the sessions' folders is trusted by Claude Code
- **THEN** no hidden session starts and the details say Claude Code needs to be opened once in a project folder

### Requirement: Refresh after a failed reading
A refresh SHALL read an agent again at once when its last reading failed, even within the refresh floor.

#### Scenario: Refresh twice after a failure
- **WHEN** a Claude reading fails and the owner presses Refresh 10 seconds later
- **THEN** a new reading is taken and the button shows "Refreshing…" until it is in
