#!/usr/bin/env nu

# Generate contour lines for all of Spain from smoothed DTM tiles.
# Pipeline: smooth/*.tif → per-tile cropped VRTs (drop 6 px from each side
# to discard retile-overlap disagreement) → per-zone VRT → per-zone warped
# TIF in EPSG:3857 → mosaic VRT → gdal_contour to local GPKG → bulk-load
# into PostGIS via ogr2ogr.
#
# Writing to GPKG first decouples the slow vector-generation step from the
# slow PG-insert step. gdal_contour-to-PG buffers features and waits on WAL
# fsync per batch; gdal_contour-to-GPKG is just SQLite appends and runs
# disk-bound. The subsequent ogr2ogr push uses PG COPY in a single pass.
#
# Per-tile VRTs reference only the inner pixels of each smooth tile — no
# raster data is duplicated, just KB-sized XML files. Cropping is
# unconditional, which loses ~12 m at Spain's outer perimeter (mostly
# coastline / international border — irrelevant for the contour layer).
#
# Requires: smooth/ populated for zones 29, 30 AND 31.
# Run via: ~/miniforge3/bin/conda run --no-capture-output -n geo nu contours.nu

# ── Configuration ─────────────────────────────────────────────────────────────

const TARGET_SRS  = "EPSG:3857"
const TR          = "2"
const ZONES       = [29 30 31]
const CROP        = 6                         # px to drop from each tile edge


# No password here: GDAL's PG driver hands this to libpq, which picks the
# password up from $PGPASSWORD or ~/.pgpass. Keep it that way — this repo is
# public.
const PG_CONN     = "PG:host=localhost user=martin dbname=martin"
const TABLE       = "cont_es_dmr"
const HEIGHT_COL  = "height"
const INTERVAL    = 10
const GPKG        = "spain_contours.gpkg"

# ── 1a. Per-tile cropped VRT (drops 6 px from each side — no data copy) ───────

print "==> Building per-tile cropped VRTs"
mkdir smooth_vrt
(
  glob smooth/*.tif
    | where {|f|
        let stem = $f | path basename | path parse | get stem
        not ($"smooth_vrt/($stem).vrt" | path exists)
      }
    | par-each -t 24 {|src|
        let stem = $src | path basename | path parse | get stem
        let dst  = $"smooth_vrt/($stem).vrt"
        let info = gdalinfo -json $src | from json
        let w = $info.size.0
        let h = $info.size.1
        (gdal_translate -of VRT
          -srcwin $CROP $CROP ($w - 2 * $CROP) ($h - 2 * $CROP)
          $src $dst o> /dev/null)
      }
)

# ── 1b. Per-zone VRT from the cropped per-tile VRTs ───────────────────────────

print "==> Building per-zone VRTs"
for zone in $ZONES {
    let vrt = $"smooth_zone($zone).vrt"
    if ($vrt | path exists) {
        print $"  skip ($vrt)"
        continue
    }
    let idx = $"_idx_smooth_($zone)"
    glob $"smooth_vrt/zone($zone)_*.vrt" | save -f $idx
    let n = open $idx | lines | length
    print $"  zone ($zone): ($n) tiles"
    gdalbuildvrt -input_file_list $idx $vrt o> /dev/null
    rm $idx
}

# ── 2. Warp each zone VRT to EPSG:3857 (3 zones in parallel) ──────────────────
# Each gdalwarp uses -multi + ALL_CPUS internally, so 3-way outer parallelism
# is plenty. Resumable: skips zones whose output already exists.

print "==> Warping per-zone VRTs to EPSG:3857"
(
  $ZONES
    | where {|z| not ($"warped_zone($z).tif" | path exists)}
    | par-each -t 3 {|zone|
        let src = $"smooth_zone($zone).vrt"
        let dst = $"warped_zone($zone).tif"
        let tmp = $"warped_zone($zone).tif.tmp"
        print $"  warp zone ($zone)"
        (gdalwarp
          -t_srs $TARGET_SRS -tr $TR $TR -tap -r bilinear
          -of GTiff
          -co COMPRESS=ZSTD -co PREDICTOR=2 -co TILED=YES
          -co NUM_THREADS=ALL_CPUS -co BIGTIFF=YES
          -multi -wo NUM_THREADS=ALL_CPUS
          $src $tmp
          o> /dev/null)
        mv $tmp $dst
        print $"  done zone ($zone)"
      }
)

# ── 3. Mosaic the three warped rasters into a single VRT ──────────────────────

print "==> Mosaicking warped zones"
glob warped_zone*.tif | save -f _idx_final
gdalbuildvrt -input_file_list _idx_final spain.vrt o> /dev/null
rm _idx_final

# ── 4. gdal_contour → local GPKG ──────────────────────────────────────────────
# Single-threaded; expect many hours over continental Spain.
# Writing to local SQLite (GPKG) is disk-bound, no PG/WAL overhead.
# Resumable check: if the GPKG already exists, skip — delete it to re-run.

if ($GPKG | path exists) {
    print $"==> Skipping contour generation; ($GPKG) already exists"
} else {
    print $"==> Generating contours → ($GPKG) — this will take hours"
    let tmp = $"($GPKG | str replace '.gpkg' '.tmp.gpkg')"
    (gdal_contour
      -f GPKG
      -nln $TABLE
      -i $INTERVAL
      -a $HEIGHT_COL
      -lco SPATIAL_INDEX=NO
      spain.vrt $tmp)
    mv $tmp $GPKG
}

# ── 5. Bulk-load GPKG into PostGIS via ogr2ogr COPY ───────────────────────────
# -overwrite drops the existing table at start; safe for re-runs.

print $"==> Loading ($GPKG) into PostGIS table ($TABLE)"
(ogr2ogr
  --config PG_USE_COPY YES
  -f PostgreSQL
  -nln $TABLE
  -overwrite
  -lco SPATIAL_INDEX=NONE
  -lco GEOMETRY_NAME=wkb_geometry
  $PG_CONN $GPKG)

# ── 6. Post-load indexes ──────────────────────────────────────────────────────

print "==> Creating spatial + height indexes"
let q = $"
  CREATE INDEX IF NOT EXISTS ($TABLE)_geom_gix ON ($TABLE) USING GIST \(wkb_geometry\);
  CREATE INDEX IF NOT EXISTS ($TABLE)_height_idx ON ($TABLE) \(($HEIGHT_COL)\);
  VACUUM ANALYZE ($TABLE);
"
$q | psql -h localhost -U martin -d martin

print "==> Done"
