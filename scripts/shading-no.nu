#!/usr/bin/env nu

# Generate shaded relief for Norway from the national 1 m LiDAR DTM (Kartverket DTM1).
# Norway port of shading-hr.nu (Croatia 1 m) / shading.nu (Poland 1 m).
#
# Source: /media/martin/18TB/no/DTM1 — 2033 GeoTIFF tiles from the Geonorge Atom
#   download service (see download-no.nu). Verified uniform across all 2033:
#   15010 x 15010 px, Float32, 1 m, nodata -32767 declared on EVERY tile,
#   EPSG:25833 embedded on EVERY tile, LZW, 512x512 blocks, 6 overviews.
#
#   NONE of the Croatian delivery's quirks apply. Stage 0 is a plain gdalbuildvrt:
#   no -a_srs, no -allow_projection_difference, no SimpleSource->ComplexSource
#   rewriting. -vrtnodata -9999 presents one clean sentinel downstream while each
#   source keeps its real -32767 as a per-source <NODATA> (same mechanism the HR
#   script used to unify four sentinels; here it unifies exactly one).
#
#   The source tiles OVERLAP by 10 px: the grid step is 15000 m but each tile is
#   15010 px, i.e. every tile carries a 5 px collar beyond its 15 km cell. The
#   overlapping pixels are identical, so the VRT needs no special handling — but
#   the window grid below is built on the 15 km CELLS, not the tile extents, or
#   windows would drift by 5 m per tile and double-cover the seams.
#
# ZOOM=16, not 17. Measured on a 1.5 km window of Ostmarka (tile 33-124-114,
#   covered by Oslo 10pkt 2024 — among the densest LiDAR in the country, i.e. the
#   best case for a finer zoom): rendering at z17 and at z16-upscaled-to-z17
#   differ by mean 1.07/255, p95 4.0, with 2.4% of pixels off by more than 5
#   levels. Indistinguishable. z15 differs by mean 3.14 with 16.6% of pixels off
#   by >5 — visible loss. The ceiling is the source: DTM1 is a 1 m grid
#   everywhere (point density 2-10 pkt/m2 changes its QUALITY, not its
#   resolution), and feature-preserving-smoothing --filter 11 strips most
#   sub-11 m variation before the hillshade is computed. z16 over Norway is
#   0.78-1.27 ground m/px (Mercator pixels shrink as cos(lat)) — the same ground
#   detail the z17 countries (SK/AT/CZ/SI, all 46-50 degN) get at 0.8 m/px, and
#   the same choice Sweden made with the same latitudes and the same 1 m LiDAR.
#
# WHY THIS SCRIPT DOES NOT USE gdal_retile, unlike every other country:
#
#   1. DISK. retiled/ + smooth/ for all of Norway is ~1.8 TB against ~670 GB free
#      on the NVMe. So windows are cut from the national VRT on demand, processed,
#      and their intermediates deleted immediately — only tiles/ accumulates.
#      Peak scratch is PARALLEL x one window, not the whole country.
#
#   2. EMPTINESS. Norway's UTM33 bbox is ~1.8 M km2 but only ~324,000 km2 is land.
#      gdal_retile over the national VRT would write ~450,000 tiles, the large
#      majority all-nodata. Deriving windows from the 2033 real tile footprints
#      (each 15 km cell = 5x5 windows of 3 km) gives 50,825 windows that all
#      contain data by construction.
#
#   Cutting with a collar from the NATIONAL VRT (not per source tile) means each
#   window's smoothing and hillshading see real neighbouring data across source
#   tile seams — the property gdal_retile's -overlap gave the other scripts.
#
# CONTOURS. smooth/ cannot be kept (~900 GB), so each window also emits a 2 m
#   downsample of its smoothed DEM into smooth2m/. That is precisely what
#   contours-hr.nu's consolidation pass produces from smooth/ — so contours-no.nu
#   can contour those directly and skip the consolidation entirely (8.5 h on
#   Croatia, which would have been ~45 h here). ~225 GB, fits on the NVMe.
#   `average` resampling is nodata-aware; bilinear/cubic would blend nodata into
#   its neighbours.
#
# PREDICTOR=1 on the window DEM is LOAD-BEARING, not a style choice.
#   feature-preserving-smoothing does I/O via the `wbgeotiff` crate, which does
#   not parse the TIFF Predictor tag (317) — fed PREDICTOR=2/3 float data it
#   decodes byte-shuffled deltas as garbage and emits +/-Inf rasters that render
#   as static, WITHOUT erroring. (Documented in shading-it.nu, verified 2026-07-17.)
#
# Smoothing is 11/16/6/6, the Poland/Croatia 1 m settings — the filter is a pixel
#   count, so 11 px = 11 m on 1 m data.
#
# Resumable at window granularity: a window whose tiles/<id>.tif exists is skipped,
# and an all-nodata window leaves a tiles/<id>.empty marker so it is not re-cut on
# the next run. Delete tiles/ (or individual outputs) to force a rebuild. Run via:
#   nice ~/miniforge3/bin/conda run --no-capture-output -n geo nu ~/fm/freemap-outdoor-map/shading-no.nu

# ── Configuration ─────────────────────────────────────────────────────────────

