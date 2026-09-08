#!/usr/bin/env nu

# Generate shaded relief for Luxembourg from the Administration du cadastre et de
# la topographie's LiDAR 2024 MNT. Luxembourg port of shading-en.nu (England 1 m).
#
# Source: ONE national Cloud-Optimized GeoTIFF, not a tile directory. This is the
#   structural difference from every previous country and it removes two whole
#   stages: there is no downloader, no per-tile verification, no national VRT and
#   no tile_origins.tsv. Step 0 below fetches the raster in a single command and
#   the window grid is derived from its own extent.
#
#   https://data.public.lu/en/datasets/lidar-2019-modele-numerique-de-terrain-mnt/
#   (the 2024 campaign is published under the same dataset family; see LICENCE)
#
#   Survey flown 2024 at >30 points/m2, vertical precision <10 cm (sigma 5.5 cm) —
#   double the 2019 density. Next acquisition planned winter 2027-2028, so this is
#   current for years. MNT = bare terrain; MNS is the surface model, do not use it.
#
# LICENCE: CC0. No attribution obligation at all — the only country here with no
#   credit line to carry. (Contrast England's mandatory verbatim Environment
#   Agency wording.) Crediting ACT anyway is good manners, not a condition.
#
# WE TAKE THE 1 m OVERVIEW, NOT THE 0.5 m FULL RESOLUTION. The delivered COG is
#   0.5 m / 114617 x 163687 / Float64 / 38.3 GB. Its first overview level is
#   exactly half — 57308 x 81843 at 1.000 m — so `-oo OVERVIEW_LEVEL=0` reads the
#   1 m grid directly over HTTP range requests and never transfers the 0.5 m data
#   at all. 4.8 GB fetched in 11 min against 38.3 GB.
#
#   Float64 -> Float32 on the way in: elevation here spans ~130-560 m, where
#   Float32 resolves ~3e-5 m. Halves the volume, loses nothing.
#
#   Why not 0.5 m: --filter 11 is a PIXEL count, so on 0.5 m data it would smooth
#   over 5.5 m instead of 11 m, i.e. a different (weaker) filter for the same
#   nominal setting — and 4x the pixels to render z17 that is already finer than
#   the smoothed signal. 1 m keeps the filter comparable with PL/HR/NO/EN.
#
# ZOOM=17, chosen visually, against the measurement. Measured first, on smoothed
#   data through this exact pipeline (Mullerthal, the roughest ground here):
#
#     z15 -> z17   mean 1.81/255   p95 5.0    4.83% of pixels off by >5
#     z16 -> z17   mean 1.03/255   p95 3.0    2.57% of pixels off by >5
#
#   2.57% is England's z16 figure (3.2%) — by the England rule that says z16.
#   BUT the samples were then looked at, and z17 read as visibly better; it also
#   matches Slovakia, the other 1 m country (sk/final.tif is 1.1943285 m/px).
#   The cost argument that decided England does not apply at this size: 2.5 GB
#   against 632 MB, versus England's 137 GB against 44 GB.
#
#   BEWARE the measurement trap that produced a WRONG first recommendation here:
#   differencing UNSMOOTHED renders gave z16 -> z17 at 8.5%, which argues loudly
#   for z17 on false grounds. --filter 11 strips most sub-11 m variation BEFORE
#   the hillshade, so any zoom sample must smooth first or it measures detail
#   that never reaches the output. Same trap for any future country.
#
#   MIND THE UNITS. A z17 pixel here is 0.77 GROUND metres, not the 1.194 m that
#   -tr says: Mercator pixels shrink as cos(lat), and at 49.8 degN that is 0.645.
#
# DATUM: NO GRID SHIFT EXISTS, AND NONE IS NEEDED. Checked with projinfo —
#   EPSG:2169 -> EPSG:4326 offers exactly two candidate operations:
#
#     LUREF to WGS 84 (4)   Molodensky-Badekas   1 m
#     LUREF to WGS 84 (3)   7-parameter Helmert  1 m
#
#   Both are stated at 1 m and NEITHER references an NTv2 grid — there is no
#   Luxembourg equivalent of OSTN15 to install, so the England hazard (PROJ
#   silently falling back to a 2 m Helmert, leaving products ~1.9 m off and
#   creating a mixed-datum trap on partial re-render) cannot arise. A z17 pixel
#   is 0.77 ground metres, so the residual 1 m is ~1.3 px and, unlike England's,
#   it is the best available rather than a fixable mistake. No action.
#
# NO gdal_fillnodata, UNLIKE EVERY OTHER COUNTRY. Measured, not assumed: 60
#   random 1200x1200 windows, 34 of them inland, 49 Mpx. 0.75% of inland pixels
#   are nodata, in 4 of 34 windows, and EVERY void blob is 17k-173k px — open
#   water (Sure reservoir, Moselle, lakes). Not one blob was <= 25 px. So
#   `-md 5` has nothing to fill and would leave the raster byte-identical while
#   costing a read+write per window. Those large water voids must stay nodata so
#   they come out transparent, which is what -md 5 does with them anyway.
#   If a future re-survey introduces speckle voids, restore the step with:
#     gdal_fillnodata.py -md 5 $smooth $dem
#   and point step 5 at $dem instead of $smooth.
#
# PREDICTOR=1 on the window DEM is LOAD-BEARING, not a style choice.
#   feature-preserving-smoothing does I/O via the `wbgeotiff` crate, which does
#   not parse the TIFF Predictor tag (317) — fed PREDICTOR=2/3 float data it
#   decodes byte-shuffled deltas as garbage and emits +/-Inf rasters that render
#   as static, WITHOUT erroring. (Documented in shading-it.nu, verified 2026-07-17.)
#   NOTE the national raster written by step 0 IS PREDICTOR=3; that is fine
#   because the smoother never sees it, only the PREDICTOR=1 window cuts.
#
# ALWAYS RUN VIA `conda run -n geo`, NEVER by putting the env's bin on PATH.
#   Doing the latter leaves PROJ_DATA unset, and then gdal_fillnodata.py /
#   gdal_edit.py degrade a PROJCRS to ENGCRS["unnamed"] and the warp fails with
#   "Cannot find coordinate operations". That looks exactly like a GDAL bug and
#   is not one. (Diagnosed the hard way, 2026-08-20.)
#
# Smoothing is 11/16/6/6, the Poland/Croatia/Norway/England 1 m settings — the
#   filter is a pixel count, so 11 px = 11 m on 1 m data.
#
# CONTOURS. smooth/ is not kept, so each window also emits a 2 m downsample of its
#   smoothed DEM into smooth2m/, which contours-lu.nu contours directly.
#   `average` resampling is nodata-aware; bilinear/cubic would blend nodata into
#   its neighbours.
#
# Resumable at window granularity: a window whose tiles/<id>.tif exists is
# skipped, and an all-nodata window leaves a tiles/<id>.empty marker so it is not
# re-cut on the next run. ~45% of the 759 windows are outside the border and land
# as .empty on the first pass. A window that throws is recorded in failed/ and
# skipped. Run via:
#   nice ~/miniforge3/bin/conda run --no-capture-output -n geo nu ~/fm/freemap-outdoor-map/shading-lu.nu

