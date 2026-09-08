#!/usr/bin/env nu

# Generate shaded relief for Belgium. Belgium port of shading-lu.nu.
#
# TWO REGIONS, TWO AUTHORITIES, ONE CRS.
#
#   Wallonia  SPW, LiDAR 2021-2022, MNT 1 m, ~0.12 m vertical, CC BY 4.0.
#             Fetched by download-wa.nu as GeoTIFF tiles in EPSG:3812.
#             This is the half that matters: the Ardennes.
#
#   Flanders  Digitaal Vlaanderen, DHMV II, DTM 1 m, flown 2013-2015 at
#             >=8 points/m2, open data, no restrictions. Served live over
#             WCS 2.0.1 — no download step at all, GDAL reads it as a single
#             247000 x 102000 virtual raster. There is no DHMV III.
#
#   IT IS NOT CLIPPED TO THE FLEMISH REGION. Verified 2026-08-30: the WCS
#   returns real terrain over the whole Brussels-Capital enclave (Grand-Place,
#   Uccle, Foret de Soignes all 100% valid) and several km INTO Wallonia
#   (Waterloo, 6 km south of the border, 118-129 m). Outside its coverage it
#   returns -9999 — the same sentinel Wallonia uses — so the unified vrtnodata
#   and the has-data check both behave. Brussels is therefore NOT a hole in the
#   country, which it would have been had DHMV stopped at the region boundary.
#
#   TRANSIENT 502s. The endpoint is behind an Azure Application Gateway and does
#   return "502 Bad Gateway" under load. Per-window failures land in failed/ and
#   are retried on the next run, which is exactly what that machinery is for —
#   but expect a non-zero failed/ count on the first pass of a large run.
#
# EPSG:3812 (ETRS89 / Belgian Lambert 2008) FOR BOTH, DELIBERATELY.
#   Wallonia's 1 m product ships only in 3812, and Flanders' WCS advertises 3812
#   alongside its native 31370. Taking 3812 for both means:
#     - ONE window grid for the whole country, no seam at the language border;
#     - NO datum hazard. projinfo gives exactly one 3812 -> WGS 84 operation and
#       it is a null transform (ETRS89-based). EPSG:31370 (Lambert 72, BD72)
#       offers THREE candidates at 1-5 m, which is the England/OSTN15 trap:
#       PROJ silently picks one and the product lands metres off OSM with no
#       error raised. Do not "simplify" this to 31370.
#
# THE REAL SEAM IS TEMPORAL, NOT SPATIAL. If Flanders is enabled, the two halves
#   were flown 6-8 years apart with different sensors. The CRS matches, so
#   geometry lines up, but expect a radiometric step along the border where one
#   side has newer quarries, embankments and building pads than the other.
#   Nothing here blends it; if it looks bad, that is the reason.
#
# ZOOM=17, MEASURED 2026-08-28 on smoothed data through this exact pipeline.
#   Three 850 m samples, each coarser render resampled onto the z17 grid and
#   differenced (grey levels, 0-255):
#
#     La Roche-en-Ardenne  z15  mean 2.46  p95 11  10.63% of px off by >5
#                          z16  mean 1.26  p95  5   4.20%
#     Rochers de Freyr     z15  mean 2.16  p95 10   9.65%
#                          z16  mean 1.32  p95  5   4.73%
#     Hesbaye plateau      z16  mean 0.45  p95  1   0.62%
#
#   For comparison: Luxembourg z16 was 2.57% and went to z17; England z16 was
#   3.20% and stayed at z16. Wallonia's Ardennes lose MORE than either, and the
#   cost argument that decided England does not apply — Wallonia is 16900 km2,
#   so z17 is roughly 8 GB against 2 GB, not 137 GB against 44 GB.
#
#   The Hesbaye row matters too: over flat Wallonia the zoom is irrelevant
#   (0.62%), so the decision rests entirely on the Ardennes third of the region.
#
#   MEASURE ON SMOOTHED DATA — this is the trap. Differencing unsmoothed renders
#   overstates the case for a finer zoom by about 3x, because --filter 11 strips
#   most sub-11 m variation BEFORE the hillshade is computed. Luxembourg measured
#   8.5% unsmoothed against 2.6% smoothed for the same comparison, and the
#   unsmoothed figure produced a confidently wrong recommendation.
#
# NO gdal_fillnodata, MEASURED (2026-08-27) — same finding as Luxembourg.
#   Random 1200x1200 windows per province, inland only, nodata blobs labelled:
#
#     Brabant Wallon   68 Mpx   3.01% nodata   1 blob <= 25 px
#     Liege            92 Mpx   4.24% nodata   0 blobs <= 25 px
#     Luxembourg (BE) 114 Mpx   1.73% nodata   0 blobs <= 25 px
#
#   Every void is >1000 px (largest ~692k) — water bodies and coverage edges,
#   which must STAY nodata so they come out transparent, and which -md 5 would
#   not touch anyway. The Ardennes provinces were checked specifically because
#   forest canopy was the plausible source of speckle; there is none. So the
#   step would cost a read+write per window across ~2700 windows to fix one
#   pixel cluster in the whole region.
#
#   FLANDERS RE-MEASURED 2026-08-30 before enabling it, because at a quarter of
#   Wallonia's density and a decade older it was a genuinely separate question.
#   27 windows / 27 Mpx inside coverage: 0.0000% nodata, NO voids of any size.
#   The delivered DHMV II raster is already gap-free within its footprint, so
#   the step stays off for both halves. To restore it:
#     let dem = $"($d)/dem.tif"
#     gdal_fillnodata.py -md 5 $smooth $dem
#   and point step 5/6 at $dem instead of $smooth.
#
# PREDICTOR=1 on the window DEM is LOAD-BEARING — feature-preserving-smoothing
#   does I/O via `wbgeotiff`, which ignores the TIFF Predictor tag (317) and
#   decodes PREDICTOR=2/3 float data as garbage (+/-Inf) WITHOUT erroring.
#
# ALWAYS RUN VIA `conda run -n geo`, never by putting the env's bin on PATH —
#   that leaves PROJ_DATA unset, degrades every CRS to ENGCRS["unnamed"], and
#   the warp fails with "Cannot find coordinate operations" hours in.
#
# Smoothing is 11/16/6/6, the PL/HR/NO/EN/LU 1 m settings.
#
# Resumable at window granularity; empty windows leave a .empty marker; failures
# land in failed/ and are retried on the next run. Run via:
#   nice ~/miniforge3/bin/conda run --no-capture-output -n geo nu ~/fm/freemap-outdoor-map/shading-be.nu

