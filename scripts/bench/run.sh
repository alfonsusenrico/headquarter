#!/bin/zsh
# run.sh MODE LOAD "N..." SECONDS: measures Headquarter or Alacritty with N sessions.
#
#   MODE    hq (the app and host from target/release, or $BENCH_BIN) or alacritty (one window each)
#   LOAD    idle, busy      the stand-in agent idle, or working
#           replay:AGENT    the stand-in replaying record.sh's recording of AGENT
#           real:AGENT      the real agent, idle (no prompt, so no tokens)
#   N...    session counts, measured in turn as sessions are added
#   SECONDS the measuring window
#
# The agents' own processes are left out on both sides; Headquarter's figures include
# its host and any helper processes the host runs. macOS only.
set -u
bench=${0:A:h}
repo=${bench:h:h}
R=${BENCH_BIN:-$repo/target/release}
A=${ALACRITTY:-/Applications/Alacritty.app/Contents/MacOS/alacritty}
export BENCH_DIR=${BENCH_DIR:-/tmp/hq-bench}
work=${BENCH_WORK:-$BENCH_DIR/work}
mode=$1 load=$2 counts=(${=3}) seconds=$4
mkdir -p $BENCH_DIR $work
rm -f $BENCH_DIR/busy $BENCH_DIR/replay

agent=claude
command=($bench/stand-in)
case $load in
  busy) touch $BENCH_DIR/busy ;;
  replay:*)
    agent=${load#replay:}
    print -r -- $BENCH_DIR/rec-$agent.bin > $BENCH_DIR/replay ;;
  real:*)
    agent=${load#real:}
    command=($agent) ;;
esac

export HQ_DATA_DIR=$BENCH_DIR/data-$mode HQ_CONFIG_DIR=$BENCH_DIR/config-${load%%:*}
rm -rf $HQ_DATA_DIR; mkdir -p $HQ_DATA_DIR $HQ_CONFIG_DIR
if [[ $load == real:* ]]; then
  print '# benchmark profile: the real agents' > $HQ_CONFIG_DIR/config.toml
else
  for name in claude codex agy; do
    print "[agents.$name]\npath = \"$bench/stand-in\"\n"
  done > $HQ_CONFIG_DIR/config.toml
fi

if [[ $mode == hq ]]; then
  $bench/fresh $R/hq new $agent --detach --cwd $work >/dev/null
  host=$(lsof -t $HQ_DATA_DIR/state.db | while read pid; do
    ps -o command= -p $pid | grep -q "hq daemon" && print $pid; done | head -1)
  $R/headquarter-app >/dev/null 2>&1 &
  app=$!
  sleep 6
  # Shows the first session, as the owner would: Cmd-1, Down, Enter.
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $app) to true" \
    -e 'delay 0.5' -e 'tell application "System Events" to keystroke "1" using command down' \
    -e 'delay 0.3' -e 'tell application "System Events" to key code 125' \
    -e 'delay 0.3' -e 'tell application "System Events" to key code 36' >/dev/null
  made=1
  for n in $counts; do
    while (( made < n )); do $bench/fresh $R/hq new $agent --detach --cwd $work >/dev/null; made=$((made + 1)); done
    sleep 10
    helpers=$(ps -axo pid=,ppid=,tty= | awk -v host=$host '$2 == host && $3 == "??" {printf "%s%s", sep, $1; sep = ","}')
    python3 -I $bench/measure.py "hq $load n=$n" $seconds app=$app host=$host helpers=$helpers
  done
  kill $app
  $R/hq stop --yes >/dev/null
else
  windows=()
  for n in $counts; do
    while (( ${#windows} < n )); do
      $bench/fresh $A --working-directory $work -e $command >/dev/null 2>&1 &
      windows+=($!)
    done
    sleep 10
    python3 -I $bench/measure.py "alacritty $load n=$n" $seconds alacritty=${(j:,:)windows}
  done
  for pid in $windows; do kill $pid; done
fi
rm -f $BENCH_DIR/busy $BENCH_DIR/replay
