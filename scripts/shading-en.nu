#!/usr/bin/env nu

# Generate shaded relief for England from the Environment Agency's national 1 m
# LIDAR Composite DTM 2022. England port of shading-no.nu (Norway 1 m).
#
# Source: /media/martin/18TB/en/DTM1 — 5876 GeoTIFF tiles from download-en.nu.
#   Verified uniform on every tile the downloader accepted (it checks each one):
#   5000 x 5000 px, Float32, 1 m, EPSG:27700 embedded, LZW, 128x128 blocks,
#   no overviews, nodata -3.4028235e+38 declared.
#
#   TILES ARE EDGE-TO-EDGE. This is the one structural difference from Norway,
#   whose DTM1 tiles carry a 5 px collar each and overlap by 10 px. Here the
#   5 km grid step matches the 5000 px tile exactly, so a tile has NO context of
#   its own — every pixel of smoothing context has to come from the national VRT.
#   That is already how this script works (windows are cut from the VRT, not from
#   source tiles), so nothing special is needed; but do not "optimise" it into
#   per-tile processing, which would put a visible seam every 5 km.
#
#   NODATA IS THE TRAP. -3.4028235e+38 is Float32's lowest value, and genuine
#   English elevations go BELOW ZERO — a sampled Devon tile bottoms out at
#   -6.06 m of real terrain. So nothing may treat "very negative" as void; only
#   the declared sentinel counts. -vrtnodata -9999 presents one clean sentinel
#   downstream while each source keeps its real -3.4e38 as a per-source <NODATA>
#   (the same mechanism the HR script used to unify four sentinels and the NO
#   script to unify one).
#
# ZOOM=16, measured rather than inherited. sample-zoom-en.nu rendered NY2005 —
#   Scafell Pike, 134-978 m, the roughest ground in England, i.e. the best case
#   for a finer zoom — at z15/z16/z17 through this exact pipeline, then upscaled
#   each to z17's grid (what a client does when it overzooms) and differenced
#   over 13.5 M pixels:
#
#     z15 -> z17   mean 6.12/255   p95 23.0   31.2% of pixels off by >5
#     z16 -> z17   mean 1.32/255   p95  4.0    3.2% of pixels off by >5
#
#   z16 is indistinguishable; z15 is visibly lossy. Norway measured 1.07 and 2.4%
#   on its own best case and chose z16 too. The ceiling is the source, not the
#   grid: the DTM is a 1 m grid and --filter 11 strips most sub-11 m variation
#   before the hillshade is computed, so z17 resamples detail already removed —
#   for 137 GB against 44 GB and roughly 4x the render time.
#
#   MIND THE UNITS. A z16 pixel here is 1.34-1.54 GROUND metres, not the 2.39 m
#   that -tr says: Mercator pixels shrink as cos(lat), so at 50-56 degN the raw
#   -tr overstates resolution by about a third. z16 therefore sits slightly
#   coarser than the 1 m source, which the measurement says does not matter.
#   Re-run sample-zoom-en.nu on a different TILE to test that claim elsewhere.
#
# DATUM: THE DELIVERED PRODUCTS ARE ~1.9 m OFF TRUE WGS84. KNOWN, ACCEPTED.
#
# EPSG:27700 is on the OSGB36 datum, and getting from there to WGS84/Web
# Mercator needs a datum shift. The correct one is OSTN15, OS's official NTv2
# grid (~0.1 m). It was NOT installed when these products were built, and PROJ
# does not error in that case — it silently falls back to a 7-parameter Helmert
# ("OSGB36 to WGS 84 (6)", 2 m stated accuracy).
#
# Measured 2026-08-17 against the real grid, 238 samples across England:
#
#     median 1.90 m   p95 3.57 m   max 4.67 m
#     mid-England 1.89 m | Lake District 0.94 m | London 1.74 m
#
# A z16 pixel here is 1.34-1.54 ground metres, so this is 1-3 px of systematic
# misregistration against OSM, and it VARIES SPATIALLY — no constant offset can
# correct it. Both shading.tif and contours_en carry it, so at least they agree
# with each other. england_contours.gpkg and the raw DTM tiles are native
# EPSG:27700 and therefore unaffected; only reprojected outputs are.
#
# MIXED-DATUM HAZARD — READ BEFORE ANY PARTIAL RE-RENDER. The OSTN15 grid is now
# present at ~/.local/share/proj/uk_os_OSTN15_NTv2_OSGBtoETRS.tif, which the geo
# env's PROJ picks up automatically. So re-rendering a handful of windows into
# the EXISTING tiles/ would place them ~1.9 m from their neighbours and leave a
# visible seam. If you re-render, re-render EVERYTHING: delete tiles/ and rebuild
# the mosaic from scratch. Do not resume a partial run across this change.
#
# OSTN15 IS NOW INSTALLED BOX-WIDE (/usr/share/proj/, 2026-08-17), verified to
# match pyproj digit-for-digit from PostGIS. So the whole machine is on OSTN15
# while these two products are not — the mixed-datum hazard above now applies to
# BOTH: re-running the splitter alone would put contours ~1.9 m from the shading,
# i.e. they would stop agreeing with each other. Currently they are consistently
# wrong together, which is the better of the two bad states.
#
# TO FIX PROPERLY (a full rebuild, ~17 h shading + ~2 h splitter): no setup left
#   — delete tiles/ and shading.tif, re-run shading-en.nu, then re-run the
#   splitter from the (unaffected) EPSG:27700 GPKG. Both pick up OSTN15 on their
#   own.
#
# WHEN COMPUTING A 4326 BBOX AFTER THAT: OSTN15 covers only the GB landmass, not
# the whole BNG rectangle, so densifying along the rectangle edges returns inf
# outside its coverage. Clamp to the grid extent, or densify over the data
# footprint rather than the raster rectangle.
#
# WHY NO gdal_retile, as in Norway and unlike the other countries:
#
#   1. DISK. retiled/ + smooth/ for all of England at 1 m is ~1 TB against ~400 GB
#      free on the NVMe. So windows are cut from the national VRT on demand,
#      processed, and their intermediates deleted immediately — only tiles/
#      accumulates. Peak scratch is PARALLEL x one window, not the whole country.
#
#   2. EMPTINESS. England's BNG bbox is ~383,000 km2 for ~130,000 km2 of land.
#      Deriving windows from the real tile footprints (each 5 km tile = 2x2
#      windows of 2.5 km) gives windows that contain data by construction.
#
# WINDOW ORIGINS ARE READ FROM THE TILES, NOT DECODED FROM THEIR NAMES. An OS
#   grid reference like NY2005 is decodable in principle, but getting the
#   500 km/100 km letter pair wrong would silently shift a whole region. One
#   gdalinfo pass over the tiles, cached to a TSV, is authoritative and costs a
#   minute. Delete the TSV to rebuild.
#
# CONTOURS. smooth/ cannot be kept (~520 GB), so each window also emits a 2 m
#   downsample of its smoothed DEM into smooth2m/, which contours-en.nu contours
#   directly. `average` resampling is nodata-aware; bilinear/cubic would blend
#   nodata into its neighbours.
#
# PREDICTOR=1 on the window DEM is LOAD-BEARING, not a style choice.
#   feature-preserving-smoothing does I/O via the `wbgeotiff` crate, which does
#   not parse the TIFF Predictor tag (317) — fed PREDICTOR=2/3 float data it
#   decodes byte-shuffled deltas as garbage and emits +/-Inf rasters that render
#   as static, WITHOUT erroring. (Documented in shading-it.nu, verified 2026-07-17.)
#
# Smoothing is 11/16/6/6, the Poland/Croatia/Norway 1 m settings — the filter is
#   a pixel count, so 11 px = 11 m on 1 m data.
#
# Resumable at window granularity: a window whose tiles/<id>.tif exists is
# skipped, and an all-nodata window leaves a tiles/<id>.empty marker so it is not
# re-cut on the next run. A window that throws is recorded in failed/ and skipped
# so one bad source tile cannot take down a multi-day run. Run via:
#   nice ~/miniforge3/bin/conda run --no-capture-output -n geo nu ~/fm/freemap-outdoor-map/shading-en.nu

