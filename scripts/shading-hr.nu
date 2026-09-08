#!/usr/bin/env nu

# Generate shaded relief for Croatia from the national 1 m LiDAR DMR (DGU HR).
# Croatia port of shading.nu (Poland 1 m) / shading-it.nu (Italy 5 m).
#
# Source: /run/media/martin/2190983A5767510F/croatia-dtm/DGU_HR_LIDAR_G1_G2_DMR
#   — a flat tree of 62351 GeoTIFF tiles, each 1200 x 800 px, Float32, 1 m pixels,
#   EPSG:3765 (HTRS96 / Croatia TM). The tiles are the raw provider delivery and
#   carry two quirks that the Poland/Italy single-file sources did not:
#
#   QUIRK 1 — inconsistent nodata. The delivery mixes four nodata sentinels across
#   tiles: -3.4028235e+38 (~49%), -99 (~42%), -32767 (~8.5%) and 0 (~0.8%). All four
#   sit far below any real Croatian terrain, so each tile's own declared nodata is
#   correct for that tile. gdalbuildvrt honours per-source nodata (writes a ComplexSource
#   <NODATA> per tile) and lets us present ONE unified nodata (-9999) downstream —
#   see stage 0b. The ~0.8% of tiles that use 0 as nodata lose genuine 0.00 m pixels,
#   but those tiles are coastal/offshore where 0 = sea = out-of-coverage anyway.
#
#   QUIRK 2 — ~45% of tiles have NO embedded CRS (gdalinfo reports a null projection,
#   they ship only the .tfw geotransform). gdalbuildvrt normally SKIPS null-projection
#   tiles ("heterogeneous projection ... got (null)"), which would silently drop nearly
#   half the country. Every tile is genuinely EPSG:3765, so stage 0 builds the mosaic
#   with `-a_srs EPSG:3765 -allow_projection_difference`: -allow_projection_difference
#   stops the skip and -a_srs stamps the CRS onto the output. -a_srs ALONE is NOT enough
#   (it only labels output; the null tiles still mismatch the reference CRS and skip).
#   Safe because every tile IS EPSG:3765 and gdalbuildvrt never reprojects — the null
#   ones merely lack the label. Verified tile placement to the pixel (2026-07-30).
#
#   QUIRK 3 — 9 tiles declare NO nodata at all. gdalbuildvrt emits those as SimpleSource
#   (unmasked). Six are full-coverage mountain tiles (harmless), but three inland G2-W09
#   tiles carry UNMARKED 0-value LiDAR voids (10-35% exact-0 blobs amid 800-1000 m
#   relief). Stage 0 rewrites the SimpleSources to ComplexSource with <NODATA>0> so the
#   voids mask cleanly. NOTE for downstream users (e.g. elevation sampling on fm6): a
#   raw gdalbuildvrt would return 0 m over those three voids — apply the same fix.
#
# Differences from the Poland script, and why:
#
#  * Source is a tile tree, so stage 0 normalises it into ONE national VRT (hr.vrt)
#    before retiling — assign CRS + unify nodata, both via gdalbuildvrt flags in a
#    single command. Non-destructive: the provider tiles on the external HDD are never
#    modified and nothing is written per tile; all state lives on NVMe.
#
#  * nodata is clean (each tile declares a real out-of-coverage sentinel), so NONE of
#    Poland's zero-speck / heal machinery applies — that existed only because GUGiK's
#    WCS overloaded 0 for both out-of-coverage AND genuine 0.00 m terrain. gdal_fillnodata
#    is still run per tile to close small interior voids (else transparent specks in the
#    relief); large out-of-coverage regions stay nodata -> transparent via the mask band.
#
#  * NO de-doubling. That was Italy-specific (its HRDTM mosaics 10 m data pixel-doubled
#    into the 5 m grid). Croatia is uniform 1 m LiDAR, so dedouble_dem.py is not used.
#
#  * PREDICTOR=1 on the retile is LOAD-BEARING, not a style choice. feature-preserving-
#    smoothing does I/O via the `wbgeotiff` crate, which does not parse the TIFF Predictor
#    tag (317) — fed PREDICTOR=2/3 float data it decodes byte-shuffled deltas as garbage
#    and emits ±Inf / f32::MAX rasters that render as static, WITHOUT erroring. Keep
#    PREDICTOR=1 on retiled/. (Documented in shading-it.nu, verified 2026-07-17.)
#
#  * ZOOM=16, like Poland (1 m source). Croatia spans ~42.4-46.5 degN; z16 = 2.39
#    3857-m/px lands at ~1.6-1.8 ground m/px against a 1 m source (safely oversampled).
#    z15 would be ~3.3-3.5 ground m/px — undersampled, discarding real LiDAR detail.
#
#  * Smoothing is Poland's 11/16/6/6 (filter is a pixel count, so 11 px = 11 m on 1 m
#    data — matched to the source resolution, NOT Italy's softer 9/15/5/5 which was
#    scaled for 5 m pixels).
#
# Resumable at every stage: src_vrt/, retiled/, smooth/ and tiles/ all skip existing
# outputs. Delete a stage's directory (or hr.vrt) to force a rebuild. Run via:
#   nice ~/miniforge3/bin/conda run --no-capture-output -n geo nu ~/fm/freemap-outdoor-map/shading-hr.nu

