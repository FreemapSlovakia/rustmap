#!/usr/bin/env nu

# Download the Norwegian national 1 m terrain model (Kartverket DTM1) tile set.
#
# Source: Geonorge INSPIRE Atom download service — 2033 pre-cut GeoTIFF tiles,
# direct HTTP, no login, no order form:
#   https://nedlasting.geonorge.no/geonorge/ATOM/hoydedata/datasett/DTM1.atom
#   -> https://nedlasting.geonorge.no/hoydedata/DTM1/33-132-158.tif  (etc.)
#
# Licence: NLOD 2.0 — free, commercial use allowed, attribution "Kartverket".
#
# The tiles are uniform (verified on a 24-tile random sample, 2026-08-03):
#   15010 x 15010 px, Float32, 1 m, nodata -32767 declared on EVERY tile,
#   EPSG:25833 embedded on every tile, LZW, 512x512 internal blocks, 6 overviews.
# So NONE of the Croatian delivery's quirks apply — no mixed nodata sentinels,
# no CRS-less tiles, no unmarked 0-voids. Downstream stage 0 is a plain
# `gdalbuildvrt -input_file_list` with -vrtnodata; no -a_srs, no
# -allow_projection_difference, no SimpleSource->ComplexSource rewriting.
#
# Tile grid step is 15000 m against 15010 px tiles, i.e. tiles OVERLAP by 10 px
# (5 px per side). The overlapping pixels are identical, so gdalbuildvrt needs
# no special handling — but a retile/crop step must not assume edge-to-edge tiles.
#
# Volume: ~1.63 TB total, avg 806 MB/tile, max 1.23 GB.
#
# RESUMABLE at tile granularity, in two independent ways:
#   * a tile whose local size already equals the server's Content-Length is skipped
#     outright (no request beyond the cached size list);
#   * a partial tile is resumed with `curl -C -` (the server sends accept-ranges).
# Interrupt with ctrl-c and re-run; nothing is re-fetched that is already on disk.
# Delete urls.tsv to re-read the Atom feed (e.g. after Kartverket republishes tiles).
#
# Run via:
#   nu ~/fm/freemap-outdoor-map/download-no.nu
# Progress is line-per-tile on stdout; a final pass verifies every tile's size and
# reports any that are short, so a clean exit means the delivery is complete.

# ── Configuration ─────────────────────────────────────────────────────────────

const DEST     = "/media/martin/18TB/no/DTM1"
const ATOM     = "https://nedlasting.geonorge.no/geonorge/ATOM/hoydedata/datasett/DTM1.atom"
const INDEX    = "/media/martin/18TB/no/urls.tsv"   # url<TAB>bytes, cached
const PARALLEL = 6                                  # concurrent tiles; polite to Kartverket, kind to the HDD
const RETRIES  = 3                                  # per-tile attempts per run

mkdir $DEST

# ── Helpers ───────────────────────────────────────────────────────────────────

def local-size [f: string]: nothing -> int {
    if ($f | path exists) { (ls -l $f | get 0.size | into int) } else { 0 }
}

# Expected size of a tile from the server, 0 if it could not be established.
# A bare HEAD is NOT reliable at this scale: a handful of the 2033 requests come
# back without a Content-Length (transient), and one unhandled miss used to abort
# the whole par-each. So: retry, tolerate a missing header, and never throw.
# -L plus `last` because a redirect chain would carry one Content-Length per hop.
def remote-size [url: string]: nothing -> int {
    for attempt in 1..4 {
        let res = (do { curl -sIL --max-time 90 --retry 2 --retry-delay 3 --retry-all-errors $url } | complete)
        if $res.exit_code == 0 {
            let lens = (
                $res.stdout | lines
                  | where {|l| ($l | str lowercase | str starts-with "content-length:")}
                  | each {|l| $l | split row ":" | get 1 | str trim}
            )
            if ($lens | is-not-empty) { return ($lens | last | into int) }
        }
        sleep 3sec
    }
    0
}

# ── 1. Tile index: URLs + expected sizes (cached; delete INDEX to refresh) ─────

if ($INDEX | path exists) {
    print $"==> Reusing tile index ($INDEX)"
} else {
    print "==> Fetching Atom feed and building tile index (2033 HEAD requests)"

    # curl, not `http get`: nu would auto-parse the atom+xml content type into a
    # record and the regex below expects raw text.
    let urls = (
        curl -s --max-time 300 $ATOM
          | parse -r 'href="(?<u>https://nedlasting\.geonorge\.no/hoydedata/DTM1/[^"]+\.tif)"'
          | get u
          | uniq
          | sort
    )
    print $"  ($urls | length) tiles in feed"

    # Guard: a transient feed failure yields zero matches, and without this the run
    # would cache an EMPTY index that every later run then happily "reuses".
    if ($urls | is-empty) {
        error make {msg: $"no tiles parsed from ($ATOM) — feed fetch failed; refusing to cache an empty index"}
    }

    let first_pass = ($urls | par-each -t $PARALLEL {|u| {url: $u, bytes: (remote-size $u)} })

    # Second, serial pass for the few that came back sizeless — usually transient
    # under concurrency, and there are too few to be worth parallelising.
    let missing = ($first_pass | where bytes == 0)
    let rows = if ($missing | is-empty) { $first_pass } else {
        print $"  ($missing | length) tile\(s\) returned no size — retrying serially"
        $first_pass | each {|r| if $r.bytes > 0 { $r } else { {url: $r.url, bytes: (remote-size $r.url)} } }
    }

    let unknown = ($rows | where bytes == 0)
    if ($unknown | is-not-empty) {
        print $"  !! ($unknown | length) tile\(s\) still have no known size — they will be fetched anyway,"
        print "     accepted on curl success, and flagged as unverified at the end."
    }

    $rows | sort-by url | to tsv | save -f $INDEX
    print $"  indexed ($rows | length) tiles, ($rows.bytes | math sum | into filesize) total"
}