# ── Configuration ─────────────────────────────────────────────────────────────

const SRC_DIR   = "/media/martin/18TB/en/DTM1"
const DATA_DIR  = "/mnt/osm/en"                  # VRT, origin cache, smooth2m/ on NVMe
const TILES_DIR = "/mnt/osm/en/tiles"            # NVMe — see WHERE THE I/O GOES
const OUT_TIF   = "/media/martin/18TB/en/shading.tif"
const EPSG      = "EPSG:27700"                   # OSGB36 / British National Grid
const NODATA    = "-9999"                        # unified nodata presented by en.vrt
const VRT       = "en.vrt"
const ORIGINS   = "tile_origins.tsv"             # tile<TAB>xmin<TAB>ymax, cached
const ZOOM      = 16                             # measured, not assumed — see header
const PARALLEL  = 24
const TMPDIR    = "/dev/shm"

const TILE_M    = 5000                           # source tile size, m (= px)
const STEP      = 2500                           # window size, m (= px); 2x2 per source tile
const COLLAR    = 6                              # m cut beyond the window on each side
const CROP      = 3                              # px cropped per edge after hillshading

# WHERE THE I/O GOES, and why.
#
#   scratch      /dev/shm  — every per-window intermediate (the window cut, the
#                 smoothed DEM, three hillshades, three warps, RGBA) is written
#                 to tmpfs and deleted on the way out. 24 windows in flight peak
#                 at a few GB of the 32 GB available, so the hot path is RAM and
#                 never touches a disk at all.
#   source reads /media/martin/18TB/en/DTM1 — 330 GB, has to stay on the HDD;
#                 read once, largely sequentially, so the HDD costs little here.
#   tiles/       /mnt/osm/en — NVMe, ON PURPOSE. This is 23504 small files that
#                 are written once and then read back in full by the merge. The
#                 18TB is ntfs-3g (FUSE, since this kernel has no ntfs3), where
#                 every small-file create is a userspace round-trip — by far the
#                 worst I/O in the run, on the worst filesystem for it.
#                 Measured cost: the z16 zoom sample is 1.09 MB/km2, so ~141 GB
#                 for England, and that is an upper bound because the sample is
#                 Scafell Pike and rough terrain compresses worst. With
#                 smooth2m/ at ~130 GB that leaves ~127 GB free on the NVMe.
#                 (An earlier comment here said tiles/ was "too big for the
#                 NVMe" — that was a guess made before the sample existed.)
#                 DELETE tiles/ once shading.tif is built; it is pure
#                 intermediate and frees ~141 GB.
#   shading.tif  stays on the 18TB: one big sequential write, and the NVMe
#                 headroom is worth more to tiles/.

