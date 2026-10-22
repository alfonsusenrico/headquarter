#!/bin/zsh
# record.sh AGENT: records one real AGENT session (claude, codex or agy) doing about 90 s
# of work in one turn, to $BENCH_DIR/rec-AGENT.bin, for run.sh's replay:AGENT load.
#
# It spends one short turn of the agent. The work is a command that prints a line a
# second; Claude Code pushes a bare `sleep` to the background, so it pings instead.
# Recordings hold the agent's screen, including its banner (account name): keep them
# out of the repository.
set -eu
bench=${0:A:h}
A=${ALACRITTY:-/Applications/Alacritty.app/Contents/MacOS/alacritty}
BENCH_DIR=${BENCH_DIR:-/tmp/hq-bench}
work=${BENCH_WORK:-$BENCH_DIR/work}
agent=$1
mkdir -p $BENCH_DIR $work

ticks='perl -e '"'"'for (1..90) { print "tick $_\n"; sleep 1 }'"'"
task="Run this shell command in the foreground and wait for it to finish: $ticks Then reply with only the word done."
case $agent in
  claude) args=('Run `ping -c 90 127.0.0.1` with the Bash tool in the foreground (run_in_background false, timeout 300000), wait for it, then reply with only the word done.' --allowedTools 'Bash(ping:*)') ;;
  codex) args=(-a never -s read-only $task) ;;
  agy) args=(--sandbox --dangerously-skip-permissions -i $task) ;;
  *) print -u2 "record.sh: claude, codex or agy"; exit 2 ;;
esac

$bench/fresh $A --working-directory $work -e script -q -r $BENCH_DIR/rec-$agent.bin $agent $args >/dev/null 2>&1 &
window=$!
sleep 110
kill $window
print "recorded $BENCH_DIR/rec-$agent.bin"
