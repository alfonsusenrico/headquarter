"""Turns suite.sh's round logs into one table: the median over the rounds of
Headquarter (app, host and the host's helpers together) against Alacritty.
Usage: summarize.py round-1.log round-2.log ..."""

import re
import statistics
import sys
from collections import defaultdict

LINE = re.compile(
    r"^(hq|alacritty) (\S+) n=(\d+)\s+(\w+)\s+x\d+\s+cpu\s+([\d.]+)%\s+"
    r"wakeups/s\s+([\d.]+)\s+mem\s+(\d+) MB"
)
LOADS = {"idle": "stand-in, idle", "busy": "stand-in, working"}


def describe(load):
    kind, _, agent = load.partition(":")
    if kind == "replay":
        return f"{agent} recording, working"
    if kind == "real":
        return f"real {agent}, idle"
    return LOADS.get(load, load)


# (mode, load, n) -> one [cpu, wakeups, memory] total per round
rounds = defaultdict(list)
for path in sys.argv[1:]:
    totals = defaultdict(lambda: [0.0, 0.0, 0.0])
    for text in open(path):
        match = LINE.match(text)
        if not match:
            continue
        mode, load, n, _group, cpu, wakeups, memory = match.groups()
        total = totals[(mode, load, int(n))]
        total[0] += float(cpu)
        total[1] += float(wakeups)
        total[2] += float(memory)
    for key, total in totals.items():
        rounds[key].append(total)


def median(key, index):
    values = [total[index] for total in rounds.get(key, [])]
    return statistics.median(values) if values else None


def cell(value, unit):
    if value is None:
        return "–"
    return f"{value:.1f}%" if unit == "%" else f"{value:,.0f} {unit}"


cases = sorted({(load, n) for (_mode, load, n) in rounds})
print("| Load | Sessions | Headquarter CPU | Alacritty CPU | Headquarter memory | "
      "Alacritty memory | Headquarter wake-ups/s | Alacritty wake-ups/s | Rounds |")
print("|---|---|---|---|---|---|---|---|---|")
for load, n in cases:
    hq, alacritty = ("hq", load, n), ("alacritty", load, n)
    count = max(len(rounds.get(hq, [])), len(rounds.get(alacritty, [])))
    print(f"| {describe(load)} | {n} | {cell(median(hq, 0), '%')} | "
          f"{cell(median(alacritty, 0), '%')} | {cell(median(hq, 2), 'MB')} | "
          f"{cell(median(alacritty, 2), 'MB')} | {cell(median(hq, 1), '')} | "
          f"{cell(median(alacritty, 1), '')} | {count} |")