# ── Configuration ─────────────────────────────────────────────────────────────

const INCLUDE_FLANDERS = true                    # see header
const FLANDERS_WCS = "WCS:https://geo.api.vlaanderen.be/DHMV/wcs?version=2.0.1&coverage=DHMVII_DTM_1m"

const DATA_ROOT = "/mnt/osm/be"                  # VRT, smooth2m/ on NVMe
const TILES_DIR = "/mnt/osm/be/tiles"
const EPSG      = "EPSG:3812"                    # ETRS89 / Belgian Lambert 2008
const NODATA    = "-9999"
const VRT       = "be.vrt"
const ZOOM      = 17                             # MEASURED — see header
const PARALLEL  = 24
const TMPDIR    = "/dev/shm"

const STEP      = 2500                           # window size, m (= px at 1 m)
const COLLAR    = 6
const CROP      = 3

# WINDOW GRID ORIGIN IS PINNED, NOT DERIVED FROM THE DATA EXTENT.
#   Deriving it (floor(extent_min / STEP) * STEP) was the original design and it
#   is a trap: the moment the VRT grows — adding Flanders moved the minimum x
#   from 542248 to 516991 — the origin moves with it, every window id changes,
#   and a resumable run silently treats thousands of finished tiles as pending.
#   Pinning the origin below any plausible extent makes ids absolute and stable
#   forever, so adding a region renders only the genuinely new windows.
#
#   These values sit below the Flanders+Wallonia union (516991, 521173) on the
#   STEP grid. Do NOT change them: doing so renames every tile.
const GRID_X0   = 515000
const GRID_Y0   = 520000

const SM_FILTER    = 11
const SM_NORM_DIFF = 16
const SM_NUM_ITER  = 6
const SM_MAX_DIFF  = 6

