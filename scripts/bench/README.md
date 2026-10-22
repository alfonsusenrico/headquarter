# Resource benchmark

Measures Headquarter (the app, its host and the host's helpers) against Alacritty with
one window per session, the way agents are often tiled, on macOS. It reads each
process's own counters (`proc_pid_rusage`): CPU time, idle wake-ups and physical
footprint. The agents' own processes are left out on both sides, since they are the
same either way.

## Loads

- **Stand-in, idle or working:** `stand-in` idles, or works like an agent (a spinner
  every 100 ms, a line a second). Costs no tokens.
- **Recording, working:** `record.sh AGENT` records one real session doing about 90 s of
  work in one short turn; `stand-in` then replays that output at its recorded timing, so
  both sides draw exactly the same real output in every round.
- **Real agents, idle:** the real CLIs started without a prompt; no tokens.

In Headquarter one session is shown, as in use; in Alacritty every window draws.

## Running

```bash
cargo build --release -p hq-cli -p hq-app
scripts/bench/record.sh claude    # one short turn each; recordings stay in $BENCH_DIR
scripts/bench/record.sh codex
scripts/bench/record.sh agy
scripts/bench/suite.sh            # three rounds, about 30 minutes each
python3 scripts/bench/summarize.py $BENCH_DIR/round-*.log
```

`BENCH_DIR` defaults to `/tmp/hq-bench`. `BENCH_WORK` is the agents' working folder; use
a folder every agent already trusts, or their first-run trust questions hold them.
Windows open and close while it runs; leave the machine alone, since Headquarter's
session is shown with key presses. Recordings show the agent's screen, account name
included: keep them out of the repository.