const SRC_DIR   = "/media/martin/18TB/no/DTM1"
const DATA_DIR  = "/mnt/osm/no"                  # VRT, window list, smooth2m/ on NVMe
const TILES_DIR = "/media/martin/18TB/no/tiles"  # ~580 GB at z16 — too big for the NVMe
const OUT_TIF   = "/media/martin/18TB/no/shading.tif"
const EPSG      = "EPSG:25833"                   # ETRS89 / UTM 33N, all tiles
const NODATA    = "-9999"                        # unified nodata presented by no.vrt
const VRT       = "no.vrt"
const ZOOM      = 16
const PARALLEL  = 24
const TMPDIR    = "/dev/shm"

const STEP      = 3000                           # window size, m (= px); 15000 / 5, so 25 per source tile
const COLLAR    = 6                              # m cut beyond the window on each side (smoothing context)
const CROP      = 3                              # px cropped per edge after hillshading; COLLAR-CROP leaves warp margin

# The 15 km CELL grid, derived from the tile naming (verified against 33-107-122,
# 33-124-114 and 33-132-158): a tile named 33-E-N spans px [x0-5, x0+15005] where
# x0 = CELL_X0 + (E - E_REF) * 15000, and likewise in y. Cells are contiguous.
const CELL_X0   = 5430.0                         # west edge of cell E=107
const CELL_YTOP = 6771000.0                      # north edge of cell N=122
const E_REF     = 107
const N_REF     = 122

# Smoothing (Poland/Croatia 1 m settings — filter is a pixel count = metres here)
const SM_FILTER    = 11
const SM_NORM_DIFF = 16
const SM_NUM_ITER  = 6
const SM_MAX_DIFF  = 6

# ── Helpers ───────────────────────────────────────────────────────────────────

def has-data [file: string]: nothing -> bool {
    let band = gdalinfo -json -mm $file err> /dev/null | from json | get bands | first
    ($band | get -o computedMin | is-not-empty)
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
    let d   = $"($TMPDIR)/no_($w.id)"

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

    # An all-nodata window (sea, or bbox corner) leaves a marker so the next run
    # skips it without re-cutting.
    if not (has-data $win) {
        rm -rf $d
        touch $"($TILES_DIR)/($w.id).empty"
        print $"  ($w.id): empty"
        return
    }

    # 2. Smooth. A zero-variance window (flat sea-level sliver) makes the smoother
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

    # 3. 2 m DEM for contours-no.nu — collar cropped, nodata-aware `average`.
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

    mv $"($d)/final.tif" $out
    rm -rf $d
    print $"  ($w.id): done"
}

# One unreadable source tile must not kill a 30-hour run. A window that throws is
# recorded in failed/ and skipped; because the pending check only looks for .tif
# and .empty, a later run retries it automatically once the source is repaired.
# (Learned the hard way: tile 33-125-145 had silent LZW corruption — correct byte
# count, bad content — and took the run down at 22751/50825.)
def process-window [w: record, tr: string]: nothing -> nothing {
    try {
        render-window $w $tr
    } catch {|e|
        let msg = ($e | get -o msg | default "unknown")
        print $"  !! ($w.id): FAILED — ($msg)"
        touch $"($DATA_DIR)/failed/($w.id)"
        rm -rf $"($TMPDIR)/no_($w.id)"
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

# 0. National VRT — plain gdalbuildvrt; every source quirk the HR script fought is
#    absent here. An input_file_list avoids argv overflow on 2033 tiles.
if ($VRT | path exists) {
    print $"==> ($VRT) exists — reusing \(delete to force a rebuild\)"
} else {
    print $"==> Building national VRT ($VRT) — unified nodata ($NODATA)"
    let idx = "_idx_no"
    glob $"($SRC_DIR)/*.tif" | save -f $idx
    print $"  (open $idx | lines | length) tiles"
    gdalbuildvrt -vrtnodata $NODATA -input_file_list $idx $"($VRT).tmp" o> /dev/null
    rm $idx
    mv $"($VRT).tmp" $VRT
}

# 1. Window grid: 25 windows per 15 km source cell, aligned to the CELL grid.
print "==> Building window list"
let windows = (
    glob $"($SRC_DIR)/*.tif"
      | each {|f|
          let parts = ($f | path basename | path parse | get stem | split row "-")
          {e: ($parts.1 | into int), n: ($parts.2 | into int)}
        }
      | each {|t|
          let x0 = $CELL_X0 + (($t.e - $E_REF) * 15000 | into float)
          let y1 = $CELL_YTOP + (($t.n - $N_REF) * 15000 | into float)
          0..4 | each {|i|
              0..4 | each {|j|
                  {
                    id: $"($t.e)-($t.n)-($i)($j)"
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

# 2. Process every window that has no output and no empty-marker yet.
let pending = (
    $windows | where {|w|
        not ($"($TILES_DIR)/($w.id).tif" | path exists) and not ($"($TILES_DIR)/($w.id).empty" | path exists)
    }
)
print $"==> ($windows | length) windows, ($pending | length) pending"

if ($pending | length) > 0 {
    $pending | par-each -t $PARALLEL {|w| process-window $w $tr}
}

# 3. Merge all tiles and build overviews.
# JXL (lossy, distance=3.0) requires GDAL linked against a libtiff with libjxl
# support (conda geo env). PREDICTOR is intentionally omitted (JXL doesn't use it).
if ($OUT_TIF | path exists) {
    print $"==> ($OUT_TIF) exists — delete it to re-merge; skipping"
} else {
    print "==> Merging tiles"
    let idx = "shading_index"
    glob $"($TILES_DIR)/*.tif" | save -f $idx
    print $"  (open $idx | lines | length) tiles"
    gdalbuildvrt -input_file_list $idx shading.vrt
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
