#!/usr/bin/env nu

# Download England's national 1 m LiDAR terrain model (Environment Agency
# LIDAR Composite DTM 2022). England port of download-no.nu / download-ro.nu.
#
# Source: the Defra Data Services Platform survey API. Two calls, no login, no
# order form, no email task:
#
#   index     POST .../backend/catalog/api/tiles/collections/survey/search
#             Content-Type: application/geo+json, body = a GeoJSON polygon
#             (application/json is rejected 415)
#   download  GET  .../tiles/collections/survey/{product}/{year}/{res}/{tile}
#                  ?subscription-key=dspui
#
# `dspui` is not a credential — it is hardcoded in the site's own JavaScript as
# the public UI key. Licence is OGL v3: free commercial use, attribution
# "Environment Agency copyright and/or database right 2022".
#
# The index is built separately by build-index-en.py, because the search refuses
# a selection of 48 or more tiles and so the country has to be swept with a grid
# of 20 km cells (957 of them). See that file. Verified 2026-08-14: 5876 tiles.
#
# PRODUCT CHOICE. lidar_composite_dtm at 1 m, not national_lidar_programme_dtm.
# The composite is a single product/year/resolution key over the whole country,
# so every tile has one deterministic URL and coverage is 100% by construction.
# NLP returns one to four different survey years per tile (2016-2023 across a
# 12-point sample), which would need per-tile year-selection logic and can leave
# holes where NLP never flew. The trade is that the composite mixes vintages
# underneath. Switching later is a config change: build-index-en.py's PRODUCT /
# YEAR / RESOLUTION, and nothing here.
#
# TILE GEOMETRY, verified on four tiles across the country (SP5060, NY8010,
# TA4000, SS9045) plus one NLP tile:
#
#   5000 x 5000 px, Float32, 1 m, LZW, 128x128 blocks, no overviews
#   EPSG:27700 (OSGB36 / British National Grid) embedded on the tile itself
#   nodata -3.4028235e+38   <- Float32 lowest, NOT a friendly sentinel
#   geotransform lands on exact 5 km boundaries (e.g. 290000, 150000)
#
# Tiles are EDGE-TO-EDGE. Unlike Norway's DTM1 there is no collar and no overlap,
# so a retile/window step can assume tiles abut exactly — but must not assume
# any spare context around a tile either.
#
# NODATA IS THE ONE THING TO WATCH. -3.4028235e+38 is Float32's lowest value,
# and genuine elevations go BELOW ZERO here (a sampled Devon tile bottoms out at
# -6.06 m, which is real, not void). So nothing downstream may treat "very
# negative" as nodata; only the declared sentinel counts. Stage 0 of shading-en.nu
# should present a unified -9999 via `gdalbuildvrt -vrtnodata`, exactly as the
# Norwegian script unifies its -32767. This script checks that the sentinel is
# actually DECLARED on every tile and says so loudly if any tile lacks it — that
# is the Croatian delivery's failure mode (unmarked voids read as real terrain)
# and it is cheap to rule out here.
#
# VERIFICATION. The payload is a ZIP, so every byte is already covered by a
# CRC-32: `unzip -t` replaces the Content-Length dance entirely, and the silent
# corruption that cost two days on the Norway run (download-dtm1.sh lesson 2)
# cannot get through. The extracted raster is still put through a strict
# `gdalinfo -checksum` afterwards, because a CRC-clean zip can hold a raster the
# server built badly.
#
# Norway's other lesson is kept: NEVER RESUME A PARTIAL. A killed run can leave
# a file whose size is right and whose middle is garbage; `curl -C -` then
# appends good data after the gap. Partials are DISCARDED and re-fetched whole.
# Cheap here — tiles average ~60 MB.
#
# Only the .tif is kept. The composite zips also ship .tfw, .tif.aux.xml,
# .tif.xml and a _Metadata.gpkg; the GeoTIFF carries its own CRS and
# georeferencing, so the sidecars are redundant. (If a tile ever turns up
# WITHOUT an embedded CRS, this script fails it loudly rather than silently
# leaning on the .tfw.)
#
# DISK. ~60 MB per tile x 5876 = ~350 GB, measured on a four-tile sample
# (45.3 / 76.6 / 42.6 / 74.2 MB). The zip is deleted once its raster is
# extracted and verified, so only the rasters accumulate.
#
# Resumable at tile granularity: a tile with a verified raster is skipped, a
# partial zip is discarded rather than resumed. Interrupt and re-run freely.
#
# Run via:
#   nu ~/fm/freemap-outdoor-map/download-en.nu
# A clean exit means every tile in the index is present and content-verified.

# ── Configuration ─────────────────────────────────────────────────────────────

const DATA_DIR = "/media/martin/18TB/en"
const INDEX    = "/media/martin/18TB/en/index/composite_dtm_1m_tiles.json"
const DEST     = "/media/martin/18TB/en/DTM1"      # extracted 1 m rasters
const WORK     = "/media/martin/18TB/en/_work"     # zips + extraction scratch
const BUILDER  = "/home/martin/fm/freemap-outdoor-map/build-index-en.py"
const HOST     = "https://environment.data.gov.uk"
const NODATA   = "-3.4028235e+38"                  # as delivered; see header
const PARALLEL = 6                                 # polite to a public gov API
const RETRIES  = 3