# ── Configuration ─────────────────────────────────────────────────────────────

const SRC       = "/media/martin/18TB/lu/luxembourg_dem_1m.tif"   # 1 m national DEM (step 0)
const SRC_URL   = "/vsicurl/https://download.data.public.lu/resources/bd-l-lidar2024-releve-3d-du-territoire-luxembourgeois/20241223-093912/MNT_Lidar2024.tif"
const DATA_DIR  = "/mnt/osm/lu"                  # origin cache, smooth2m/ on NVMe
const TILES_DIR = "/mnt/osm/lu/tiles"            # NVMe — many small files, see EN header
const OUT_TIF   = "/media/martin/18TB/lu/shading.tif"
const EPSG      = "EPSG:2169"                    # LUREF / Luxembourg TM
const NODATA    = "-9999"
const ZOOM      = 17                             # see header — visual, matches sk
const PARALLEL  = 24
const TMPDIR    = "/dev/shm"

const STEP      = 2500                           # window size, m (= px at 1 m)
const COLLAR    = 6                              # m cut beyond the window on each side
const CROP      = 3                              # px cropped per edge after hillshading

# Smoothing (Poland/Croatia/Norway/England 1 m settings — filter is a pixel count)
const SM_FILTER    = 11
const SM_NORM_DIFF = 16
const SM_NUM_ITER  = 6
const SM_MAX_DIFF  = 6


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