# ── Drive discovery ───────────────────────────────────────────────────────────
# udisks2 moves the removable 18TB between /media and /run/media, and the unused
# path survives as an empty root-owned directory on / — a stale hardcoded path
# silently fills the root filesystem instead of failing. Pick the real one.
def find-drive []: nothing -> string {
    let found = (
        ["/run/media/martin/18TB" "/media/martin/18TB"]
          | where {|p| (do { mountpoint -q $p } | complete).exit_code == 0 }
    )
    if ($found | is-empty) {
        error make {msg: "the 18TB drive is not mounted at /media/martin/18TB or /run/media/martin/18TB. Check `lsblk -o NAME,LABEL,SIZE,MOUNTPOINT` (label 18TB)."}
    }
    $found | first
}

let DRIVE   = (find-drive)
let SRC_DIR = $"($DRIVE)/be/MNT1M"               # Wallonia tiles from download-wa.nu
let OUT_TIF = $"($DRIVE)/be/shading.tif"

print $"==> drive: ($DRIVE)"

# PROJ sanity — a missing PROJ database does not fail loudly, it degrades every
# CRS to ENGCRS and breaks the warp long after the run has started.
let _probe = (do { gdalsrsinfo -o proj4 $EPSG } | complete)
if $_probe.exit_code != 0 or ($_probe.stdout | str trim | is-empty) {
    error make {msg: $"PROJ cannot resolve ($EPSG) — run via: nice ~/miniforge3/bin/conda run --no-capture-output -n geo nu shading-be.nu"}
}

# ── Helpers ───────────────────────────────────────────────────────────────────

# A FALSE here writes a permanent .empty marker, so it must mean "gdalinfo ran
# and found no valid pixel", never "gdalinfo did not run".
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

def band-calc [wa: string, wb: string, wc: string]: nothing -> string {
    let ea  = "0.8 * (255 - A)"
    let eb  = "0.7 * (255 - B)"
    let ec  = "1.0 * (255 - C)"
    let num = $"($ea) * ($wa) + ($eb) * ($wb) + ($ec) * ($wc)"
    let den = $"0.01 + ($ea) + ($eb) + ($ec)"
    "((" + $num + ") / (" + $den + ") - 128.0) + 128.0"
}

def alpha-calc []: nothing -> string {
    let ea = "0.8 * (255 - A)"
    let eb = "0.7 * (255 - B)"
    let ec = "1.0 * (255 - C)"
    "255.0 - 255.0 * ((1.0 - " + $ea + " / 255.0) * (1.0 - " + $eb + " / 255.0) * (1.0 - " + $ec + " / 255.0))"
}

def render-window [w: record, tr: string, vrt: string, data_dir: string]: nothing -> nothing {
    let out = $"($TILES_DIR)/($w.id).tif"
    let d   = $"($TMPDIR)/be_($w.id)"

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
      $vrt $win o> /dev/null)

    if not (has-data $win) {
        rm -rf $d
        touch $"($TILES_DIR)/($w.id).empty"
        print $"  ($w.id): empty"
        return
    }

    # 2. Smooth. A zero-variance window panics the smoother and has nothing to
    #    smooth anyway — pass it through.
    let band = gdalinfo -json -mm $win err> /dev/null | from json | get bands | first
    let smooth = $"($d)/smooth.tif"
    if ($band.computedMin? == $band.computedMax?) {
        cp $win $smooth
    } else {
        (feature-preserving-smoothing --dem $win -o $smooth
          --filter $SM_FILTER --norm_diff $SM_NORM_DIFF
          --num_iter $SM_NUM_ITER --max_diff $SM_MAX_DIFF)
    }

    # 3. 2 m DEM for contours-be.nu — collar cropped, nodata-aware `average`.
    let dem2 = $"($data_dir)/smooth2m/($w.id).tif"
    if not ($dem2 | path exists) {
        let tmp2 = $"($d)/dem2m.tif"
        (gdal_translate -q -of GTiff
          -projwin $w.xmin $w.ymax $w.xmax $w.ymin
          -tr 2 2 -r average -a_nodata $NODATA
          ...$co $smooth $tmp2 o> /dev/null)
        mv $tmp2 $dem2
    }

    # 4. (no gdal_fillnodata — measured unnecessary, see header)

    # 5. Three Igor hillshades on the collared window so edges have neighbours.
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

    # 7. Warp each to EPSG:3857 at zoom-level pixel size. -tap puts every window
    #    on the same global grid so the tiles mosaic without seams.
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
    let vrt_stack = $"($d)/stack.vrt"
    gdalbuildvrt -separate $vrt_stack $"($d)/R.tif" $"($d)/G.tif" $"($d)/B.tif" $"($d)/A.tif" o> /dev/null
    gdal_edit.py -colorinterp_1 red -colorinterp_2 green -colorinterp_3 blue $vrt_stack o> /dev/null
    sed -i '/<NoDataValue>/d; /<NODATA>/d; /<SrcRect/d; /<DstRect/d; s/ComplexSource/SimpleSource/g' $vrt_stack
    sed -i 's|</VRTDataset>|<MaskBand><VRTRasterBand dataType="Byte"><SimpleSource><SourceFilename relativeToVRT="1">a-warped.tif</SourceFilename><SourceBand>1</SourceBand></SimpleSource></VRTRasterBand></MaskBand></VRTDataset>|' $vrt_stack

    (gdal_translate --config GDAL_TIFF_INTERNAL_MASK YES -of GTiff
      ...$co_big $vrt_stack $"($d)/final.tif" o> /dev/null)

    # gdal_calc.py can drop one of R/G/B/A without gdalbuildvrt -separate
    # complaining; gdalbuildvrt then SKIPS the tile at merge time behind a
    # warning swallowed by its progress bar, punching a silent hole in the
    # mosaic. England lost two tiles that way for real. Throw instead.
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

