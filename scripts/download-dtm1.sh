#!/usr/bin/env bash
# Download Kartverket's national 1 m terrain model (DTM1) — portable version of
# download-no.nu for servers without nushell (e.g. fm6).
#
# Source: Geonorge INSPIRE Atom download service. 2033 pre-cut GeoTIFF tiles,
# plain HTTP, no login, no order form. ~1.63 TB total, avg 806 MB/tile.
# Licence NLOD 2.0 — free, commercial use allowed, attribution "Kartverket".
#
# Tiles are uniform: 15010 x 15010 px, Float32, 1 m, nodata -32767, EPSG:25833,
# LZW, 512x512 blocks, 6 overviews. Grid step is 15000 m against 15010 px tiles,
# i.e. adjacent tiles OVERLAP by 10 px (5 px collar per side) with identical
# values — gdalbuildvrt needs no special handling, but don't assume tiles are
# edge-to-edge when cutting them up.
#
# ─────────────────────────────────────────────────────────────────────────────
# TWO LESSONS FROM THE NORWAY RUN, both baked in below. Do not "optimise" them
# back out; they each cost about a day.
#
# 1. NEVER RESUME A PARTIAL FILE. The obvious `curl -C -` resume is unsafe here.
#    When a download run is killed, files in flight can be left with correct size
#    metadata but unflushed tail blocks. `-C -` then appends good data after a
#    garbage gap, producing a file whose byte count matches Content-Length
#    exactly and whose middle is corrupt. Three tiles arrived that way
#    (33-125-144/145/146) and took down a 30-hour render — twice, because the
#    corruption is invisible to any size check. Partial files are therefore
#    DISCARDED and re-fetched whole. The cost is small: only the handful of files
#    actually in flight at the interruption are re-downloaded.
#
# 2. VERIFY CONTENT, NOT JUST SIZE — AND STRICTLY. A tile is sound only if
#    gdalinfo -checksum emits NO ERROR. Accepting "a Checksum= line was printed"
#    is not enough: when only SOME blocks fail to decode, GDAL prints ERRORs AND
#    a checksum, so partial corruption passes. That mistake let 33-125-144
#    through a full 2033-tile scan.
# ─────────────────────────────────────────────────────────────────────────────
#
# Resumable between runs: a tile whose local size equals the server's
# Content-Length is skipped. Interrupt and re-run freely.
#
# Usage:  ./download-dtm1.sh [DEST_DIR] [PARALLEL]
#   e.g.  ./download-dtm1.sh /fm/storage2/dtm 6
#
# Needs bash, curl, coreutils. GDAL is optional — without it the content
# verification stage is skipped and the script says so loudly. Run under
# nohup/tmux: ~10 h at the ~41 MB/s a 6-way run gets from Kartverket.

set -u

DEST="${1:-/fm/storage2/dtm}"
PARALLEL="${2:-6}"
ATOM="https://nedlasting.geonorge.no/geonorge/ATOM/hoydedata/datasett/DTM1.atom"
INDEX="$DEST/urls.tsv"
RETRIES=3

mkdir -p "$DEST"

# ── 1. Tile index: url<TAB>bytes, cached (delete urls.tsv to refresh) ─────────

if [ -s "$INDEX" ]; then
  echo "==> Reusing tile index $INDEX"
else
  echo "==> Fetching Atom feed"
  URLS=$(curl -s --max-time 300 "$ATOM" \
    | grep -oE 'https://nedlasting\.geonorge\.no/hoydedata/DTM1/[^"]+\.tif' \
    | sort -u)
  COUNT=$(printf '%s\n' "$URLS" | grep -c . || true)
  echo "  $COUNT tiles in feed"
  # A transient feed failure would otherwise cache an EMPTY index that every
  # later run happily "reuses".
  if [ "${COUNT:-0}" -lt 100 ]; then
    echo "!! only $COUNT tiles parsed from the feed — refusing to cache a bad index" >&2
    exit 1
  fi

  echo "==> Reading sizes ($COUNT HEAD requests)"
  printf '%s\n' "$URLS" | xargs -P "$PARALLEL" -I{} sh -c '
    u="{}"
    for a in 1 2 3 4; do
      len=$(curl -sIL --max-time 90 "$u" | tr -d "\r" \
              | awk "BEGIN{IGNORECASE=1} /^content-length:/ {v=\$2} END{print v}")
      [ -n "$len" ] && { printf "%s\t%s\n" "$u" "$len"; exit 0; }
      sleep 3
    done
    printf "%s\t0\n" "$u"     # unknown size: fetched anyway, judged on curl exit
  ' > "$INDEX.tmp"
  mv "$INDEX.tmp" "$INDEX"
