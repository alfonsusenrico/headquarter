#!/bin/zsh
# suite.sh: the README comparison, ROUNDS times (default 3), one log per round in
# $BENCH_DIR; summarize.py turns the logs into the table. Needs record.sh's three
# recordings for the replay loads. Takes about 30 minutes a round.
bench=${0:A:h}
export BENCH_DIR=${BENCH_DIR:-/tmp/hq-bench}
# A sleeping display stops Headquarter drawing and unpaces Alacritty, which voids
# every number after it; hold the display awake until this script exits.
caffeinate -d -i -w $$ &
for round in $(seq ${ROUNDS:-3}); do
  log=$BENCH_DIR/round-$round.log
  : > $log
  for mode in hq alacritty; do
    $bench/run.sh $mode idle "1 10 25 50" 30 >> $log
    $bench/run.sh $mode busy "1 10 25 50" 30 >> $log
    for agent in claude codex agy; do
      $bench/run.sh $mode replay:$agent "1 3 10" 30 >> $log
    done
    for agent in claude codex agy; do
      $bench/run.sh $mode real:$agent "3" 60 >> $log
    done
  done
done