# Smoothing (Poland/Croatia/Norway 1 m settings — filter is a pixel count = metres here)
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

# ── Helpers ───────────────────────────────────────────────────────────────────

# Does the window contain any real data? A FALSE here writes a permanent .empty
# marker, so it must mean "gdalinfo ran and found no valid pixel" and never
# "gdalinfo did not run". Treating a transient failure as no-data silently drops
# a window from the country forever: SU5555-00 was lost that way on 2026-08-18
# and only surfaced because it still had a smooth2m tile from an earlier run.
# Throwing instead routes the window to failed/, where it is retried.
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

# Cut one window (plus collar) out of the national VRT, smooth it, emit the 2 m
# DEM for contours, and render the RGBA shaded relief tile. All intermediates
# live in TMPDIR and are removed on the way out, so disk never accumulates.
def render-window [w: record, tr: string]: nothing -> nothing {
    let out = $"($TILES_DIR)/($w.id).tif"
    let d   = $"($TMPDIR)/en_($w.id)"

    rm -rf $d
    mkdir $d

    let co      = [-co COMPRESS=ZSTD -co PREDICTOR=2 -co TILED=YES -co NUM_THREADS=ALL_CPUS]
    let co_big  = [...$co -co BIGTIFF=YES]
    let co_calc = [--co=COMPRESS=ZSTD --co=PREDICTOR=2 --co=TILED=YES --co=NUM_THREADS=ALL_CPUS --co=BIGTIFF=YES]

    # 1. Cut window + collar from the national VRT. PREDICTOR=1 — see header.
    let win = $"($d)/win.tif"
    (gdal_translate -q -of GTiff
      -projwin ($w.xmin - $COLLAR) ($w.ymax + $COLLAR) ($w.xmax + $COLLAR) ($w.ymin - $COLLAR)
      -co COMPRESS=DEFLATE -co PREDICTOR=1 -co TILED=YES
      $VRT $win o> /dev/null)

    # An all-nodata window (sea, or a gap in coverage) leaves a marker so the
    # next run skips it without re-cutting.
    if not (has-data $win) {
        rm -rf $d
        touch $"($TILES_DIR)/($w.id).empty"
        print $"  ($w.id): empty"
        return
    }

    # 2. Smooth. A zero-variance window (flat estuary sliver) makes the smoother
    #    panic and has no relief to smooth anyway — pass it through.
    let band = gdalinfo -json -mm $win err> /dev/null | from json | get bands | first
    let smooth = $"($d)/smooth.tif"
    if ($band.computedMin? == $band.computedMax?) {
        cp $win $smooth
    } else {
        (feature-preserving-smoothing --dem $win -o $smooth
          --filter $SM_FILTER --norm_diff $SM_NORM_DIFF
          --num_iter $SM_NUM_ITER --max_diff $SM_MAX_DIFF)
    }

    # 3. 2 m DEM for contours-en.nu — collar cropped, nodata-aware `average`.
    let dem2 = $"($DATA_DIR)/smooth2m/($w.id).tif"
    if not ($dem2 | path exists) {
        let tmp2 = $"($d)/dem2m.tif"
        (gdal_translate -q -of GTiff
          -projwin $w.xmin $w.ymax $w.xmax $w.ymin
          -tr 2 2 -r average -a_nodata $NODATA
          ...$co $smooth $tmp2 o> /dev/null)
        mv $tmp2 $dem2
    }

    # 4. Close small interior voids so they don't become transparent specks.
    #    Large out-of-coverage regions exceed -md 5 and stay nodata -> transparent.
    let dem = $"($d)/dem.tif"
    gdal_fillnodata.py -md 5 $smooth $dem o> /dev/null err> /dev/null

    # 5. Three Igor hillshades at different azimuths, on the collared window so
    #    edges have real neighbours.
    gdaldem hillshade $dem $"($d)/_a.tif" -az -120 -igor -compute_edges ...$co o> /dev/null
    gdaldem hillshade $dem $"($d)/_b.tif" -az  60  -igor -compute_edges ...$co o> /dev/null
    gdaldem hillshade $dem $"($d)/_c.tif" -az -45  -igor -compute_edges ...$co o> /dev/null

    # 6. Crop the collar (minus the warp margin) off each hillshade.
    let info = gdalinfo -json $dem | from json
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

    # A tile is only "done" if it really has all four bands. gdal_calc.py can fail
    # to write one of R/G/B/A without gdalbuildvrt -separate complaining, which
    # yields a 3-band tile that looks finished. gdalbuildvrt then SKIPS such a
    # tile at merge time with a warning swallowed by a progress bar, punching a
    # silent hole in the national mosaic. (Happened once for real: NU2020-00 lost
    # its alpha and left a 2.5 km gap on the Northumberland coast, found only by
    # grepping the merge log.) Throwing here routes the window through
    # process-window's catch, so it lands in failed/ and is retried next pass.
    # Four bands AND the right colour interpretation. gdalbuildvrt groups inputs
    # by both, and silently SKIPS any that disagree — a 4-band tile whose bands
    # are all Undefined (gdal_edit.py having failed) is dropped from the mosaic
    # just as surely as a 3-band one. SU5520-11 was lost that way on 2026-08-18:
    # "does not support heterogeneous band color interpretation: expected Red,
    # got Undefined".
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

# One unreadable source tile must not kill a multi-day run. A window that throws
# is recorded in failed/ and skipped; because the pending check only looks for
# .tif and .empty, a later run retries it automatically once the source is
# repaired. (Norway lesson: one tile with silent LZW corruption took down a
# 30-hour render at 22751/50825 — twice.)
def process-window [w: record, tr: string]: nothing -> nothing {
    try {
        render-window $w $tr
    } catch {|e|
        let msg = ($e | get -o msg | default "unknown")
        print $"  !! ($w.id): FAILED — ($msg)"
        touch $"($DATA_DIR)/failed/($w.id)"
        rm -rf $"($TMPDIR)/en_($w.id)"
    }
}

# ── Pipeline ──────────────────────────────────────────────────────────────────

mkdir $DATA_DIR
mkdir $TILES_DIR
mkdir $"($DATA_DIR)/smooth2m"
mkdir $"($DATA_DIR)/failed"
cd $DATA_DIR

if (not ($SRC_DIR | path exists)) or ((glob $"($SRC_DIR)/*.tif" | length) == 0) {
    error make {msg: $"($SRC_DIR) is empty — run download-en.nu first"}
}

let pi = (1 | math arctan) * 4
let tr = ($pi * 2 * 6378137 / 256 / (2 ** $ZOOM) | into string)
print $"ZOOM=($ZOOM) TR=($tr)"

# 0. National VRT. -vrtnodata unifies the delivered Float32-min sentinel to
#    -9999; an input_file_list avoids argv overflow on 5876 tiles.
if ($VRT | path exists) {
    print $"==> ($VRT) exists — reusing \(delete to force a rebuild\)"
} else {
    print $"==> Building national VRT ($VRT) — unified nodata ($NODATA)"
    let idx = "_idx_en"
    glob $"($SRC_DIR)/*.tif" | save -f $idx
    print $"  (open $idx | lines | length) tiles"
    gdalbuildvrt -vrtnodata $NODATA -input_file_list $idx $"($VRT).tmp" o> /dev/null
    rm $idx
    mv $"($VRT).tmp" $VRT
}

# 1. Tile origins, read from the tiles themselves — see header. Cached.
if not ($ORIGINS | path exists) {
    print "==> Reading tile origins (one gdalinfo pass, cached)"
    let rows = (
        glob $"($SRC_DIR)/*.tif"
          | par-each -t $PARALLEL {|f|
              let g = (gdalinfo -json $f | from json | get geoTransform)
              {tile: ($f | path parse | get stem), xmin: $g.0, ymax: $g.3}
            }
          | sort-by tile
    )
    $rows | to tsv | save -f $"($ORIGINS).tmp"
    mv $"($ORIGINS).tmp" $ORIGINS
    print $"  ($rows | length) origins"
}

# 2. Window grid: 2x2 windows of 2.5 km per 5 km source tile.
print "==> Building window list"
let windows = (
    open --raw $ORIGINS | from tsv
      | each {|t|
          let x0 = ($t.xmin | into float)
          let y1 = ($t.ymax | into float)
          0..1 | each {|i|
              0..1 | each {|j|
                  {
                    id: $"($t.tile)-($i)($j)"
                    xmin: ($x0 + ($i * $STEP | into float))
                    xmax: ($x0 + (($i + 1) * $STEP | into float))
                    ymax: ($y1 - ($j * $STEP | into float))
                    ymin: ($y1 - (($j + 1) * $STEP | into float))
                  }
              }
          } | flatten
        }
      | flatten
)
print $"  ($windows | length) windows"

# 3. Process every window that has no output and no empty-marker yet.
let pending = (
    $windows | where {|w|
        not ($"($TILES_DIR)/($w.id).tif" | path exists) and not ($"($TILES_DIR)/($w.id).empty" | path exists)
    }
)
print $"==> ($windows | length) windows, ($pending | length) pending"

if ($pending | length) > 0 {
    $pending | par-each -t $PARALLEL {|w| process-window $w $tr}
}

# 4. Refuse to merge a partial country.
# The merge is the one irreversible-looking step: once shading.tif exists, every
# later run skips straight past it, so a mosaic built while windows were still
# missing would quietly become the final product. A window can be missing for
# ordinary reasons (the run was killed, one source tile is unreadable and landed
# in failed/), and those runs are meant to be retried, not shipped.
let unfinished = (
    $windows | where {|w|
        not ($"($TILES_DIR)/($w.id).tif" | path exists) and not ($"($TILES_DIR)/($w.id).empty" | path exists)
    }
)
let failed = (glob $"($DATA_DIR)/failed/*" | each {|f| $f | path basename })

# Backstop for the same failure mode, covering tiles written by older runs that
# predate the per-tile check above. gdalbuildvrt takes its band count from the
# first input and silently skips every input that disagrees, so one malformed
# tile becomes an invisible hole rather than an error. Delete offenders here and
# refuse to merge; the next pass regenerates them.
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
        print $"    ($failed | length) window\(s\) recorded in ($DATA_DIR)/failed/ — likely an unreadable"
        print $"    source tile. Verify it with `gdalinfo -checksum`, re-fetch it with"
        print $"    download-en.nu, then delete the marker\(s\) and re-run:"
        $failed | first 10 | each {|f| print $"      ($f)" }
    }
    print "    Re-run this script; finished windows are skipped, so it resumes cheaply."
    exit 1
}

