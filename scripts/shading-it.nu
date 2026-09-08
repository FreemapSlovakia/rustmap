#!/usr/bin/env nu

# Generate shaded relief for Italy from the national 5 m HRDTM.
# Italy port of shading.nu (which targets Poland's 1 m GUGiK DTM).
#
# Source: /mnt/osm/it/it.tif — ONE 22 GB file, 195520 x 257218 px, Float32,
# 5 m pixels, EPSG:6875 (RDN2008 / Italy zone N-E), nodata = -9999.
#
# Differences from the Poland script, and why:
#
#  * Source is a single raster, not a tile tree, so there is no source VRT step —
#    gdal_retile.py reads it.tif directly.
#
#  * nodata is a proper -9999, not Poland's overloaded 0. Poland's WCS returned 0
#    both for out-of-coverage AND for genuine 0.00 m coastal terrain, which forced
#    the speck-list / heal machinery. None of that applies here. gdal_fillnodata is
#    still run per tile to close small interior voids (they would otherwise show as
#    transparent specks in the relief); large sea regions stay nodata → transparent.
#
#  * PREDICTOR=1 on the retile is LOAD-BEARING, not a style choice. Our
#    feature-preserving-smoothing does I/O via the `wbgeotiff` crate, which does not
#    parse the TIFF Predictor tag (317) at all — it inflates the DEFLATE stream and
#    returns the bytes as-is. Fed PREDICTOR=3 data those bytes are still byte-shuffled
#    deltas, so they decode as float garbage and the tool emits ±Inf / f32::MAX
#    rasters that render as static. It does NOT error: read_band_f32 returns Ok.
#    it.tif itself is PREDICTOR=3, so the retile is what launders it. Do not
#    "optimise" this to PREDICTOR=3. (Verified on a real retiled tile, 2026-07-17.)
#
#  * ZOOM=15, not 16. EPSG:3857 metres inflate by 1/cos(lat); Italy spans 36-47°N,
#    so z15 = 4.78 3857-m/px lands at ~3.3-3.9 ground m/px against a 5 m source
#    (1.3-1.5x oversampled). z14 would be ~6.5-7.7 ground m/px — undersampled by the
#    same factor, throwing away real detail. No zoom lands on 5 m; z15 is the safe
#    side of the miss. z16 would be 4x the pixels for pure interpolation.
#
#  * Smoothing is --filter 9 --norm_diff 15 --num_iter 5 --max_diff 5, softer than
#    Poland's 11/16/6/6 — a filter is a pixel count, so 11 px is 11 m on 1 m data but
#    55 m on 5 m data. Measured on a Umbria sample (42.56°N 12.66°E): 9/15/5/5 moves
#    the surface 0.38 m mean / 1.7 m p99, Poland's 11/16/6/6 moves 0.43 m / 2.0 m —
#    both negligible vs a 10 m contour interval, so this is an aesthetic call, not a
#    correctness one. Raise the filter if the relief still reads as noisy.
#    NOTE: parts of the source are contour-interpolated and carry corduroy striping
#    and TIN facets. Feature-preserving smoothing treats those as FEATURES and will
#    not remove them at any setting — a low-pass prefilter would be needed instead.
#
# Resumable at every stage: retiled/, smooth/ and tiles/ all skip existing outputs.
# Delete a stage's directory to force a rebuild. Run via:
#   nice ~/miniforge3/bin/conda run --no-capture-output -n geo nu ~/fm/freemap-outdoor-map/shading-it.nu

# ── Configuration ─────────────────────────────────────────────────────────────

const DATA_DIR = "/mnt/osm/it"
const SRC      = "/mnt/osm/it/it.tif"
const ZOOM     = 15          # target zoom; z15 ≈ 3.3-3.9 ground m/px over Italy vs 5 m source
const PARALLEL = 24          # tiles processed in parallel
const PS       = 2000        # retile tile size, px
const OVERLAP  = 6           # half of retile overlap; hillshading context
const CROP     = 3           # px cropped per edge after hillshading; OVERLAP - CROP leaves warp margin
const TMPDIR   = "/dev/shm"  # ramdisk for intermediates

# Smoothing (see header — tuned for 5 m, softer than Poland's 1 m settings)
const SM_FILTER    = 9
const SM_NORM_DIFF = 15
const SM_NUM_ITER  = 5
const SM_MAX_DIFF  = 5