def process-window [w: record, tr: string, vrt: string, data_dir: string]: nothing -> nothing {
    try {
        render-window $w $tr $vrt $data_dir
    } catch {|e|
        let msg = ($e | get -o msg | default "unknown")
        print $"  !! ($w.id): FAILED — ($msg)"
        touch $"($data_dir)/failed/($w.id)"
        rm -rf $"($TMPDIR)/be_($w.id)"
    }
}

# ── Pipeline ──────────────────────────────────────────────────────────────────

mkdir $DATA_ROOT
mkdir $TILES_DIR
mkdir $"($DATA_ROOT)/smooth2m"
mkdir $"($DATA_ROOT)/failed"
cd $DATA_ROOT

if (not ($SRC_DIR | path exists)) or ((glob $"($SRC_DIR)/**/*.tif" | length) == 0) {
    error make {msg: $"($SRC_DIR) is empty — run download-wa.nu first"}
}

let pi = (1 | math arctan) * 4
let tr = ($pi * 2 * 6378137 / 256 / (2 ** $ZOOM) | into string)
print $"ZOOM=($ZOOM) TR=($tr)"

# ── 0. National VRT ───────────────────────────────────────────────────────────
# Wallonia tiles, optionally plus Flanders straight off the WCS. gdalbuildvrt
# takes the WCS as just another source; GDAL reads it lazily, so no download.
if ($VRT | path exists) {
    print $"==> ($VRT) exists — reusing \(delete to force a rebuild\)"
} else {
    print $"==> Building national VRT ($VRT) — unified nodata ($NODATA)"
    let idx = "_idx_be"
    let wal = (glob $"($SRC_DIR)/**/*.tif")
    print $"  Wallonia: ($wal | length) tiles"
    # ORDER MATTERS: gdalbuildvrt resolves overlaps last-listed-wins, and the two
    # sources DO overlap — DHMV II is not clipped to the Flemish Region, it covers
    # the Brussels enclave and spills several km into Wallonia (verified at
    # Waterloo, 6 km south of the border). Wallonia must therefore be listed LAST
    # so its 2021-2022 data at higher density wins the overlap strip; otherwise
    # 2013-2015 Flemish data would overwrite good Walloon ground and push the age
    # seam kilometres inside Wallonia instead of onto the region boundary.
    # FLANDERS MUST BE PRE-WARPED TO EPSG:3812 BEFORE IT CAN JOIN THE VRT.
    #   GDAL reports the WCS in its native EPSG:31370, and gdalbuildvrt SILENTLY
    #   SKIPS sources whose CRS disagrees with the first one. Listing the raw WCS
    #   alongside the 3812 province rasters produced a VRT containing ONE source
    #   — Flanders alone, all five Wallonia rasters dropped behind a warning
    #   swallowed by the progress bar. Only the pinned-grid guard caught it.
    #
    #   The warp also fixes a datum problem. 31370 is BD72; going straight to
    #   EPSG:3857 offers PROJ nothing better than a 1 m Helmert. Routing through
    #   3812 (ETRS89) uses the official IGN NTv2 grid at 0.01 m — measured
    #   deviation from the Helmert is 0.14 m mean / 0.33 m max, i.e. under half a
    #   z17 pixel, so this is correctness housekeeping rather than a rescue.
    #   Requires be_ign_bd72lb72_etrs89lb08.tif; install with
    #     projsync --file be_ign_bd72lb72_etrs89lb08.tif
    #   It lands in ~/.local/share/proj and works with PROJ_NETWORK=OFF.
    #
    #   -of VRT keeps it lazy: no pixels are fetched until a window is cut.
    let sources = if $INCLUDE_FLANDERS {
        let fl_vrt = "zone_fl.vrt"
        if not ($fl_vrt | path exists) {
            print "  Flanders: warping WCS 31370 -> 3812 (lazy VRT)"
            (gdalwarp -q -of VRT -t_srs EPSG:3812 -tr 1 1 -r bilinear
              -srcnodata $NODATA -dstnodata $NODATA
              $FLANDERS_WCS $"($fl_vrt).tmp")
            # SECOND SILENT-SKIP CAUSE, distinct from the CRS one: gdalbuildvrt
            # also drops sources whose band COLOUR INTERPRETATION disagrees with
            # the first input. A warped VRT comes out Undefined while the
            # province GeoTIFFs are Gray, which silently reduced the mosaic to
            # one source again. Force it to match.
            gdal_edit.py -colorinterp_1 gray $"($fl_vrt).tmp"
            mv $"($fl_vrt).tmp" $fl_vrt
        }
        print "  Flanders: zone_fl.vrt — listed first, Wallonia wins overlaps"
        ([$fl_vrt] | append $wal)
    } else {
        print "  Flanders: SKIPPED (INCLUDE_FLANDERS = false)"
        $wal
    }
    $sources | str join "\n" | save -f $idx
    gdalbuildvrt -vrtnodata $NODATA -input_file_list $idx $"($VRT).tmp" o> /dev/null

    # VERIFY EVERY SOURCE LANDED. gdalbuildvrt silently SKIPS inputs that
    # disagree with the first one, reporting it only in a warning swallowed by
    # its progress bar. TWO different causes hit this build for real:
    #   - CRS mismatch    (raw WCS is 31370, provinces are 3812)
    #   - colour interp   (warped VRT is Undefined, provinces are Gray)
    # Either one reduced the mosaic to a single source. The merge step has this
    # guard already; the input VRT needs it just as much, because a dropped
    # source here means a whole region silently missing from the country.
    let n_offered = ($sources | length)
    let n_used = (
        open --raw $"($VRT).tmp"
          | parse -r '<SourceFilename[^>]*>([^<]+)</SourceFilename>'
          | get capture0 | uniq | length
    )
    if $n_used != $n_offered {
        rm -f $"($VRT).tmp"
        rm $idx
        error make {msg: $"gdalbuildvrt kept only ($n_used) of ($n_offered) sources — the rest were silently skipped \(CRS or colour-interpretation mismatch\). Run gdalbuildvrt by hand to see the warnings."}
    }
    print $"  verified: all ($n_offered) sources present in ($VRT)"

    rm $idx
    mv $"($VRT).tmp" $VRT
}