# 5. Merge all tiles and build overviews.
# JXL (lossy, distance=3.0) requires GDAL linked against a libtiff with libjxl
# support (conda geo env). PREDICTOR is intentionally omitted (JXL doesn't use it).
if ($OUT_TIF | path exists) {
    # Existing output is never overwritten. To rebuild, RENAME it rather than
    # deleting — an old mosaic is worth keeping until the new one is checked,
    # and a re-merge is ~1.5 h plus overviews:
    #   mv shading.tif shading-YYYY-MM-DD.tif
    print $"==> ($OUT_TIF) exists — rename it \(don't delete\) to re-merge; skipping"
} else {
    print "==> Merging tiles"
    let idx = "shading_index"
    glob $"($TILES_DIR)/*.tif" | save -f $idx
    print $"  (open $idx | lines | length) tiles"
    gdalbuildvrt -input_file_list $idx shading.vrt

    # THE GUARD THAT ACTUALLY MATTERS. gdalbuildvrt silently SKIPS any input
    # whose band count, data type or colour interpretation disagrees with the
    # first input, reporting it only in a warning swallowed by its own progress
    # bar. Two different holes reached a "finished" 25 GB mosaic that way:
    # NU2020-00 (3 bands) on 2026-08-17 and SU5520-11 (all-Undefined colour
    # interpretation) on 2026-08-18. Checking the specific defects I happened to
    # meet is whack-a-mole; checking that every offered tile actually landed in
    # the VRT catches all of them, including reasons not yet encountered.
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