# ── Configuration ─────────────────────────────────────────────────────────────

const SRC_DIR  = "/run/media/martin/2190983A5767510F/croatia-dtm/DGU_HR_LIDAR_G1_G2_DMR"
const DATA_DIR = "/mnt/osm/hr"           # all working state on NVMe
const EPSG     = "EPSG:3765"             # HTRS96 / Croatia TM (all tiles, incl. the CRS-less ones)
const NODATA   = "-9999"                 # unified nodata presented by hr.vrt
const VRT      = "hr.vrt"                # national mosaic VRT (built in stage 0)
const ZOOM     = 16                      # z16 ~ 1.6-1.8 ground m/px over Croatia vs 1 m source
const PARALLEL = 24                      # tiles processed in parallel
const PS       = 2000                    # retile tile size, px
const OVERLAP  = 6                       # half of retile overlap; hillshading context
const CROP     = 3                       # px cropped per edge after hillshading; OVERLAP - CROP leaves warp margin
const TMPDIR   = "/dev/shm"              # ramdisk for intermediates

# Smoothing (Poland's 1 m settings — filter is a pixel count = metres on 1 m data)
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
    # relief. Large out-of-coverage regions exceed -md 5 and stay nodata, hence
    # transparent via the mask band — flat water has ~0 alpha anyway.
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

# 0. Normalise the provider tile tree into ONE national VRT (hr.vrt) in a single
#    gdalbuildvrt — both provider quirks are fixed by flags, non-destructively:
#      * -a_srs EPSG:3765 + -allow_projection_difference: stamps the CRS onto the
#        output AND stops gdalbuildvrt skipping the ~45% of tiles that ship no CRS
#        (see header QUIRK 2 — -a_srs alone would still skip them).
#      * -vrtnodata -9999: presents one clean nodata; each source keeps its own real
#        sentinel (-3.4e38 / -99 / -32767 / 0) as a per-source <NODATA>, correctly
#        masked. An input_file_list avoids argv overflow on 62k tiles.
if ($VRT | path exists) {
    print $"==> ($VRT) exists — reusing \(delete to force a rebuild\)"
} else {
    print $"==> Building national VRT ($VRT) — assign ($EPSG), unified nodata ($NODATA)"
    let idx = "_idx_hr"
    glob $"($SRC_DIR)/*.tif" | save -f $idx
    print $"  (open $idx | lines | length) tiles"
    (gdalbuildvrt -a_srs $EPSG -allow_projection_difference -vrtnodata $NODATA
      -input_file_list $idx $"($VRT).tmp" o> /dev/null)
    rm $idx
    # A handful of provider tiles (9 in this delivery) declare NO nodata at all, so
    # gdalbuildvrt emits them as <SimpleSource> (no per-source mask). Three carry
    # UNMARKED 0-value LiDAR voids — 10-35% exact-0 blobs amid 800-1000 m relief — which
    # would otherwise render as false flat patches in the hillshade and spurious 0 m
    # contour rings. Rewrite every SimpleSource -> ComplexSource with <NODATA>0> so those
    # 0-voids mask like any other sentinel. Harmless for the full-coverage no-nodata
    # tiles (they contain no 0 px). gdalbuildvrt only emits SimpleSource for no-nodata
    # sources here (no tile's nodata equals the -9999 vrtnodata), so this hits exactly
    # those 9 and nothing else.
    sed -i 's|<SimpleSource>|<ComplexSource>|; s|</SimpleSource>|      <NODATA>0</NODATA>\n    </ComplexSource>|' $"($VRT).tmp"
    mv $"($VRT).tmp" $VRT
}

# 1. Retile the national VRT with overlap (overlap is kept through processing to
#    avoid hillshade edge artifacts). PREDICTOR=1 is required — see header.
print "==> Retiling"
mkdir retiled
if ((glob retiled/*.tif | length) > 0) {
    print "  retiled/ is non-empty — reusing (delete the dir to force a re-tile)"
} else {
    (gdal_retile.py $VRT -ps $PS $PS -overlap 12 -targetDir retiled
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
            # panics on zero-variance input. A flat tile has no relief to smooth anyway.
            print $"  copy ($a)"
            cp $f $dst
        } else {
            let tmp = $"smooth/_tmp/($a)"
            print $"  smooth ($a)"
            (feature-preserving-smoothing --dem $f -o $tmp
              --filter $SM_FILTER --norm_diff $SM_NORM_DIFF
              --num_iter $SM_NUM_ITER --max_diff $SM_MAX_DIFF)
            mv $tmp $dst
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
# Final shading.tif uses JXL (lossy, distance=3.0); requires GDAL linked against a
# libtiff with libjxl support (conda geo env). PREDICTOR is intentionally omitted
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
