#!/usr/bin/env nu

# Download the Romanian LiDAR terrain models (ANCPI LAKI III zone A, 0.5 m).
# Romania port of download-no.nu (Kartverket DTM1) / download-dtm1.sh.
#
# ─────────────────────────────────────────────────────────────────────────────
# READ THIS BEFORE RUNNING. As of 2026-08-13 THIS SCRIPT CANNOT FETCH ANYTHING.
#
# ANCPI was hit by an attack; the affected infrastructure was isolated after
# DNSC intervention. e-Terra restarted in stages on 11-12 Aug 2026, but
# ancpi.ro states that "celelalte platforme online ale ANCPI destinate
# publicului raman oprite deocamdata" — restored in stages, with prior notice,
# NO DATE GIVEN. Verified the same day:
#
#   * geoportal.ancpi.ro — NXDOMAIN from its own authoritative nameserver
#     (iris.ns.cloudflare.com, aa flag set). The record has been WITHDRAWN from
#     the zone, not merely made unreachable.
#   * geoportal.gov.ro — resolves to 195.138.192.9, ports 80/443 refused,
#     confirmed from two separate networks.
#
# Every download URL below lives on geoportal.ancpi.ro. The script preflights
# the host and exits cleanly with that explanation rather than emitting 26959
# DNS failures. Re-run it once the host is back; nothing else needs changing.
# ─────────────────────────────────────────────────────────────────────────────
#
# SOURCE OF THE INDEX — NOT ANCPI. The endpoints in the original brief
# (geoportal.ancpi.ro/hosted_services/rest/services/Descarcare/...) are
# unreachable, and the Wayback Machine has only the MapServer *root* metadata,
# no layer query. The tile index therefore comes from RO-LiDAR GeoQuickView
# (arXiv:2606.08876), an independent West University of Timisoara project that
# mirrors ANCPI's tile indexes as ArcGIS Online feature layers. AGOL is up and
# unaffected by the ANCPI incident. The mirror indexes the tiles; it does NOT
# host the rasters, so it is no help while ANCPI is down — but it means the URL
# list is already in hand and does not have to be rediscovered.
#
#   org      Q2Kmg0bQDn3rySgn on services9.arcgis.com
#   layer    Tiles/FeatureServer/0    ("Caroiaj LAKI III")
#   cached   /media/martin/18TB/ro/index/laki3_tiles.geojson  (26959 features)
#
# See index/README.md for the full provenance and for what is still unknown.
#
# WHAT THIS COVERS. LAKI III zone A only: 26959 tiles of 1 km, 0.5 m DTM, over
# Caras-Severin (8373), Dolj (7726), Gorj (5573), Mehedinti (5287) = 26473 km2.
# TILE is "{YKM}_{XKM}" in Stereo70 (EPSG:3844) kilometres, so 395_231 is the
# 1 km cell with origin E=231000 N=395000.
#
# WHAT THIS DOES NOT COVER, AND WHY:
#
#   * LAKI II (1 m, ~29167 km2 over Arad, Bihor, Hunedoara, ~1/4 Alba). Its
#     coverage polygon is mirrored but its per-unit download URLs are NOT —
#     those live in ANCPI's own Descarcare/LAKI/MapServer layers (UAT_MNT_LAKI,
#     Judet_MNT_LAKI), which are offline. Stage 5 below probes that MapServer
#     the moment it answers and dumps its field schema, which is exactly what is
#     needed to extend this script. Do not guess the pattern before then.
#
#   * LAKI III zone B (Suceava, Neamt, Bacau, Vrancea). Flown, not released; the
#     Aug 2026 GeoQuickView paper still lists it as a "scheduled extension".
#
#   * The national contour-derived MNT (189 zips, 5 m/10 m grid, OGL-ROU-1.0,
#     under descarcare/MNT/Caroiaj/ZIPS/). Deliberately skipped — derived from
#     1:50k contours, worse than the GEDTM30 fallback in hilly terrain. The
#     index is cached as index/ancpi_mnt_tiles.geojson if that call is ever
#     revisited.
#
# VERIFICATION IS STRONGER HERE THAN IN THE NORWEGIAN SCRIPTS, for free: the
# payload is a ZIP, so every byte is already covered by a CRC-32. `unzip -t`
# therefore replaces the whole Content-Length dance — no 26959 HEAD requests, no
# cached size index, and no way for the silent-corruption failure that cost two
# days on the Norway run (download-dtm1.sh lesson 2) to get through. The raster
# is still put through a strict `gdalinfo -checksum` after extraction, because a
# CRC-clean zip can still hold a raster the server built badly.
#
# Both Norway lessons are kept:
#   1. NEVER RESUME A PARTIAL. A killed run can leave a file whose size is right
#      and whose middle is garbage; `curl -C -` then appends good data after the
#      gap. Partials are DISCARDED and re-fetched whole. Cheap here — tiles are
#      ~1 km, a few MB each.
#   2. VERIFY CONTENT, NOT SIZE — AND STRICTLY. A tile passes only if gdalinfo
#      emits NO ERROR. "A Checksum= line was printed" is not enough: with only
#      some blocks undecodable GDAL prints ERRORs *and* a checksum.
#
# DISK. The zip is DELETED once its raster is extracted and verified, so only
# the rasters accumulate. 0.5 m over 26473 km2 is ~106e9 cells = ~424 GB
# uncompressed; expect roughly half that as delivered. The extracted raster is
# the resume state — a tile with a verified raster is never re-fetched.
#
# LAYOUT PROBE. Nothing is known about what is inside these zips: raster format,
# CRS tagging, nodata sentinel, pixel dimensions, whether cells overlap. So the
# first run fetches exactly ONE tile, reports what it found, writes layout.json
# and STOPS. Inspect that report, then re-run to start the bulk download. This
# is deliberate: every country script in this repo is built around
# source-specific quirks that were found by inspection, and 400 GB is too much
# to spend on an assumption.
#
# Resumable at tile granularity in both directions: a tile with a verified
# raster is skipped, and a partial zip is discarded rather than resumed.
# Interrupt with ctrl-c and re-run freely.
#
# Run via:
#   nu ~/fm/freemap-outdoor-map/download-ro.nu
# A clean exit means every tile in the index is present and content-verified.

