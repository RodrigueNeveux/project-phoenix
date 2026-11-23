#!/usr/bin/env bash
# Measure the already running official Discord, read-only from /proc.
# Never starts, stops or signals it. usage: discord_official.sh [seconds=30]
set -euo pipefail
secs=${1:-30}
mapfile -t pids < <(pgrep -f 'discord/app-')
(( ${#pids[@]} )) || { echo "official Discord not running" >&2; exit 1; }
ver=$(tr '\0' ' ' < /proc/${pids[0]}/cmdline | grep -o 'app-[0-9.]*' | head -1 || true)
echo "discord_version=$ver"
for p in "${pids[@]}"; do
  t=$(tr '\0' ' ' < /proc/$p/cmdline | grep -o -- '--type=[a-z-]*' | head -1 || true)
  r=$(awk '/^Pss:/{print $2}' /proc/$p/smaps_rollup 2>/dev/null || echo 0)
  echo "  pid=$p ${t:-main} pss_mib=$((r/1024))"
done
"$(dirname "$0")/procstat.sh" "$secs" "${pids[@]}"