# De-doubling: repair 2x-replicated (10 m-into-5 m) patches before smoothing. Italy's
# HRDTM is a mosaic; the Dolomites / high Alps are 10 m data pixel-doubled into the
# 5 m grid, which makes staircase contours and terraced hillshading. dedouble_dem.py
# detects those patches (exact-equality signature) and reconstructs the true 10 m
# surface there, leaving real 5 m LiDAR untouched. Run BEFORE smoothing so both the
# hillshade and the contours (which read smooth/) benefit. Uses the SYSTEM python3,
# not the geo env — it needs scipy (absent from geo) and only touches DEFLATE, no JXL.
const DEDOUBLE   = "/home/martin/fm/freemap-outdoor-map/dedouble_dem.py"
const SYS_PYTHON = "/usr/bin/python3"

# ── Helpers ───────────────────────────────────────────────────────────────────

def has-data [file: string]: nothing -> bool {
    let band = gdalinfo -json -mm $file err> /dev/null | from json | get bands | first
    ($band | get -o computedMin | is-not-empty)
}

# Weighted multi-directional hillshade blend formula for one RGB band.
# wa/wb/wc are hex weights for azimuth directions a/b/c.
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

# Process one smooth tile into a warped RGBA shaded-relief GeoTIFF.
# Output goes to tiles/<stem>.tif ; uses a tmp dir for crash-safe resumability.
def process-tile [src: string, tr: string]: nothing -> nothing {
    let stem = $src | path basename | path parse | get stem
    let d    = $"($TMPDIR)/shading_($stem)"
    let out  = $"tiles/($stem).tif"
    print $"  tile ($stem): hillshade"

    rm -rf $d
    mkdir $d

    let co      = [-co COMPRESS=ZSTD -co PREDICTOR=2 -co TILED=YES -co NUM_THREADS=ALL_CPUS]
    let co_big  = [...$co -co BIGTIFF=YES]
    let co_calc = [--co=COMPRESS=ZSTD --co=PREDICTOR=2 --co=TILED=YES --co=NUM_THREADS=ALL_CPUS --co=BIGTIFF=YES]

    # Close small interior voids so they don't become transparent specks in the
    # relief. Large out-of-coverage regions (sea) exceed -md 5 and stay nodata,
    # hence transparent via the mask band — flat water has ~0 alpha anyway.
    let dem = $"($d)/dem.tif"
    gdal_fillnodata.py -md 5 $src $dem o> /dev/null err> /dev/null

    # Three hillshades at different azimuths (run on the overlapped smooth tile so
    # edges have real neighbours)
    gdaldem hillshade $dem $"($d)/_a.tif" -az -120 -igor -compute_edges ...$co o> /dev/null
    gdaldem hillshade $dem $"($d)/_b.tif" -az  60  -igor -compute_edges ...$co o> /dev/null
    gdaldem hillshade $dem $"($d)/_c.tif" -az -45  -igor -compute_edges ...$co o> /dev/null

    # Crop overlap from hillshades — discards edge pixels degraded by smoothing
    let info = gdalinfo -json $src | from json
    let w = $info.size.0
    let h = $info.size.1
    for name in [a b c] {
        let raw = $"($d)/_($name)_raw.tif"
        mv $"($d)/_($name).tif" $raw
        gdal_translate -srcwin $CROP $CROP ($w - 2 * $CROP) ($h - 2 * $CROP) ...$co $raw $"($d)/_($name).tif" o> /dev/null
        rm $raw
    }

    # Warp each to EPSG:3857 at zoom-level pixel size
    print $"  tile ($stem): warp"
    for name in [a b c] {
        gdalwarp -t_srs EPSG:3857 -tr $tr $tr -tap -r cubic -dstnodata none -of GTiff ...$co_big -multi -wo NUM_THREADS=ALL_CPUS -wo INIT_DEST=0 $"($d)/_($name).tif" $"($d)/($name)-warped.tif" o> /dev/null
    }

    # Compute RGBA bands from the three warped hillshades
    print $"  tile ($stem): bands"
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

    # Stack RGBA into a VRT with the alpha as internal mask
    print $"  tile ($stem): stack + translate"
    let vrt = $"($d)/stack.vrt"
    gdalbuildvrt -separate $vrt $"($d)/R.tif" $"($d)/G.tif" $"($d)/B.tif" $"($d)/A.tif" o> /dev/null
    gdal_edit.py -colorinterp_1 red -colorinterp_2 green -colorinterp_3 blue $vrt o> /dev/null
    sed -i '/<NoDataValue>/d; /<NODATA>/d; /<SrcRect/d; /<DstRect/d; s/ComplexSource/SimpleSource/g' $vrt
    sed -i 's|</VRTDataset>|<MaskBand><VRTRasterBand dataType="Byte"><SimpleSource><SourceFilename relativeToVRT="1">a-warped.tif</SourceFilename><SourceBand>1</SourceBand></SimpleSource></VRTRasterBand></MaskBand></VRTDataset>|' $vrt

    # Translate to final GeoTIFF
    gdal_translate --config GDAL_TIFF_INTERNAL_MASK YES -of GTiff ...$co_big $vrt $"($d)/final.tif" o> /dev/null

    mv $"($d)/final.tif" $out
    rm -rf $d
    print $"  tile ($stem): done"
}