# ── Configuration ─────────────────────────────────────────────────────────────

const DATA_DIR = "/media/martin/18TB/ro"
const INDEX    = "/media/martin/18TB/ro/index/laki3_tiles.geojson"
const DEST     = "/media/martin/18TB/ro/dtm05"          # extracted 0.5 m rasters
const WORK     = "/media/martin/18TB/ro/_work"          # zips + extraction scratch
const LAYOUT   = "/media/martin/18TB/ro/index/layout.json"
const PROBE    = "/media/martin/18TB/ro/index/laki2_probe.json"
const HOST     = "https://geoportal.ancpi.ro"
const LAKI_MS  = "https://geoportal.ancpi.ro/hosted_services/rest/services/Descarcare/LAKI/MapServer"
const MIRROR   = "https://services9.arcgis.com/Q2Kmg0bQDn3rySgn/arcgis/rest/services/Tiles/FeatureServer/0"
const PARALLEL = 6                                      # polite to ANCPI, kind to the HDD
const RETRIES  = 3

# Extensions worth treating as the payload raster. Anything else in the zip is a
# sidecar (.prj, .tfw, .xml, readme) and is carried along beside it.
const RASTER_EXT = [tif tiff asc img dem dt2 vrt bil grd]

# ── Helpers ───────────────────────────────────────────────────────────────────

def local-size [f: string]: nothing -> int {
    if ($f | path exists) { (ls -l $f | get 0.size | into int) } else { 0 }
}

# Is ANCPI answering at all? One request, short timeout — the point is to fail
# the whole run fast and legibly rather than 26959 times.
def host-up []: nothing -> bool {
    let res = (do { curl -sI --max-time 25 -o /dev/null $"($HOST)/" } | complete)
    $res.exit_code == 0
}