# ── Mount guard ───────────────────────────────────────────────────────────────
# The 18TB is removable and udisks2 has already moved it once (/media/... ->
# /run/media/... across the 2026-08-16 reboot, with the device letter shifting
# too). The old path survives as an empty root-owned directory on /, which has
# far less free space than this script needs — so a stale path does not fail, it
# silently fills the root filesystem. Fail fast instead.
def assert-mounted [p: string]: nothing -> nothing {
    let res = (do { mountpoint -q $p } | complete)
    if $res.exit_code != 0 {
        error make {msg: $"($p) is not a mountpoint — the 18TB drive is not mounted there. Check `lsblk -o NAME,LABEL,SIZE,MOUNTPOINT` \(label 18TB\) and repoint the paths in this script."}
    }
}

assert-mounted "/media/martin/18TB"

# ── Helpers ───────────────────────────────────────────────────────────────────

def local-size [f: string]: nothing -> int {
    if ($f | path exists) { (ls -l $f | get 0.size | into int) } else { 0 }
}

def host-up []: nothing -> bool {
    let res = (do { curl -sI --max-time 25 -o /dev/null $"($HOST)/" } | complete)
    $res.exit_code == 0
}

# Strict content check. Any ERROR fails, and a run that produces no Checksum=
# line fails too — with only some blocks undecodable GDAL prints both.
def raster-ok [f: string]: nothing -> bool {
    let res = (do { gdalinfo -checksum $f } | complete)
    if $res.exit_code != 0 { return false }
    let out = $"($res.stdout)($res.stderr)"
    if ($out | str contains "ERROR") { return false }
    ($out | str contains "Checksum=")
}

# Fetch one zip whole. True only if it arrived AND its CRCs check out.
# A partial is discarded, never resumed.
def fetch-zip [url: string, dst: string]: nothing -> bool {
    for attempt in 1..$RETRIES {
        rm -f $dst
        # The server builds the zip on demand, so the first bytes can be a while
        # coming; --max-time has to cover preparation as well as transfer.
        let res = (
            do {
                (curl -sSL --fail --retry 5 --retry-delay 5 --retry-all-errors
                   --max-time 1800 -o $dst $url)
            } | complete
        )
        if $res.exit_code == 0 and (local-size $dst) > 0 {
            let t = (do { unzip -t -q $dst } | complete)
            if $t.exit_code == 0 { return true }
            print $"      zip CRC failed on attempt ($attempt)"
        } else {
            print $"      curl exit ($res.exit_code) on attempt ($attempt)"
        }
    }
    rm -f $dst
    false
}

# Unpack one verified zip, keeping only the GeoTIFF, renamed to the tile id.
# Returns the raster path, or null if the zip held no .tif.
def unpack [zip: string, tile: string]: nothing -> any {
    let d = $"($WORK)/x_($tile)"
    rm -rf $d
    mkdir $d

    let u = (do { unzip -o -q -d $d $zip } | complete)
    if $u.exit_code != 0 {
        rm -rf $d
        return null
    }

    # `.tif` only — never `.tif.aux.xml`, which also ends in a known extension
    # but is a sidecar. Largest wins if a delivery ever ships more than one.
    let tifs = (
        glob $"($d)/**/*.tif"
          | where {|f| ($f | path type) == "file" }
          | each {|f| {f: $f, n: (local-size $f)} }
          | sort-by n
    )
    if ($tifs | is-empty) {
        rm -rf $d
        return null
    }

    let out = $"($DEST)/($tile).tif"
    mv -f ($tifs | last | get f) $out
    rm -rf $d
    $out
}

# Does the tile carry its own CRS and its own nodata? Both are assumed by
# stage 0 of the shading script; a tile missing either would poison the mosaic
# quietly, so it is checked here where it is cheap to act on.
def tile-flags [f: string]: nothing -> record {
    let j = (do { gdalinfo -json $f } | complete)
    if $j.exit_code != 0 { return {crs: false, nodata: false} }
    let d = ($j.stdout | from json)
    {
        crs:    (($d | get -o coordinateSystem.wkt | default "") | is-not-empty)
        nodata: (($d.bands | first | get -o noDataValue) != null)
    }
}

# Everything one tile needs. Any failure leaves nothing behind, so the next run
# simply retries it.
def get-tile [t: record, i: int, n: int]: nothing -> nothing {
    let zip = $"($WORK)/($t.tile).zip"

    if not (fetch-zip $t.url $zip) {
        print $"  [($i)/($n)] ($t.tile): FAILED to fetch — re-run to retry"
        return
    }

    let raster = (unpack $zip $t.tile)
    if $raster == null {
        print $"  [($i)/($n)] ($t.tile): zip held no .tif — kept for inspection"
        mv -f $zip $"($WORK)/unexpected_($t.tile).zip"
        return
    }

    if not (raster-ok $raster) {
        print $"  [($i)/($n)] ($t.tile): raster failed gdalinfo — discarding, re-run to retry"
        rm -f $raster
        rm -f $zip
        return
    }

    rm -f $zip
    print $"  [($i)/($n)] ($t.tile): ok"
}