# PROJ sanity — see the conda-run note in the header. A missing PROJ database
# does not fail loudly, it degrades every CRS to ENGCRS and breaks the warp
# 40 minutes into the run.
let _probe = (do { gdalsrsinfo -o proj4 $EPSG } | complete)
if $_probe.exit_code != 0 or ($_probe.stdout | str trim | is-empty) {
    error make {msg: $"PROJ cannot resolve ($EPSG) — you are probably not inside the geo env. Run via: nice ~/miniforge3/bin/conda run --no-capture-output -n geo nu ($env.CURRENT_FILE? | default 'shading-lu.nu')"}
}

# ── Helpers ───────────────────────────────────────────────────────────────────

# Does the window contain any real data? A FALSE here writes a permanent .empty
# marker, so it must mean "gdalinfo ran and found no valid pixel" and never
# "gdalinfo did not run". Treating a transient failure as no-data silently drops
# a window from the country forever. Throwing instead routes it to failed/.
def has-data [file: string]: nothing -> bool {
    let res = (do { gdalinfo -json -mm $file } | complete)
    if $res.exit_code != 0 {
        error make {msg: $"gdalinfo failed on ($file) — refusing to call it empty"}
    }
    let bands = ($res.stdout | from json | get -o bands | default [])
    if ($bands | is-empty) {
        error make {msg: $"gdalinfo reported no bands for ($file) — refusing to call it empty"}
    }
    (($bands | first) | get -o computedMin | is-not-empty)
}

# Weighted multi-directional hillshade blend formula for one RGB band.
def band-calc [wa: string, wb: string, wc: string]: nothing -> string {
    let ea  = "0.8 * (255 - A)"
    let eb  = "0.7 * (255 - B)"
    let ec  = "1.0 * (255 - C)"
    let num = $"($ea) * ($wa) + ($eb) * ($wb) + ($ec) * ($wc)"
    let den = $"0.01 + ($ea) + ($eb) + ($ec)"
    "((" + $num + ") / (" + $den + ") - 128.0) + 128.0"
}

# Alpha channel: inverse of "all directions dark simultaneously".
def alpha-calc []: nothing -> string {
    let ea = "0.8 * (255 - A)"
    let eb = "0.7 * (255 - B)"
    let ec = "1.0 * (255 - C)"
    "255.0 - 255.0 * ((1.0 - " + $ea + " / 255.0) * (1.0 - " + $eb + " / 255.0) * (1.0 - " + $ec + " / 255.0))"
}

