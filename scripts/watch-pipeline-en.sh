#!/usr/bin/env bash
# Drive the England pipeline to completion, unattended: download, then shading.
#
# Supersedes watch-download-en.sh, which only supervised the download. Same
# reasoning, extended a stage: both scripts are resumable and both exit non-zero
# while anything is outstanding, so "keep starting passes until one exits clean"
# is a complete supervision strategy for either. What this adds is that stage 2
# starts on its own when stage 1 finishes, so a download that completes at 4am
# does not sit idle until someone notices.
#
# STALL DETECTION. Retrying forever is only right while progress is being made.
# Each pass is measured by a stage-specific counter before and after; a pass that
# adds nothing counts as a stall, and after MAX_STALLS consecutive stalls the
# stage gives up and says so loudly rather than looping on something that cannot
# succeed. Any progress resets the counter. Backoff doubles to a 30 min cap.
#
# WHY STAGE 2 IS SAFE TO RETRY. shading-en.nu skips any window that already has
# an output or an .empty marker, and refuses to build the national mosaic while
# any window is still missing — so a killed pass costs only the windows in
# flight, and no partial country can be mistaken for the finished product.
#
# CONTOURS ARE NOT CHAINED. contours-en.nu consolidates a few hundred GB and then
# runs gdal_contour for hours; it is a separate decision, and the shading output
# is worth looking at first. The command is logged when stage 2 finishes.
#
# NEVER PUT THIS LOG ON THE 18TB, AND NEVER GIVE IT TWO WRITERS. Both rules come
# from the same incident, and each one alone is enough to hang the supervisor:
#
#   /media/martin/18TB is ntfs3. The first version of this script logged with
#   `printf ... | tee -a "$LOG"` while the launcher ALSO redirected stdout to
#   "$LOG", and ran each stage with `>>"$LOG"` on top of that. Three independent
#   appenders to one file on ntfs3 wedged the shell in ntfs_file_write_iter — D
#   state, deaf to SIGKILL, holding the inode lock until reboot — after writing
#   exactly two lines. The filesystem was fine throughout (249 MB/s, single-writer
#   appends fine); the contention was the whole problem.
#
# So: the log lives on btrfs, and there is EXACTLY ONE writer — the launcher's
# redirect. log() prints to stdout and nothing tees; stages inherit stdout rather
# than re-redirecting. Do not "helpfully" add a tee back.
#
# RUN IT DETACHED, so nothing tied to a shell or an editor session can take it
# down (the first England run lost its downloader and the ANCPI monitor in the
# same instant, which is what prompted all of this):
#
#   setsid nohup ~/fm/freemap-outdoor-map/watch-pipeline-en.sh \
#     >/mnt/osm/en/watch-pipeline.log 2>&1 < /dev/null &
#
# Check on it with:
#   tail -30 /mnt/osm/en/watch-pipeline.log
#   ls /media/martin/18TB/en/DTM1/*.tif | wc -l          # stage 1, of 5876
#   ls /media/martin/18TB/en/tiles/ | wc -l              # stage 2, of 23504
#
# To stop it:
#   pkill -f watch-pipeline-en.sh && pkill -f 'download-en.nu|shading-en.nu'

set -u

REPO="$(dirname "$(readlink -f "$0")")"
LOG="/mnt/osm/en/watch-pipeline.log"   # btrfs, NOT the ntfs3 18TB — see header
SRC_DIR="/media/martin/18TB/en/DTM1"
TILES_DIR="/mnt/osm/en/tiles"        # NVMe — see shading-en.nu, WHERE THE I/O GOES
CONDA="$HOME/miniforge3/bin/conda"

SRC_TOTAL=5876                       # tiles in the EA index
WINDOWS_PER_TILE=4                   # shading-en.nu cuts 2x2 windows per 5 km tile
MAX_STALLS=5

# feature-preserving-smoothing lives in ~/.cargo/bin, which a detached process
# started from a non-login context may not have on PATH. shading-en.nu fails
# without it, so put it there explicitly rather than depending on inheritance.
export PATH="$HOME/.cargo/bin:$PATH"

log() { printf '%s  %s\n' "$(date -Is)" "$1"; }   # stdout only — see header

