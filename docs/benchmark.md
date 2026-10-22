# Headquarter against Alacritty: CPU and memory

Measured on 2026-10-22 on an Apple silicon Mac with `scripts/bench/suite.sh`: three
rounds, the median per case. Each case runs N sessions, waits 10 s, then measures 30 s
(60 s for real agents) with `scripts/bench/measure.py`.

- **Headquarter** is the app, its host and the host's helpers together; **Alacritty** is
  one window per session, all of them. The agents' own processes are left out on both
  sides.
- The display stayed on throughout: a sleeping display stops drawing and voids the
  numbers.
- **Loads:**
  - a stand-in agent, idle or working (a spinner every 100 ms and a line a second);
  - the real agents, idle;
  - recordings of real Claude Code, Codex and agy sessions, replayed at their recorded
    speed.
- Headquarter shows one session at a time and draws only that one; Alacritty draws
  every window.

| Load | Sessions | Headquarter CPU | Alacritty CPU | Headquarter memory | Alacritty memory | Headquarter wake-ups/s | Alacritty wake-ups/s | Rounds |
|---|---|---|---|---|---|---|---|---|
| stand-in, working | 1 | 8.9% | 2.7% | 177 MB | 184 MB | 129  | 2  | 3 |
| stand-in, working | 10 | 10.1% | 14.0% | 173 MB | 1,249 MB | 106  | 17  | 3 |
| stand-in, working | 25 | 12.6% | 20.2% | 168 MB | 3,147 MB | 130  | 47  | 3 |
| stand-in, working | 50 | 14.8% | 23.1% | 170 MB | 6,251 MB | 125  | 87  | 3 |
| stand-in, idle | 1 | 0.6% | 0.0% | 122 MB | 94 MB | 19  | 1  | 3 |
| stand-in, idle | 10 | 0.8% | 0.1% | 114 MB | 517 MB | 14  | 6  | 3 |
| stand-in, idle | 25 | 0.7% | 0.2% | 141 MB | 1,227 MB | 13  | 15  | 3 |
| stand-in, idle | 50 | 0.6% | 0.5% | 116 MB | 2,696 MB | 14  | 30  | 3 |
| real agy, idle | 3 | 0.8% | 0.0% | 120 MB | 221 MB | 14  | 2  | 3 |
| real claude, idle | 3 | 0.8% | 0.0% | 116 MB | 222 MB | 19  | 2  | 3 |
| real codex, idle | 3 | 0.8% | 0.0% | 124 MB | 228 MB | 14  | 2  | 3 |
| agy recording, working | 1 | 10.8% | 2.9% | 174 MB | 195 MB | 131  | 2  | 3 |
| agy recording, working | 3 | 10.5% | 6.6% | 159 MB | 422 MB | 131  | 6  | 3 |
| agy recording, working | 10 | 11.2% | 15.6% | 163 MB | 1,247 MB | 134  | 20  | 3 |
| claude recording, working | 1 | 5.2% | 3.6% | 165 MB | 197 MB | 110  | 4  | 3 |
| claude recording, working | 3 | 5.3% | 8.4% | 160 MB | 429 MB | 104  | 11  | 3 |
| claude recording, working | 10 | 4.4% | 16.9% | 161 MB | 1,264 MB | 84  | 35  | 3 |
| codex recording, working | 1 | 12.3% | 11.8% | 162 MB | 209 MB | 129  | 20  | 3 |
| codex recording, working | 3 | 12.4% | 28.0% | 157 MB | 459 MB | 122  | 57  | 3 |
| codex recording, working | 10 | 14.2% | 40.2% | 161 MB | 1,346 MB | 130  | 220  | 3 |

## What it shows

- **Memory:** Headquarter stays at 115 to 180 MB whatever the session count. Alacritty
  uses about 95 MB for one idle window and grows with every window: about 220 MB for 3,
  1.2 to 1.3 GB for 10, and 2.7 GB idle or 6.3 GB working for 50.
- **CPU while agents work:**
  - Alacritty is cheaper for a single session: 2.7% against 8.9% for the stand-in, and
    Codex is even at about 12%.
  - Headquarter is cheaper from 3 sessions for the Claude Code and Codex recordings,
    and from 10 for agy and the stand-in. Its cost stays nearly flat, while Alacritty's
    grows with every busy window.
- **CPU at idle:** Alacritty is near 0%; Headquarter uses 0.6 to 0.8%. Its app redraws
  status, and its host looks for agents running in other terminals every 5 s while the
  window is shown, and once a minute while it is hidden.
- **Variation:** Alacritty's figures vary more between rounds at high session counts.

To run it yourself, record the agents first with `scripts/bench/record.sh`, then run
`scripts/bench/suite.sh` and `scripts/bench/summarize.py` (see `scripts/bench/README.md`).