# ── 0. Index ──────────────────────────────────────────────────────────────────

mkdir $DATA_DIR
mkdir $DEST
mkdir $WORK

if not ($INDEX | path exists) {
    print $"==> ($INDEX) missing — building it \(957 search calls, a few minutes\)"
    python3 $BUILDER
    if not ($INDEX | path exists) {
        error make {msg: $"($BUILDER) did not produce ($INDEX) — see index_gaps.json"}
    }
}

let idx   = (open --raw $INDEX | from json)
let tiles = ($idx.tiles | each {|t| {tile: $t.tile, url: $t.url} })
print $"==> ($tiles | length) tiles: ($idx.product) ($idx.year) ($idx.resolution)m"

let have    = (glob $"($DEST)/*.tif" | each {|f| $f | path parse | get stem })
let pending = ($tiles | where {|t| $t.tile not-in $have })
print $"==> ($have | length) already verified, ($pending | length) to fetch \(~($pending | length | $in * 60 / 1024 | math round) GB\)"

# ── 1. Download ───────────────────────────────────────────────────────────────

if ($pending | is-empty) {
    print "==> Nothing to fetch."
} else if (not (host-up)) {
    error make {msg: $"($HOST) is not answering — nothing can be downloaded right now"}
} else {
    print $"==> Fetching with ($PARALLEL) workers"
    let n = ($pending | length)
    $pending | enumerate | par-each -t $PARALLEL {|it| get-tile $it.item ($it.index + 1) $n }
}

# ── 2. Verify the whole delivery ──────────────────────────────────────────────

print "==> Verifying every extracted raster (strict: any ERROR fails)"

let present = (glob $"($DEST)/*.tif")
let checked = (
    $present | par-each -t $PARALLEL {|f|
        {file: $f, ok: (raster-ok $f), flags: (tile-flags $f)}
    }
)
let corrupt = ($checked | where ok == false)
let stems   = ($present | each {|f| $f | path parse | get stem })
let missing = ($tiles | where {|t| $t.tile not-in $stems })

print $"    present: ($present | length) of ($tiles | length)   corrupt: ($corrupt | length)   missing: ($missing | length)"

if ($corrupt | is-not-empty) {
    print "==> Corrupt rasters — deleting so the next run re-fetches:"
    $corrupt | first 20 | each {|c| print $"    ($c.file | path basename)" }
    $corrupt | each {|c| rm -f $c.file }
}

# ── 3. The delivery-quirk checks that stage 0 depends on ──────────────────────
# Both of these are cheap here and expensive later: an undeclared nodata reads
# as real terrain and renders as a cliff, and a CRS-less tile silently lands in
# the wrong place in the mosaic.

let sound   = ($checked | where ok == true)
let no_crs  = ($sound | where {|c| not $c.flags.crs })
let no_nd   = ($sound | where {|c| not $c.flags.nodata })

if ($no_crs | is-empty) {
    print $"    CRS:    declared on all ($sound | length) tiles"
} else {
    print $"    !! CRS MISSING on ($no_crs | length) tile\(s\) — gdalbuildvrt will misplace them."
    print $"       Stage 0 must pass -a_srs EPSG:27700 for these, as shading-hr.nu does."
    $no_crs | first 10 | each {|c| print $"         ($c.file | path basename)" }
}

if ($no_nd | is-empty) {
    print $"    nodata: declared on all ($sound | length) tiles"
} else {
    print $"    !! NODATA UNDECLARED on ($no_nd | length) tile\(s\). Voids will read as real"
    print $"       terrain. Do NOT work around this by thresholding on very negative"
    print $"       values — genuine elevations here go below zero \(a Devon tile bottoms"
    print $"       out at -6.06 m\). Inspect these before building the VRT."
    $no_nd | first 10 | each {|c| print $"         ($c.file | path basename)" }
}

let unexpected = (glob $"($WORK)/unexpected_*.zip")
if ($unexpected | is-not-empty) {
    print $"==> ($unexpected | length) zip\(s\) held no .tif and were kept in ($WORK)"
    $unexpected | first 10 | each {|f| print $"    ($f | path basename)" }
}

# ── 4. Verdict ────────────────────────────────────────────────────────────────

if ($missing | is-empty) and ($corrupt | is-empty) {
    print ""
    print $"==> Complete and verified: ($present | length) tiles in ($DEST)"
    print $"    Next: shading-en.nu. Stage 0 is a plain gdalbuildvrt with"
    print $"    -vrtnodata -9999 to unify the delivered ($NODATA) sentinel,"
    print $"    the same mechanism shading-no.nu uses for -32767."
} else {
    print ""
    print $"==> Incomplete: ($missing | length) missing, ($corrupt | length) corrupt \(deleted\) — re-run to fetch them"
    exit 1
}