# Cut one window (plus collar) out of the national raster, smooth it, emit the
# 2 m DEM for contours, and render the RGBA shaded relief tile. All intermediates
# live in TMPDIR and are removed on the way out, so disk never accumulates.
def render-window [w: record, tr: string]: nothing -> nothing {
    let out = $"($TILES_DIR)/($w.id).tif"
    let d   = $"($TMPDIR)/lu_($w.id)"

    rm -rf $d
    mkdir $d

    let co      = [-co COMPRESS=ZSTD -co PREDICTOR=2 -co TILED=YES -co NUM_THREADS=ALL_CPUS]
    let co_big  = [...$co -co BIGTIFF=YES]
    let co_calc = [--co=COMPRESS=ZSTD --co=PREDICTOR=2 --co=TILED=YES --co=NUM_THREADS=ALL_CPUS --co=BIGTIFF=YES]

    # 1. Cut window + collar. PREDICTOR=1 — see header. Bounds are pre-clamped to
    #    the raster extent by the window builder, so -projwin never overhangs.
    let win = $"($d)/win.tif"
    (gdal_translate -q -of GTiff
      -projwin $w.cxmin $w.cymax $w.cxmax $w.cymin
      -co COMPRESS=DEFLATE -co PREDICTOR=1 -co TILED=YES
      $SRC $win o> /dev/null)

    # An all-nodata window (outside the border, or open water) leaves a marker so
    # the next run skips it without re-cutting.
    if not (has-data $win) {
        rm -rf $d
        touch $"($TILES_DIR)/($w.id).empty"
        print $"  ($w.id): empty"
        return
    }

    # 2. Smooth. A zero-variance window makes the smoother panic and has no relief
    #    to smooth anyway — pass it through.
    let band = gdalinfo -json -mm $win err> /dev/null | from json | get bands | first
    let smooth = $"($d)/smooth.tif"
    if ($band.computedMin? == $band.computedMax?) {
        cp $win $smooth
    } else {
        (feature-preserving-smoothing --dem $win -o $smooth
          --filter $SM_FILTER --norm_diff $SM_NORM_DIFF
          --num_iter $SM_NUM_ITER --max_diff $SM_MAX_DIFF)
    }

    # 3. 2 m DEM for contours-lu.nu — collar cropped, nodata-aware `average`.
    let dem2 = $"($DATA_DIR)/smooth2m/($w.id).tif"
    if not ($dem2 | path exists) {
        let tmp2 = $"($d)/dem2m.tif"
        (gdal_translate -q -of GTiff
          -projwin $w.dxmin $w.dymax $w.dxmax $w.dymin
          -tr 2 2 -r average -a_nodata $NODATA
          ...$co $smooth $tmp2 o> /dev/null)
        mv $tmp2 $dem2
    }

    # 4. (no gdal_fillnodata — measured unnecessary here, see header)

    # 5. Three Igor hillshades at different azimuths, on the collared window so
    #    edges have real neighbours.
    gdaldem hillshade $smooth $"($d)/_a.tif" -az -120 -igor -compute_edges ...$co o> /dev/null
    gdaldem hillshade $smooth $"($d)/_b.tif" -az  60  -igor -compute_edges ...$co o> /dev/null
    gdaldem hillshade $smooth $"($d)/_c.tif" -az -45  -igor -compute_edges ...$co o> /dev/null

    # 6. Crop the collar (minus the warp margin) off each hillshade.
    let info = gdalinfo -json $smooth | from json
    let w_px = $info.size.0
    let h_px = $info.size.1
    for name in [a b c] {
        let raw = $"($d)/_($name)_raw.tif"
        mv $"($d)/_($name).tif" $raw
        (gdal_translate -q -srcwin $CROP $CROP ($w_px - 2 * $CROP) ($h_px - 2 * $CROP)
          ...$co $raw $"($d)/_($name).tif" o> /dev/null)
        rm $raw
    }

    # 7. Warp each to EPSG:3857 at zoom-level pixel size. -tap aligns every window
    #    to the same global grid, so the tiles mosaic without seams.
    for name in [a b c] {
        (gdalwarp -t_srs EPSG:3857 -tr $tr $tr -tap -r cubic -dstnodata none -of GTiff
          ...$co_big -multi -wo NUM_THREADS=ALL_CPUS -wo INIT_DEST=0
          $"($d)/_($name).tif" $"($d)/($name)-warped.tif" o> /dev/null)
    }

    # 8. RGBA from the three warped hillshades.
    let inputs = [-A $"($d)/a-warped.tif" -B $"($d)/b-warped.tif" -C $"($d)/c-warped.tif"]

    #                       [a]    [b]    [c]
    let r_calc = band-calc "0x20" "0xFF" "0x00"
    let g_calc = band-calc "0x30" "0xEE" "0x00"
    let b_calc = band-calc "0x60" "0x00" "0x00"
    let a_calc = alpha-calc

    gdal_calc.py ...$inputs ...$co_calc $"--outfile=($d)/R.tif" $"--calc=($r_calc)" o> /dev/null
    gdal_calc.py ...$inputs ...$co_calc $"--outfile=($d)/G.tif" $"--calc=($g_calc)" o> /dev/null
    gdal_calc.py ...$inputs ...$co_calc $"--outfile=($d)/B.tif" $"--calc=($b_calc)" o> /dev/null
    gdal_calc.py ...$inputs ...$co_calc $"--outfile=($d)/A.tif" $"--calc=($a_calc)" o> /dev/null

    # 9. Stack RGBA with the alpha as an internal mask, then write the tile.
    let vrt = $"($d)/stack.vrt"
    gdalbuildvrt -separate $vrt $"($d)/R.tif" $"($d)/G.tif" $"($d)/B.tif" $"($d)/A.tif" o> /dev/null
    gdal_edit.py -colorinterp_1 red -colorinterp_2 green -colorinterp_3 blue $vrt o> /dev/null
    sed -i '/<NoDataValue>/d; /<NODATA>/d; /<SrcRect/d; /<DstRect/d; s/ComplexSource/SimpleSource/g' $vrt
    sed -i 's|</VRTDataset>|<MaskBand><VRTRasterBand dataType="Byte"><SimpleSource><SourceFilename relativeToVRT="1">a-warped.tif</SourceFilename><SourceBand>1</SourceBand></SimpleSource></VRTRasterBand></MaskBand></VRTDataset>|' $vrt

    (gdal_translate --config GDAL_TIFF_INTERNAL_MASK YES -of GTiff
      ...$co_big $vrt $"($d)/final.tif" o> /dev/null)

    # A tile is only "done" if it really has all four bands AND the right colour
    # interpretation. gdal_calc.py can fail to write one of R/G/B/A without
    # gdalbuildvrt -separate complaining, and gdalbuildvrt then SKIPS such a tile
    # at merge time with a warning swallowed by a progress bar, punching a silent
    # hole in the national mosaic. Both failure modes bit England for real
    # (NU2020-00, SU5520-11). Throwing here routes the window to failed/.
    let binfo = (gdalinfo -json $"($d)/final.tif" | from json | get bands)
    if ($binfo | length) != 4 {
        error make {msg: $"($w.id): produced ($binfo | length) bands, expected 4 — discarding"}
    }
    let ci = ($binfo | each {|b| $b | get -o colorInterpretation | default "" } | first 3)
    if $ci != [Red Green Blue] {
        error make {msg: $"($w.id): colour interpretation is ($ci), expected [Red Green Blue] — discarding"}
    }

    mv $"($d)/final.tif" $out
    rm -rf $d
    print $"  ($w.id): done"
}