count_src()   { ls "$SRC_DIR"/*.tif 2>/dev/null | wc -l | tr -d ' '; }
count_tiles() { ls "$TILES_DIR" 2>/dev/null | grep -cE '\.(tif|empty)$'; }

# Run a stage until it exits 0, or until it stalls MAX_STALLS times in a row.
# $1 label   $2 counter function   $3 expected total   $4.. command
supervise() {
  local label="$1" counter="$2" total="$3"; shift 3
  local pass=0 stalls=0 sleep_for=60 before after gained rc

  log "[$label] starting — $($counter)/$total"

  while true; do
    pass=$((pass + 1))
    before=$($counter)
    log "[$label] pass $pass starting at $before/$total"

    "$@" 2>&1
    rc=$?

    after=$($counter)
    gained=$((after - before))

    if [ "$rc" = "0" ]; then
      log "[$label] pass $pass exited clean — $after/$total"
      return 0
    fi

    if [ "$gained" -gt 0 ]; then
      stalls=0
      sleep_for=60
      log "[$label] pass $pass incomplete (exit $rc), +$gained -> $after/$total; retry in ${sleep_for}s"
    else
      stalls=$((stalls + 1))
      log "[$label] pass $pass NO progress (exit $rc), still $after/$total — stall $stalls/$MAX_STALLS"
      if [ "$stalls" -ge "$MAX_STALLS" ]; then
        log "[$label] GIVING UP after $MAX_STALLS stalled passes at $after/$total."
        log "[$label] This is past a transient failure — read the log above. Nothing done is lost;"
        log "[$label] re-run this script once the cause is fixed and it resumes."
        return 1
      fi
      sleep_for=$(( sleep_for * 2 ))
      [ "$sleep_for" -gt 1800 ] && sleep_for=1800
      log "[$label] retry in ${sleep_for}s"
    fi

    sleep "$sleep_for"
  done
}

log "=============================================================="
log "pipeline supervisor starting (pid $$)"

# Record the process-group id so the run can be paused and resumed without
# hunting for pids. setsid makes this script the group leader, so the whole
# pipeline (nu, conda, every gdal child) shares this pgid:
#   kill -STOP -$(cat /mnt/osm/en/pipeline.pgid)     # pause
#   kill -CONT -$(cat /mnt/osm/en/pipeline.pgid)     # resume
# Pausing is safe at any point: shading is pure local compute with no network
# and no partially-written output — a tile is only moved into place once it is
# complete and has passed the 4-band check.
ps -o pgid= -p $$ | tr -d " " > /mnt/osm/en/pipeline.pgid
log "process group $(cat /mnt/osm/en/pipeline.pgid) — pause with: kill -STOP -$(cat /mnt/osm/en/pipeline.pgid)"

# Run the whole pipeline at low priority. This is a multi-day bulk job on a
# machine that is also used interactively, and it will happily saturate 24 cores
# and the NVMe otherwise. Set on the supervisor itself: nice level and I/O class
# are inherited across fork/exec, so every stage, every gdal call and every curl
# picks these up without further plumbing.
#
# Non-root can only ever RAISE the nice value, so this is one-way within a run —
# restart the supervisor to reset it. For an even gentler run use `renice -n 19`
# and `ionice -c3` (idle); idle I/O needs no privileges since 2.6.25, but it can
# stall badly if anything else competes for the disk.
renice -n 15 -p $$ >/dev/null 2>&1 || true
ionice -c2 -n7 -p $$ >/dev/null 2>&1 || true


# The 18TB is removable, and udisks2 has already moved it once: it was
# /media/martin/18TB before the 2026-08-16 reboot and /media/martin/18TB
# after (the device letter shifted too, sdb2 -> sda2). The OLD PATH SURVIVES as
# an empty root-owned directory on /, which has ~99 GB free against this
# pipeline's ~330 GB — so a stale path does not fail, it silently fills the root
# filesystem. Refuse to start unless the data root is genuinely a mount.
DATA_ROOT="/media/martin/18TB"
if ! mountpoint -q "$DATA_ROOT"; then
  log "ABORT: $DATA_ROOT is not a mountpoint — the 18TB drive is not mounted there."
  log "  Find it:  lsblk -o NAME,LABEL,SIZE,MOUNTPOINT   (label is 18TB)"
  log "  Then repoint the paths in this script and download/shading/contours-en."
  log "  Refusing to run: writing to a stale path would fill / instead."
  exit 1
fi


# ── Stage 1: download ─────────────────────────────────────────────────────────

# SKIP_DOWNLOAD=1 jumps straight to shading. Stage 1 re-reads all 330 GB through
# gdalinfo -checksum on every run, which is an hour well spent the first time and
# pure waste when you are only re-running shading (e.g. to rebuild one window and
# re-merge). Only set it when the delivery has already passed a clean verify.
if [ "${SKIP_DOWNLOAD:-0}" = "1" ]; then
  have=$(count_src)
  if [ "$have" != "$SRC_TOTAL" ]; then
    log "ABORT: SKIP_DOWNLOAD=1 but only $have/$SRC_TOTAL tiles are present."
    log "  Refusing to shade a partial country. Unset SKIP_DOWNLOAD and re-run."
    exit 1
  fi
  log "[download] SKIPPED by SKIP_DOWNLOAD=1 — $have/$SRC_TOTAL tiles present"
elif ! supervise "download" count_src "$SRC_TOTAL" nice nu "$REPO/download-en.nu"; then
  log "stopping: download did not complete, so shading would render a partial country"
  exit 1
fi

log "download complete: $(count_src)/$SRC_TOTAL tiles, $(du -sh "$SRC_DIR" | cut -f1)"

# ── Stage 2: shading ─────────────────────────────────────────────────────────
# Needs the conda geo env: the merge writes JXL, which requires GDAL linked
# against a libtiff with libjxl support.

WINDOW_TOTAL=$(( $(count_src) * WINDOWS_PER_TILE ))
log "shading: expecting $WINDOW_TOTAL windows at z16"

if ! supervise "shading" count_tiles "$WINDOW_TOTAL" \
     nice "$CONDA" run --no-capture-output -n geo nu "$REPO/shading-en.nu"; then
  log "stopping: shading incomplete"
  exit 1
fi

log "=============================================================="
log "SHADING DONE — /media/martin/18TB/en/shading.tif"
log "NEXT, when you want it (hours; not chained on purpose):"
log "  nice $CONDA run --no-capture-output -n geo nu $REPO/contours-en.nu"
log "=============================================================="
