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
- **WHEN** quotas are requested for a provider whose quota is only available through its own configuration files
- **THEN** that provider's quota is reported as unavailable with that reason

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
The system SHALL show, under the sidebar, centred rings for the enabled agents nearest their limits, as many as fit on one line, each filled to the share used of its tightest current window, SHALL fold the rest into a "+N" chip that opens every agent's usage, and SHALL name the agent nearest its limit when one has used at least half of a window.

#### Scenario: Three agents
- **WHEN** Claude, Codex and agy are enabled and Codex has used 82% of its five-hour window
- **THEN** three centred rings are shown, Codex's first and filled to 82%, agy's dotted, and the label reads "codex 82% · 5h"

#### Scenario: Narrow sidebar
- **WHEN** the rings do not fit on one line
- **THEN** the agents furthest from their limits fold into a "+N" chip, and every agent stays reachable from it

### Requirement: Usage details
The system SHALL show, when the owner opens an agent's ring, each window with its percentage used and its reset time in local 12-hour time, including per-model windows the agent reports, and SHALL let the owner refresh the reading. Clicking the ring that opened the panel, pressing Escape, or clicking outside SHALL close it.

#### Scenario: Codex details
- **WHEN** the owner opens the Codex ring
- **THEN** the popover lists the five-hour and weekly windows with bars, percentages and reset times, and a Refresh button, with no reading age

#### Scenario: Refresh
- **WHEN** the owner presses Refresh
- **THEN** the reading is taken again and the popover updates without being reopened

#### Scenario: Unavailable agent
- **WHEN** the owner opens agy's ring
- **THEN** the popover states why its usage cannot be read and shows no percentage

#### Scenario: Closed by its own ring
- **WHEN** Claude's usage panel is open and the owner clicks Claude's ring again
- **THEN** the panel closes

#### Scenario: Per-model week
- **WHEN** Claude's `/usage` shows "Current week (Fable)" at 3%
- **THEN** Claude's details list "week · Fable" at 3% below the all-models week

### Requirement: Reading cadence
The system SHALL read Codex usage 10 seconds after a hosted Codex session's turn ends, every five minutes while a client shows usage, and on refresh; SHALL take Claude usage from each report of a hosted Claude session and with a hidden `/usage` read on refresh, every ten minutes while a client shows usage and no hosted Claude session has reported, and after a Claude session in another terminal finishes a turn, at most every two minutes; and SHALL NOT read either when no client is showing usage.

#### Scenario: App closed
- **WHEN** no app window is open
- **THEN** no Codex reading and no hidden Claude read is started

#### Scenario: Codex turn ends
- **WHEN** a hosted Codex session finishes a turn while the app is open
- **THEN** Codex usage is read again within about 10 seconds

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