let tiles = (open --raw $INDEX | from tsv)   # --raw: plain `open` would auto-parse the .tsv already
let total = ($tiles.bytes | math sum)
print $"==> ($tiles | length) tiles, ($total | into filesize) total -> ($DEST)"

# ── 2. Download what is missing or short ──────────────────────────────────────
# A tile with bytes == 0 has no known size (see remote-size); it is always
# attempted and judged on curl's exit status alone.

let pending = (
    $tiles | where {|t|
        let f = ($DEST | path join ($t.url | path basename))
        ($t.bytes == 0) or ((local-size $f) != $t.bytes)
    }
)

let have = (($tiles | length) - ($pending | length))
let remaining = (if ($pending | is-empty) { 0 } else { $pending | get bytes | math sum })
print $"==> ($have) tiles already complete, ($pending | length) to fetch \(($remaining | into filesize) remaining, before resume credit\)"

if ($pending | length) > 0 {
    let n = ($pending | length)
    (
      $pending | enumerate | par-each -t $PARALLEL {|it|
        let name = ($it.item.url | path basename)
        let dst  = ($DEST | path join $name)
        # Last chance to learn the size: an index entry may predate a transient miss.
        let want = (if $it.item.bytes > 0 { $it.item.bytes } else { remote-size $it.item.url })
        mut ok = false

        for attempt in 1..$RETRIES {
            let start = (local-size $dst)
            if $start > 0 {
                print $"  [($it.index + 1)/($n)] ($name): resuming at ($start | into filesize)"
            } else if $want > 0 {
                print $"  [($it.index + 1)/($n)] ($name): fetching ($want | into filesize)"
            } else {
                print $"  [($it.index + 1)/($n)] ($name): fetching \(size unknown\)"
            }

            # -C "-" resumes from the local byte count; --retry rides out transient 5xx.
            # The "-" MUST be quoted — bare `-` parses as nu's minus operator.
            let res = (
                do {
                    (curl -sSL --fail --retry 5 --retry-delay 5 --retry-all-errors
                       --max-time 7200 -C "-" -o $dst $it.item.url)
                } | complete
            )

            let got = (local-size $dst)
            if $res.exit_code == 0 and (($want == 0 and $got > 0) or $got == $want) {
                let note = (if $want == 0 { " \(size unverified\)" } else { "" })
                print $"  [($it.index + 1)/($n)] ($name): ok($note)"
                $ok = true
                break
            }

            print $"  [($it.index + 1)/($n)] ($name): attempt ($attempt) failed \(exit ($res.exit_code), ($got | into filesize) of ($want | into filesize)\)"
            if ($res.stderr | str trim | is-not-empty) {
                print $"      ($res.stderr | str trim)"
            }
            # No rm: the partial file is the resume point for the next attempt.
        }

        if not $ok {
            print $"  !! ($name): giving up this run — re-run the script to retry"
        }
      }
    )
}

# ── 3. Verify — a clean report here means the delivery is complete ─────────────

print "==> Verifying tile sizes"

# Re-resolve any still-unsized tiles here rather than trusting the cached 0, so a
# transient miss during the index build does not permanently weaken verification.
let checked = (
    $tiles | par-each -t $PARALLEL {|t|
        let f = ($DEST | path join ($t.url | path basename))
        let want = (if $t.bytes > 0 { $t.bytes } else { remote-size $t.url })
        {tile: ($t.url | path basename), expected: $want, actual: (local-size $f)}
    }
)

# A zero-byte/absent file is always wrong, even when the expected size is unknown —
# otherwise a tile that failed to download entirely would pass as "unverified".
let bad        = ($checked | where {|c| $c.actual == 0 or ($c.expected > 0 and $c.actual != $c.expected)})
let unverified = ($checked | where {|c| $c.expected == 0 and $c.actual > 0})

if ($bad | is-empty) {
    print $"==> Complete: ($tiles | length) tiles, ($checked | get actual | math sum | into filesize) in ($DEST)"
    if ($unverified | is-not-empty) {
        print $"    ($unverified | length) tile\(s\) downloaded but size-unverified \(server never returned Content-Length\):"
        $unverified | each {|u| print $"      ($u.tile): ($u.actual | into filesize) on disk"}
        print "    Spot-check these with `gdalinfo` before building the VRT."
    }
    print "    Next: build the national VRT (plain gdalbuildvrt -vrtnodata -32767), then retile/smooth."
} else {
    print $"==> ($bad | length) tile\(s\) incomplete — re-run to resume:"
    $bad | first 20 | each {|b| print $"    ($b.tile): ($b.actual | into filesize) of ($b.expected | into filesize)"}
    exit 1
}
