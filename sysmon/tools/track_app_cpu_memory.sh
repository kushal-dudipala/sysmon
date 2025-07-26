#!/usr/bin/env bash
set -euo pipefail

# macOS-only: uses BSD ps output; 100% CPU ≈ one core

# Usage:
#   ./tools/measure_app.sh <cmd> [args...]
#   PROC_NAME=sysmon ./tools/measure_app.sh cargo run --release
#   ./tools/measure_app.sh ./target/release/sysmon
#   ./tools/measure_app.sh ./sysmon.app/Contents/MacOS/sysmon
#
# Set INTERVAL=0.5 for faster updates.

if [[ $# -eq 0 ]]; then
  echo "usage: $0 <command> [args...]"
  exit 2
fi

INTERVAL="${INTERVAL:-1}"
PROC_NAME="${PROC_NAME:-sysmon}"

# Launch the command in background
"$@" &
PARENT_PID=$!
TARGET_PID=$PARENT_PID

echo "Started PID=$PARENT_PID: $*"

# If they're using `cargo run`, try to follow the actual app process.
if [[ "$1" == "cargo" && "${2:-}" == "run" ]]; then
  # Wait briefly for a child process that matches PROC_NAME
  for _ in {1..50}; do
    # Newest child of the cargo process (-P = parent)
    child="$(pgrep -n -P "$PARENT_PID" || true)"
    if [[ -n "$child" ]]; then
      comm="$(ps -o comm= -p "$child" 2>/dev/null || true)"
      if [[ "$comm" == *"$PROC_NAME"* ]]; then
        TARGET_PID="$child"
        echo "🔁 following child PID=$TARGET_PID ($comm)"
        break
      fi
    fi
    sleep 0.1
  done
fi

trap 'kill -TERM "$PARENT_PID" 2>/dev/null || true; exit' INT TERM

printf "%-8s %-6s %-8s %-8s\n" "TIME" "CPU%" "RSS(MB)" "VSZ(MB)"
max_rss_kb=0

# Helper to print one sample for a PID (returns nonzero if PID vanished)
print_sample() {
  local pid="$1"
  local line
  line="$(ps -o %cpu= -o rss= -o vsz= -p "$pid" 2>/dev/null || true)"
  [[ -z "$line" ]] && return 1
  read -r cpu rss_kb vsz_kb <<<"$line"
  local rss_mb vsz_mb ts
  rss_mb=$(awk -v kb="$rss_kb" 'BEGIN { printf "%.1f", kb/1024 }')
  vsz_mb=$(awk -v kb="$vsz_kb" 'BEGIN { printf "%.1f", kb/1024 }')
  ts=$(date +%H:%M:%S)
  printf "%-8s %-6s %-8s %-8s\n" "$ts" "$cpu" "$rss_mb" "$vsz_mb"
  if [[ "$rss_kb" -gt "$max_rss_kb" ]]; then max_rss_kb="$rss_kb"; fi
  return 0
}

# Main loop: prefer TARGET_PID; if it exits but the parent still lives,
# fall back to the parent so we keep printing until everything is done.
while true; do
  if kill -0 "$TARGET_PID" 2>/dev/null; then
    print_sample "$TARGET_PID" || break
  elif kill -0 "$PARENT_PID" 2>/dev/null; then
    print_sample "$PARENT_PID" || break
  else
    break
  fi
  sleep "$INTERVAL"
done

max_rss_mb=$(awk -v kb="$max_rss_kb" 'BEGIN { printf "%.1f", kb/1024 }')
echo "Done. Peak RSS ≈ ${max_rss_mb} MB."