# ── 1. Window grid from the VRT extent ────────────────────────────────────────
print "==> Building window list"
let gi = (gdalinfo -json $VRT | from json)
let gt = $gi.geoTransform
let rx0 = $gt.0
let ry1 = $gt.3
let rx1 = ($rx0 + ($gi.size.0 | into float) * $gt.1)
let ry0 = ($ry1 + ($gi.size.1 | into float) * $gt.5)
print $"  extent ($rx0), ($ry0) -> ($rx1), ($ry1)"

# Indices are absolute against the pinned origin; only the range covering the
# current extent is iterated, so ids never move when the extent changes.
if $rx0 < $GRID_X0 or $ry0 < $GRID_Y0 {
    error make {msg: $"data extent \(($rx0), ($ry0)\) starts before the pinned grid origin \(($GRID_X0), ($GRID_Y0)\) — lower GRID_X0/GRID_Y0, but note that renames every tile"}
}
let i0 = ((($rx0 - $GRID_X0) / $STEP) | math floor)
let i1 = (((($rx1 - $GRID_X0) / $STEP) | math ceil) - 1)
let j0 = ((($ry0 - $GRID_Y0) / $STEP) | math floor)
let j1 = (((($ry1 - $GRID_Y0) / $STEP) | math ceil) - 1)
let ncol = ($i1 - $i0 + 1)
let nrow = ($j1 - $j0 + 1)