# One unreadable window must not kill the run. A window that throws is recorded
# in failed/ and skipped; because the pending check only looks for .tif and
# .empty, a later run retries it automatically.
def process-window [w: record, tr: string]: nothing -> nothing {
    try {
        render-window $w $tr
    } catch {|e|
        let msg = ($e | get -o msg | default "unknown")
        print $"  !! ($w.id): FAILED — ($msg)"
        touch $"($DATA_DIR)/failed/($w.id)"
        rm -rf $"($TMPDIR)/lu_($w.id)"
    }
}

# ── Pipeline ──────────────────────────────────────────────────────────────────

mkdir $DATA_DIR
mkdir $TILES_DIR
mkdir $"($DATA_DIR)/smooth2m"
mkdir $"($DATA_DIR)/failed"
cd $DATA_DIR

let pi = (1 | math arctan) * 4
let tr = ($pi * 2 * 6378137 / 256 / (2 ** $ZOOM) | into string)
print $"ZOOM=($ZOOM) TR=($tr)"

# ── 0. Fetch the national 1 m DEM (one command, ~11 min, 4.8 GB) ──────────────
# Reads the COG's 1 m overview over HTTP range requests; the 0.5 m data is never
# transferred. Skipped if the file is already there.
if ($SRC | path exists) {
    print $"==> ($SRC) exists — reusing \(delete to re-fetch\)"
} else {
    print $"==> Fetching 1 m DEM from data.public.lu — 4.8 GB, ~11 min"
    let tmp = $"($SRC).tmp"
    rm -f $tmp
    (gdal_translate
      --config CPL_VSIL_CURL_ALLOWED_EXTENSIONS .tif
      --config GDAL_DISABLE_READDIR_ON_OPEN EMPTY_DIR
      --config GDAL_HTTP_MAX_RETRY 10 --config GDAL_HTTP_RETRY_DELAY 3
      --config GDAL_HTTP_TIMEOUT 120 --config GDAL_CACHEMAX 2048
      -oo OVERVIEW_LEVEL=0
      -ot Float32
      -co COMPRESS=ZSTD -co ZSTD_LEVEL=1 -co PREDICTOR=3
      -co TILED=YES -co BLOCKXSIZE=512 -co BLOCKYSIZE=512
      -co BIGTIFF=YES -co NUM_THREADS=ALL_CPUS
      $SRC_URL $tmp)
    mv $tmp $SRC
}