# ── Pipeline ──────────────────────────────────────────────────────────────────

cd $DATA_DIR

let pi = (1 | math arctan) * 4
let tr = ($pi * 2 * 6378137 / 256 / (2 ** $ZOOM) | into string)
print $"ZOOM=($ZOOM) TR=($tr)"

# 1. Retile the national DTM with overlap (overlap is kept through processing to
#    avoid hillshade edge artifacts). PREDICTOR=1 is required — see header.
print "==> Retiling"
mkdir retiled
if ((glob retiled/*.tif | length) > 0) {
    print "  retiled/ is non-empty — reusing (delete the dir to force a re-tile)"
} else {
    (gdal_retile.py $SRC -ps $PS $PS -overlap 12 -targetDir retiled
      -co COMPRESS=DEFLATE -co PREDICTOR=1)
}

# 2. Smooth tiles — resumable, skips existing and all-nodata (sea) tiles
print "==> Smoothing"
mkdir smooth
mkdir smooth/_tmp
(
  glob retiled/*.tif
    | where {|f| not ($"smooth/($f | path basename)" | path exists)}
    | where {|f| has-data $f}
    | par-each -t $PARALLEL {|f|
        let a    = $f | path basename
        let dst  = $"smooth/($a)"
        let band = gdalinfo -json -mm $f err> /dev/null | from json | get bands | first
        let cmin = $band | get -o computedMin
        let cmax = $band | get -o computedMax
        if ($cmin == $cmax) {
            # Flat tile (constant elevation, e.g. sea-level coast sliver) — the smoother
            # panics on zero-variance input (carried over from the Poland/Spain runs).
            # A flat tile has no 10 m relief so de-doubling would no-op anyway.
            print $"  copy ($a)"
            cp $f $dst
        } else {
            let dd  = $"smooth/_tmp/dd_($a)"     # de-doubled (system python3, has scipy)
            let tmp = $"smooth/_tmp/($a)"
            print $"  dedouble+smooth ($a)"
            (^$SYS_PYTHON $DEDOUBLE $f $dd)
            (feature-preserving-smoothing --dem $dd -o $tmp
              --filter $SM_FILTER --norm_diff $SM_NORM_DIFF
              --num_iter $SM_NUM_ITER --max_diff $SM_MAX_DIFF)
            mv $tmp $dst
            rm -f $dd
        }
      }
)

# 3. Process each smooth tile into shaded relief — resumable
print "==> Processing tiles"
mkdir tiles
(
  glob smooth/*.tif
    | sort
    | where {|f| not ($"tiles/($f | path basename | path parse | get stem).tif" | path exists)}
    | par-each -t $PARALLEL {|src| process-tile $src $tr}
)

# 4. Merge all tiles and build overviews
# Final shading.tif uses JXL (lossy, distance=3.0); requires GDAL linked against
# a libtiff with libjxl support — see README. PREDICTOR is intentionally omitted
# (JXL doesn't use it).
print "==> Merging tiles"
glob tiles/*.tif | save -f shading_index
gdalbuildvrt -input_file_list shading_index shading.vrt
sed -i 's|<ColorInterp>Alpha</ColorInterp>|<ColorInterp>Undefined</ColorInterp>|g' shading.vrt
gdal_translate --config GDAL_TIFF_INTERNAL_MASK YES --config GDAL_TIFF_INTERNAL_MASK_TO_8BIT YES -of GTiff -co COMPRESS=JXL -co JXL_LOSSLESS=NO -co JXL_DISTANCE=3.0 -co TILED=YES -co BLOCKXSIZE=256 -co BLOCKYSIZE=256 -co BIGTIFF=YES -co NUM_THREADS=ALL_CPUS shading.vrt shading.tif
rm shading.vrt shading_index
gdal_edit.py -colorinterp_4 alpha shading.tif
print "==> Building overviews"
gdaladdo --config GDAL_TIFF_INTERNAL_MASK YES --config GDAL_CACHEMAX 4096 --config GDAL_NUM_THREADS ALL_CPUS --config COMPRESS_OVERVIEW JXL --config JXL_LOSSLESS_OVERVIEW NO --config JXL_DISTANCE_OVERVIEW 3.0 -r average shading.tif
print "==> Done"
