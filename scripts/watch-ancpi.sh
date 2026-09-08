#!/usr/bin/env bash
# Watch for ANCPI's geoportal coming back, so the Romanian LiDAR download can
# start unattended.
#
# ANCPI was taken offline after a security incident (infrastructure isolated
# following DNSC intervention). e-Terra restarted in stages on 11-12 Aug 2026;
# the remaining public platforms — geoportal.ancpi.ro included, which is where
# every LAKI download URL lives — are still down, to be restored "etapizat, cu
# anunt prealabil", with no date given. See download-ro.nu and index/README.md.
#
# The DNS record for geoportal.ancpi.ro has been WITHDRAWN from the zone
# (NXDOMAIN straight from iris.ns.cloudflare.com with the aa flag), so DNS
# reappearing is the earliest possible signal — it must happen before anything
# can be served. The HTTP check then confirms the host is actually answering and
# not merely resolving to a holding page.
#
# Both are checked because either alone lies: DNS can come back hours before the
# service does, and geoportal.gov.ro already resolves while refusing every
# connection.
#
# Usage:
#   ./watch-ancpi.sh [--run] [INTERVAL_SECONDS]
#
#   --run   on success, immediately run download-ro.nu. That is safe to leave
#           unattended: download-ro.nu's first act is the layout probe, which
#           fetches exactly ONE tile, writes index/layout.json and stops. So
#           --run gets you a real tile to inspect the moment ANCPI returns,
#           without starting a 400 GB transfer nobody is watching.
#
# Default interval is 600 s. The log records state changes and an hourly
# heartbeat, not every poll, so it stays readable across a multi-week wait.
#
# Run under tmux/nohup, or as a systemd user unit:
#   nohup ./watch-ancpi.sh --run >/dev/null 2>&1 &

set -u

RUN=0
if [ "${1:-}" = "--run" ]; then RUN=1; shift; fi
INTERVAL="${1:-600}"

HOSTNAME_="geoportal.ancpi.ro"
PROBE_URL="https://geoportal.ancpi.ro/"
LOG="/mnt/osm/ro/watch-ancpi.log"   # btrfs: a log on the 18TB keeps a fd open there and blocks unmounting it
DOWNLOADER="$(dirname "$(readlink -f "$0")")/download-ro.nu"

mkdir -p "$(dirname "$LOG")"

log() { printf '%s  %s\n' "$(date -Is)" "$1" >>"$LOG"; }

log "watching $HOSTNAME_ every ${INTERVAL}s (run-on-success=$RUN)"

last_state=""
last_beat=0

while true; do
  # 1. Is the name back in DNS at all?
  if ip=$(getent hosts "$HOSTNAME_" 2>/dev/null | awk 'NR==1{print $1}') && [ -n "$ip" ]; then
    dns="up ($ip)"
  else
    dns="down"
  fi

  # 2. Does it answer HTTP? Any status code counts — a 403 or a maintenance 503
  #    still means the host is serving again and is worth waking up for.
  code=$(curl -sI --max-time 25 -o /dev/null -w '%{http_code}' "$PROBE_URL" 2>/dev/null || true)
  if [ -n "$code" ] && [ "$code" != "000" ]; then
    http="up ($code)"
  else
    http="down"
  fi

  state="dns=$dns http=$http"

  if [ "$state" != "$last_state" ]; then
    log "CHANGE  $state"
    last_state="$state"
    last_beat=$(date +%s)
  elif [ $(( $(date +%s) - last_beat )) -ge 3600 ]; then
    log "still   $state"
    last_beat=$(date +%s)
  fi

  if [ "$http" != "down" ]; then
    log "ANCPI IS BACK — $state"
    if [ "$RUN" = "1" ] && [ -x "$(command -v nu || true)" ]; then
      log "starting $DOWNLOADER (layout probe: fetches one tile, then stops)"
      nu "$DOWNLOADER" 2>&1 | tee -a "$LOG"
      log "download-ro.nu exited $?"
    else
      log "not auto-running; start it with: nu $DOWNLOADER"
    fi
    exit 0
  fi

  sleep "$INTERVAL"
done