# ── 1. Window grid, derived from the raster's own extent ──────────────────────
# Snapped to a STEP-multiple grid so window ids map to round LUREF coordinates,
# then CLAMPED to the raster so -projwin never overhangs (a partial overhang makes
# gdal_translate silently return a smaller raster, which would break the fixed
# collar assumption in step 6).

print "==> Building window list"
let gi = (gdalinfo -json $SRC | from json)
let gt = $gi.geoTransform
let rx0 = $gt.0
let ry1 = $gt.3
let rx1 = ($rx0 + ($gi.size.0 | into float) * $gt.1)
let ry0 = ($ry1 + ($gi.size.1 | into float) * $gt.5)
print $"  raster extent ($rx0), ($ry0) -> ($rx1), ($ry1)"

let gx0 = (($rx0 / $STEP) | math floor) * $STEP
let gy0 = (($ry0 / $STEP) | math floor) * $STEP
let ncol = (((($rx1 - $gx0) / $STEP) | math ceil))
let nrow = (((($ry1 - $gy0) / $STEP) | math ceil))

let windows = (
    0..($ncol - 1) | each {|i|
        0..($nrow - 1) | each {|j|
            let xmin = ($gx0 + ($i * $STEP | into float))
            let xmax = ($xmin + ($STEP | into float))
            let ymin = ($gy0 + ($j * $STEP | into float))
            let ymax = ($ymin + ($STEP | into float))
            {
              id:    $"c(($i | fill -a r -c '0' -w 2))r(($j | fill -a r -c '0' -w 2))"
              xmin:  $xmin
              xmax:  $xmax
              ymin:  $ymin
              ymax:  $ymax
              # collared bounds, clamped to the raster (what gets cut & smoothed)
              cxmin: ([($xmin - $COLLAR) $rx0] | math max)
              cxmax: ([($xmax + $COLLAR) $rx1] | math min)
              cymin: ([($ymin - $COLLAR) $ry0] | math max)
              cymax: ([($ymax + $COLLAR) $ry1] | math min)
              # bare window bounds clamped to the raster — the 2 m contour tile.
              # Must NOT use the unclamped bounds: on an edge window -projwin
              # would overhang the smoothed window, and gdal_translate answers a
              # partial overhang by silently returning a smaller raster (plus a
              # warning per window). Clamping makes the request exact.
              dxmin: ([$xmin $rx0] | math max)
              dxmax: ([$xmax $rx1] | math min)
              dymin: ([$ymin $ry0] | math max)
              dymax: ([$ymax $ry1] | math min)
            }
        }
    }
      | flatten
      | where {|w| $w.cxmin < $w.cxmax and $w.cymin < $w.cymax }
)
print $"  ($ncol) x ($nrow) grid -> ($windows | length) windows intersecting the raster"

# ── 2. Process every window with no output and no empty-marker yet ────────────

let pending = (
    $windows | where {|w|
        not ($"($TILES_DIR)/($w.id).tif" | path exists) and not ($"($TILES_DIR)/($w.id).empty" | path exists)
    }
)
print $"==> ($windows | length) windows, ($pending | length) pending"

if ($pending | length) > 0 {
    $pending | par-each -t $PARALLEL {|w| process-window $w $tr}
}

# ── 3. Refuse to merge a partial country ──────────────────────────────────────
# Once shading.tif exists every later run skips straight past the merge, so a
# mosaic built while windows were still missing would quietly become the final
# product.

let unfinished = (
    $windows | where {|w|
        not ($"($TILES_DIR)/($w.id).tif" | path exists) and not ($"($TILES_DIR)/($w.id).empty" | path exists)
    }
)
let failed = (glob $"($DATA_DIR)/failed/*" | each {|f| $f | path basename })

print "==> Checking every tile has 4 bands (gdalbuildvrt skips those that do not)"
let malformed = (
    glob $"($TILES_DIR)/*.tif"
      | par-each -t $PARALLEL {|f|
          let n = (do { gdalinfo -json $f } | complete)
          if $n.exit_code != 0 {
              {file: $f, bands: -1}
          } else {
              {file: $f, bands: ($n.stdout | from json | get bands | length)}
          }
        }
      | where bands != 4
)
if ($malformed | is-not-empty) {
    print $"==> ($malformed | length) malformed tile\(s\) — deleting so the next pass rebuilds them:"
    $malformed | first 20 | each {|m| print $"      ($m.file | path basename): ($m.bands) bands" }
    $malformed | each {|m| rm -f $m.file }
    print "    Re-run; refusing to merge a mosaic with holes."
    exit 1
}