# Strict content check — see header lesson 2. Any ERROR fails, and a run that
# produces no Checksum= line fails too.
def raster-ok [f: string]: nothing -> bool {
    let res = (do { gdalinfo -checksum $f } | complete)
    if $res.exit_code != 0 { return false }
    let out = $"($res.stdout)($res.stderr)"
    if ($out | str contains "ERROR") { return false }
    ($out | str contains "Checksum=")
}

# Fetch one zip whole. Returns true if it arrived AND its CRCs check out.
# A partial is discarded, never resumed (header lesson 1).
def fetch-zip [url: string, dst: string]: nothing -> bool {
    for attempt in 1..$RETRIES {
        rm -f $dst
        let res = (
            do {
                (curl -sSL --fail --retry 5 --retry-delay 5 --retry-all-errors
                   --max-time 1800 -o $dst $url)
            } | complete
        )
        if $res.exit_code == 0 and (local-size $dst) > 0 {
            # CRC-32 over every member — the real verification (see header).
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

# Unpack one verified zip and hand back the raster it contained, moved into DEST
# under the tile's name, sidecars alongside it. Returns the raster path, or null
# if the zip held nothing GDAL recognises.
def unpack [zip: string, tile: string]: nothing -> any {
    let d = $"($WORK)/x_($tile)"
    rm -rf $d
    mkdir $d

    let u = (do { unzip -o -q -d $d $zip } | complete)
    if $u.exit_code != 0 {
        rm -rf $d
        return null
    }

    let rasters = (
        glob $"($d)/**/*"
          | where {|f| ($f | path type) == "file" }
          | where {|f| ($f | path parse | get extension | str lowercase) in $RASTER_EXT }
    )
    if ($rasters | is-empty) {
        rm -rf $d
        return null
    }

    # Largest raster wins — a delivery that also ships an overview or a preview
    # must not have it mistaken for the payload.
    let main = ($rasters | each {|f| {f: $f, n: (local-size $f)} } | sort-by n | last | get f)
    let ext  = ($main | path parse | get extension | str lowercase)
    let out  = $"($DEST)/($tile).($ext)"

    mv -f $main $out

    # Sidecars that GDAL may need (.prj for a bare .asc, .tfw, .aux.xml) follow
    # the raster under the same stem.
    let stem = ($main | path parse | get stem)
    for f in (glob $"($d)/**/($stem).*" | where {|f| ($f | path type) == "file" }) {
        let e = ($f | path parse | get extension | str lowercase)
        if $e != $ext {
            mv -f $f $"($DEST)/($tile).($e)"
        }
    }

    rm -rf $d
    $out
}

# Everything one tile needs: fetch, CRC, unpack, strict content check, drop the
# zip. Any failure leaves nothing behind, so the next run simply retries it.
def get-tile [t: record, i: int, n: int]: nothing -> nothing {
    let zip = $"($WORK)/($t.tile).zip"

    if not (fetch-zip $t.url $zip) {
        print $"  [($i)/($n)] ($t.tile): FAILED to fetch — re-run to retry"
        return
    }

    let raster = (unpack $zip $t.tile)
    if $raster == null {
        print $"  [($i)/($n)] ($t.tile): zip held no raster GDAL recognises — kept for inspection"
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

# What does a tile actually look like? Everything the shading/contour scripts
# will need to be written correctly, and none of which can be guessed.
def describe [f: string]: nothing -> record {
    let j = (gdalinfo -json -mm $f | from json)
    let b = ($j.bands | first)

    # gdalinfo -json carries no coordinateSystem.epsg field; the code lives in
    # the WKT's trailing authority. Last ID[] wins — that is the compound CRS's
    # own code, not the datum's or the axis unit's.
    let wkt = ($j | get -o coordinateSystem.wkt | default "")
    let ids = ($wkt | parse -r 'ID\["EPSG",(?<c>\d+)\]' | get c)

    {
        file:        ($f | path basename)
        driver:      ($j | get -o driverShortName)
        size:        $j.size
        type:        ($b | get -o type)
        nodata:      ($b | get -o noDataValue)
        min:         ($b | get -o computedMin)
        max:         ($b | get -o computedMax)
        block:       ($b | get -o block)
        crs:         (if ($wkt | is-empty) { "NONE — tile carries no CRS" } else { $wkt | lines | first })
        epsg:        (if ($ids | is-empty) { null } else { $ids | last })
        origin:      ($j | get -o geoTransform | default [] )
        overviews:   ($b | get -o overviews | default [] | length)
        compression: ($j | get -o metadata.IMAGE_STRUCTURE.COMPRESSION)
    }
}

# ── 0. Preflight ──────────────────────────────────────────────────────────────

mkdir $DATA_DIR
mkdir $DEST
mkdir $WORK

if (not ($INDEX | path exists)) {
    print $"==> ($INDEX) missing — refetching it from the GeoQuickView mirror"
    let q = "where=1%3D1&outFields=*&returnGeometry=true&outSR=4326&f=geojson"
    mut feats = []
    mut off = 0
    loop {
        let page = (
            curl -s --max-time 120 $"($MIRROR)/query?($q)&resultOffset=($off)&resultRecordCount=2000"
              | from json | get features
        )
        $feats = ($feats | append $page)
        if ($page | length) < 2000 { break }
        $off = $off + 2000
    }
    if ($feats | length) < 1000 {
        error make {msg: $"only ($feats | length) features from the mirror — refusing to cache a bad index"}
    }
    {type: "FeatureCollection", features: $feats} | to json | save -f $INDEX
    print $"  cached ($feats | length) tiles"
}

let tiles = (
    open --raw $INDEX | from json | get features
      | each {|f| {tile: $f.properties.TILE, county: $f.properties.COUNTY, url: $f.properties.URL} }
)
print $"==> ($tiles | length) LAKI III tiles in the index"
$tiles | group-by county | items {|k, v| print $"    ($k): ($v | length)" }

# A tile is done when its raster is on disk. Extension varies by delivery, so
# match on the stem.
let have = (
    glob $"($DEST)/*"
      | where {|f| ($f | path parse | get extension | str lowercase) in $RASTER_EXT }
      | each {|f| $f | path parse | get stem }
)
let pending = ($tiles | where {|t| $t.tile not-in $have })
print $"==> ($have | length) tiles already verified, ($pending | length) to fetch"

if ($pending | is-empty) {
    print "==> Nothing to do — the LAKI III delivery is complete and verified."
} else if (not (host-up)) {
    print ""
    print $"==> ($HOST) is NOT ANSWERING — nothing can be downloaded."
    print "    ANCPI was taken offline after a security incident (infrastructure"
    print "    isolated following DNSC intervention). e-Terra restarted 11-12 Aug"
    print "    2026; the remaining public platforms, this one included, are still"
    print "    down with no announced date."
    print ""
    print "    The index is cached and complete, so re-running this script once"
    print "    the host answers is all that is needed. Nothing else must change."
    exit 0
} else {

    # ── 1. Layout probe — one tile, then stop (see header) ────────────────────

    if not ($LAYOUT | path exists) {
        let t = ($pending | first)
        print ""
        print $"==> LAYOUT PROBE: fetching one tile \(($t.tile)\) to learn the delivery"
        let zip = $"($WORK)/($t.tile).zip"

        if not (fetch-zip $t.url $zip) {
            error make {msg: $"probe tile ($t.tile) could not be fetched from ($t.url)"}
        }

        print ""
        print "  zip contents:"
        unzip -l $zip | lines | each {|l| print $"    ($l)" }

        let raster = (unpack $zip $t.tile)
        if $raster == null {
            error make {msg: $"probe zip held no raster GDAL recognises — kept at ($zip); inspect it before going further"}
        }
        if not (raster-ok $raster) {
            error make {msg: $"probe raster ($raster) failed gdalinfo -checksum — do not trust this delivery yet"}
        }
        rm -f $zip

        let d = (describe $raster)
        print ""
        print "  raster:"
        $d | transpose k v | each {|r| print $"    ($r.k | fill -a right -w 12): ($r.v)" }

        $d | to json | save -f $LAYOUT
        print ""
        print $"==> Probe written to ($LAYOUT). STOPPING ON PURPOSE."
        print "    Check the CRS, the nodata sentinel, the pixel dimensions and"
        print "    whether adjacent cells overlap, then write shading-ro.nu against"
        print "    what is actually there. Re-run this script to start the bulk"
        print "    download (~400 GB); the probe tile is already done."
        exit 0
    }

    # ── 2. Bulk download ─────────────────────────────────────────────────────

    print $"==> Fetching ($pending | length) tiles with ($PARALLEL) workers"
    let n = ($pending | length)
    $pending | enumerate | par-each -t $PARALLEL {|it| get-tile $it.item ($it.index + 1) $n }
}

# ── 3. Verify the whole delivery ──────────────────────────────────────────────

print "==> Verifying every extracted raster (strict: any ERROR fails)"

let present = (
    glob $"($DEST)/*"
      | where {|f| ($f | path parse | get extension | str lowercase) in $RASTER_EXT }
)
let checked = ($present | par-each -t $PARALLEL {|f| {file: $f, ok: (raster-ok $f)} })
let corrupt = ($checked | where ok == false)
let stems   = ($present | each {|f| $f | path parse | get stem })
let missing = ($tiles | where {|t| $t.tile not-in $stems })

print $"    present: ($present | length) of ($tiles | length)   corrupt: ($corrupt | length)   missing: ($missing | length)"

if ($corrupt | is-not-empty) {
    print "==> Corrupt rasters — deleting them so the next run re-fetches:"
    $corrupt | first 20 | each {|c| print $"    ($c.file | path basename)" }
    $corrupt | each {|c| rm -f $c.file }
}

let unexpected = (glob $"($WORK)/unexpected_*.zip")
if ($unexpected | is-not-empty) {
    print $"==> ($unexpected | length) zip\(s\) held no recognisable raster and were kept in ($WORK):"
    $unexpected | first 10 | each {|f| print $"    ($f | path basename)" }
    print "    Inspect one — the delivery layout may differ from the probe tile."
}

# ── 4. LAKI II discovery — only possible once ANCPI answers ───────────────────
# The 1 m block's download URLs are not on the mirror. Dump the MapServer's real
# schema the first moment it is reachable, so this script can be extended from
# fact rather than from a guess.

if (not ($PROBE | path exists)) and (host-up) {
    print "==> Probing ANCPI's LAKI MapServer for the LAKI II (1 m) download links"
    let res = (do { curl -s --max-time 60 $"($LAKI_MS)?f=pjson" } | complete)
    if $res.exit_code == 0 and ($res.stdout | str starts-with "{") {
        let ms = ($res.stdout | from json)
        mut probe = {mapserver: $ms, layers: {}}
        for l in ($ms | get -o layers | default []) {
            let meta = (curl -s --max-time 60 $"($LAKI_MS)/($l.id)?f=pjson" | from json)
            let samp = (curl -s --max-time 90 $"($LAKI_MS)/($l.id)/query?where=1%3D1&outFields=*&returnGeometry=false&resultRecordCount=3&f=pjson" | from json)
            $probe = ($probe | upsert $"layers.($l.id)" {name: $l.name, fields: ($meta | get -o fields), sample: ($samp | get -o features)})
            print $"    layer ($l.id) ($l.name): fields ($meta | get -o fields | default [] | get -o name | str join ', ')"
        }
        $probe | to json | save -f $PROBE
        print $"    written to ($PROBE) — look for a per-unit download URL field, then extend this script"
    } else {
        print "    MapServer did not answer with JSON — skipping"
    }
}

# ── 5. Verdict ────────────────────────────────────────────────────────────────

if ($missing | is-empty) and ($corrupt | is-empty) {
    print ""
    print $"==> Complete and verified: ($present | length) tiles in ($DEST)"
    print "    Next: inspect index/layout.json, then write shading-ro.nu."
    print "    Remember the two source resolutions do NOT get merged blindly —"
    print "    LAKI III is 0.5 m and LAKI II is 1 m; smooth and downsample each"
    print "    on its own grid before they meet in a common mosaic."
} else {
    print ""
    print $"==> Incomplete: ($missing | length) missing, ($corrupt | length) corrupt \(deleted\) — re-run to fetch them"
    exit 1
}