let windows = (
    $i0..$i1 | each {|i|
        $j0..$j1 | each {|j|
            let xmin = ($GRID_X0 + ($i * $STEP | into float))
            let ymin = ($GRID_Y0 + ($j * $STEP | into float))
            {
              id:   $"c(($i | fill -a r -c '0' -w 3))r(($j | fill -a r -c '0' -w 3))"
              xmin: $xmin
              xmax: ($xmin + ($STEP | into float))
              ymin: $ymin
              ymax: ($ymin + ($STEP | into float))
            }
        }
    } | flatten
)
print $"  ($ncol) x ($nrow) grid -> ($windows | length) windows"

# ── 2. Process pending windows ────────────────────────────────────────────────
let pending = (
    $windows | where {|w|
        not ($"($TILES_DIR)/($w.id).tif" | path exists) and not ($"($TILES_DIR)/($w.id).empty" | path exists)
    }
)
print $"==> ($windows | length) windows, ($pending | length) pending"

if ($pending | length) > 0 {
    $pending | par-each -t $PARALLEL {|w| process-window $w $tr $VRT $DATA_ROOT}
}

# ── 3. Refuse to merge a partial country ──────────────────────────────────────
let unfinished = (
    $windows | where {|w|
        not ($"($TILES_DIR)/($w.id).tif" | path exists) and not ($"($TILES_DIR)/($w.id).empty" | path exists)
    }
)
let failed = (glob $"($DATA_ROOT)/failed/*" | each {|f| $f | path basename })

print "==> Checking every tile has 4 bands (gdalbuildvrt skips those that do not)"
let malformed = (
    glob $"($TILES_DIR)/*.tif"
      | par-each -t $PARALLEL {|f|
          let n = (do { gdalinfo -json $f } | complete)
          if $n.exit_code != 0 { {file: $f, bands: -1} } else { {file: $f, bands: ($n.stdout | from json | get bands | length)} }
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
        print $"    ($failed | length) window\(s\) in ($DATA_ROOT)/failed/ — delete the marker\(s\) and re-run to retry:"
        $failed | first 10 | each {|f| print $"      ($f)" }
    }
    print "    Re-run this script; finished windows are skipped, so it resumes cheaply."
    exit 1
}

# ── 4. Merge and build overviews ──────────────────────────────────────────────
if ($OUT_TIF | path exists) {
    print $"==> ($OUT_TIF) exists — rename it \(don't delete\) to re-merge; skipping"
} else {
    print "==> Merging tiles"
    let idx = "shading_index"
    glob $"($TILES_DIR)/*.tif" | save -f $idx
    print $"  (open $idx | lines | length) tiles"
    gdalbuildvrt -input_file_list $idx shading.vrt

    # gdalbuildvrt silently SKIPS inputs whose band count, type or colour
    # interpretation disagrees with the first, reporting it only in a warning
    # swallowed by its progress bar. Verifying that every offered tile landed in
    # the VRT catches all such defects, including ones not yet met.
    let offered = (open $idx | lines | each {|f| $f | path basename})
    let used = (
        open --raw shading.vrt
          | parse -r '<SourceFilename[^>]*>([^<]+)</SourceFilename>'
          | get capture0 | each {|f| $f | path basename} | uniq
    )
    let dropped = ($offered | where {|f| $f not-in $used})
    if ($dropped | is-not-empty) {
        print ""
        print $"==> NOT MERGING: gdalbuildvrt dropped ($dropped | length) of ($offered | length) tiles."
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