if ($unfinished | is-not-empty) {
    print ""
    print $"==> NOT MERGING: ($unfinished | length) of ($windows | length) windows still have no output."
    if ($failed | is-not-empty) {
        print $"    ($failed | length) window\(s\) recorded in ($DATA_DIR)/failed/ — delete the"
        print $"    marker\(s\) and re-run to retry:"
        $failed | first 10 | each {|f| print $"      ($f)" }
    }
    print "    Re-run this script; finished windows are skipped, so it resumes cheaply."
    exit 1
}

# ── 4. Merge all tiles and build overviews ────────────────────────────────────
# JXL (lossy, distance=3.0) requires GDAL linked against a libtiff with libjxl
# support (conda geo env). PREDICTOR is intentionally omitted (JXL doesn't use it).

if ($OUT_TIF | path exists) {
    # Existing output is never overwritten. To rebuild, RENAME it rather than
    # deleting — an old mosaic is worth keeping until the new one is checked:
    #   mv shading.tif shading-YYYY-MM-DD.tif
    print $"==> ($OUT_TIF) exists — rename it \(don't delete\) to re-merge; skipping"
} else {
    print "==> Merging tiles"
    let idx = "shading_index"
    glob $"($TILES_DIR)/*.tif" | save -f $idx
    print $"  (open $idx | lines | length) tiles"
    gdalbuildvrt -input_file_list $idx shading.vrt

    # THE GUARD THAT ACTUALLY MATTERS. gdalbuildvrt silently SKIPS any input whose
    # band count, data type or colour interpretation disagrees with the first
    # input, reporting it only in a warning swallowed by its own progress bar.
    # Checking that every offered tile actually landed in the VRT catches all such
    # defects, including ones not yet encountered.
    let offered = (open $idx | lines | each {|f| $f | path basename})
    let used = (
        open --raw shading.vrt
          | parse -r '<SourceFilename[^>]*>([^<]+)</SourceFilename>'
          | get capture0
          | each {|f| $f | path basename}
          | uniq
    )
    let dropped = ($offered | where {|f| $f not-in $used})
    if ($dropped | is-not-empty) {
        print ""
        print $"==> NOT MERGING: gdalbuildvrt dropped ($dropped | length) of ($offered | length) tiles."
        print "    Each would be an invisible hole in the national mosaic."
        $dropped | first 20 | each {|f| print $"      ($f)" }
        print "    Delete those tiles and re-run; the windows will be regenerated."
        rm -f shading.vrt
        rm $idx
        exit 1
    }
    print $"  verified: all ($offered | length) tiles present in the VRT"

    rm $idx
    sed -i 's|<ColorInterp>Alpha</ColorInterp>|<ColorInterp>Undefined</ColorInterp>|g' shading.vrt

    (gdal_translate --config GDAL_TIFF_INTERNAL_MASK YES --config GDAL_TIFF_INTERNAL_MASK_TO_8BIT YES
      -of GTiff -co COMPRESS=JXL -co JXL_LOSSLESS=NO -co JXL_DISTANCE=3.0
      -co TILED=YES -co BLOCKXSIZE=256 -co BLOCKYSIZE=256 -co BIGTIFF=YES -co NUM_THREADS=ALL_CPUS
      shading.vrt $"($OUT_TIF).tmp")
    mv $"($OUT_TIF).tmp" $OUT_TIF
    rm shading.vrt
    gdal_edit.py -colorinterp_4 alpha $OUT_TIF

    print "==> Building overviews"
    (gdaladdo --config GDAL_TIFF_INTERNAL_MASK YES --config GDAL_CACHEMAX 4096
      --config GDAL_NUM_THREADS ALL_CPUS --config COMPRESS_OVERVIEW JXL
      --config JXL_LOSSLESS_OVERVIEW NO --config JXL_DISTANCE_OVERVIEW 3.0
      -r average $OUT_TIF)
}
print "==> Done"