fi

TOTAL=$(awk -F'\t' '{s+=$2} END {printf "%.2f", s/1e12}' "$INDEX")
echo "==> $(wc -l < "$INDEX") tiles, ${TOTAL} TB -> $DEST"

# ── 2. Fetch what is missing or the wrong size (never resume — see header) ────

export DEST RETRIES
awk -F'\t' '{print $1"\t"$2}' "$INDEX" | xargs -P "$PARALLEL" -I{} sh -c '
  line="{}"
  url=${line%%	*}
  want=${line##*	}
  name=$(basename "$url")
  dst="$DEST/$name"

  have=0
  [ -f "$dst" ] && have=$(stat -c%s "$dst" 2>/dev/null || echo 0)
  if [ "$want" != "0" ] && [ "$have" = "$want" ]; then exit 0; fi

  # A partial file is DISCARDED, never resumed. See header lesson 1.
  if [ "$have" -gt 0 ]; then
    echo "  $name: discarding partial ($have of $want) and re-fetching whole"
    rm -f "$dst"
  fi

  i=1
  while [ "$i" -le "$RETRIES" ]; do
    [ "$i" = 1 ] && echo "  $name: fetching $want"
    rm -f "$dst"
    curl -sSL --fail --retry 5 --retry-delay 5 --retry-all-errors \
         --max-time 7200 -o "$dst" "$url"
    rc=$?
    got=0; [ -f "$dst" ] && got=$(stat -c%s "$dst" 2>/dev/null || echo 0)
    if [ "$rc" = "0" ] && { [ "$want" = "0" ] && [ "$got" -gt 0 ] || [ "$got" = "$want" ]; }; then
      echo "  $name: ok"
      exit 0
    fi
    echo "  $name: attempt $i failed (curl $rc, $got of $want)"
    i=$((i+1))
  done
  echo "  !! $name: giving up this run — re-run to retry"
'

# ── 3. Size check ────────────────────────────────────────────────────────────

echo "==> Checking sizes"
BAD=$(awk -F'\t' -v d="$DEST" '
  {
    n = $1; sub(/.*\//, "", n)
    cmd = "stat -c%s \"" d "/" n "\" 2>/dev/null || echo 0"
    cmd | getline sz; close(cmd)
    if (sz == 0 || ($2 != 0 && sz != $2)) print n": "sz" of "$2
  }' "$INDEX")

if [ -n "$BAD" ]; then
  echo "==> Incomplete — re-run to fetch the rest:"
  printf '%s\n' "$BAD" | head -20
  exit 1
fi
echo "  all $(wc -l < "$INDEX") tiles present at the expected size"

# ── 4. Content check — the one that actually matters (see header lesson 2) ───

if ! command -v gdalinfo >/dev/null 2>&1; then
  echo "!! gdalinfo not found — CONTENT NOT VERIFIED."
  echo "!! Sizes alone cannot detect the corruption this script guards against."
  echo "!! Install GDAL and re-run, or verify elsewhere before using the data."
  exit 0
fi

echo "==> Verifying content of $(wc -l < "$INDEX") tiles (strict: any ERROR fails)"
VLOG="$DEST/verify_pixels.log"
: > "$VLOG"
ls "$DEST"/*.tif | xargs -P "$PARALLEL" -I{} sh -c '
  f="{}"
  out=$(gdalinfo -checksum "$f" 2>&1)
  if echo "$out" | grep -q "ERROR"; then
    printf "CORRUPT %s | %s\n" "$(basename "$f")" "$(echo "$out" | grep -m1 ERROR | cut -c1-90)"
  elif echo "$out" | grep -q "Checksum="; then
    printf "ok %s\n" "$(basename "$f")"
  else
    printf "CORRUPT %s | no checksum produced\n" "$(basename "$f")"
  fi
' >> "$VLOG" 2>&1

NBAD=$(grep -c '^CORRUPT ' "$VLOG" || true)
echo "  ok: $(grep -c '^ok ' "$VLOG")   corrupt: $NBAD"
if [ "$NBAD" -gt 0 ]; then
  grep '^CORRUPT ' "$VLOG" | head -20
  echo "==> Delete the corrupt tiles listed above and re-run to re-fetch them."
  exit 1
fi

echo "==> Complete and verified: $(wc -l < "$INDEX") tiles in $DEST"
echo "    Next: gdalbuildvrt -vrtnodata -32767 -input_file_list <list> no.vrt"
