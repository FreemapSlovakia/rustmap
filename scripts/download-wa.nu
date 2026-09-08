#!/usr/bin/env nu

# Fetch the Wallonia 1 m MNT (LiDAR 2021-2022) from the SPW INSPIRE Atom service.
#
# Source: Service public de Wallonie, "Relief de la Wallonie - Modèle Numérique
#   de Terrain - 1m (MNT) 2021-2022", flown 2021-2022, absolute vertical accuracy
#   ~0.12 m over the whole territory. Licence CC BY 4.0 — attribution REQUIRED,
#   unlike Luxembourg's CC0:
#
#       © SPW - Service public de Wallonie (MNT 1 m 2021-2022, CC BY 4.0)
#
#   Catalogue: https://geoportail.wallonie.be/catalogue/fe13bc84-e371-46ca-9632-8ad4139f1ee5.html
#   Atom:      https://geoservices.wallonie.be/geotraitement/spwdatadownload/results/
#                  fe13bc84-e371-46ca-9632-8ad4139f1ee5/atom_dataset.xml
#
# EPSG:3812 IS NOT A PREFERENCE, IT IS THE ONLY OPTION AT 1 m — and it is the
#   one we want anyway. 3812 is ETRS89 / Belgian Lambert 2008, so PROJ offers
#   exactly ONE operation to WGS 84 and it is a null transformation. The older
#   EPSG:31370 (Lambert 72, BD72 datum) offers THREE candidates at 1-5 m and is
#   the England/OSTN15 trap all over again: PROJ picks one silently and the
#   product ends up metres from OSM with no error anywhere. Never take the
#   31370 variant of this data, even though the 0.5 m product offers it.
#
#   Flanders' WCS also advertises 3812, so the whole country can live in one
#   ETRS89-based CRS with no datum grid and no seam at the language border.
#
# NO RESUME. Verified against the live endpoint 2026-08-27: `HEAD` returns no
#   Content-Length, and a `Range:` request is answered with 200 + the whole file
#   rather than 206. So a partial transfer cannot be continued — it restarts
#   from zero. Hence: download per PROVINCE (~10-13 GB each) rather than the
#   single ~50 GB region file, write to a .part and only rename on success, and
#   retry the whole file on failure. Measured ~3.4 MB/s, so budget about an hour
#   per province.
#
# WHY NOT THE 0.5 m PRODUCT: `--filter 11` in the smoother is a PIXEL count, so
#   on 0.5 m data it would smooth over 5.5 m rather than 11 m — a different and
#   weaker filter for the same nominal setting, breaking comparability with
#   PL/HR/NO/EN/LU. Plus 4x the bytes for detail no zoom we render can show.
#
# Resumable at province granularity: a province whose unpacked marker exists is
# skipped. Delete the marker to force a re-fetch. Run via:
#   nice ~/miniforge3/bin/conda run --no-capture-output -n geo nu ~/fm/freemap-outdoor-map/scripts/download-wa.nu

# ── Configuration ─────────────────────────────────────────────────────────────

const BASE = "https://geoservices.wallonie.be/geotraitement/spwdatadownload/results/fe13bc84-e371-46ca-9632-8ad4139f1ee5"
const PROVINCES = ["BRABANT_WALLON" "HAINAUT" "LIEGE" "LUXEMBOURG" "NAMUR"]
const RETRIES = 3

# ── Drive discovery ───────────────────────────────────────────────────────────
# udisks2 moves this removable drive between /media/martin/18TB and
# /run/media/martin/18TB (twice during 2026-08-27 alone), and the path it is NOT
# using survives as an empty root-owned directory on / — so a hardcoded stale
# path does not fail, it silently fills the root filesystem. Pick whichever is
# actually a mountpoint, and refuse to run if neither is.
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

let DRIVE    = (find-drive)
let DATA_DIR = $"($DRIVE)/be"
let ZIP_DIR  = $"($DATA_DIR)/zips"
let TIF_DIR  = $"($DATA_DIR)/MNT1M"

print $"==> drive: ($DRIVE)"

mkdir $ZIP_DIR
mkdir $TIF_DIR

# ── Fetch + unpack, one province at a time ────────────────────────────────────

for prov in $PROVINCES {
    let name = $"RELIEF_WALLONIE_MNT_1M_2021_2022_GEOTIFF_3812_PROV_($prov).zip"
    let url  = $"($BASE)/($name)"
    let zip  = $"($ZIP_DIR)/($name)"
    let done = $"($TIF_DIR)/.($prov).unpacked"

    if ($done | path exists) {
        print $"==> ($prov): already unpacked — skipping"
        continue
    }

    if not ($zip | path exists) {
        mut ok = false
        for attempt in 1..$RETRIES {
            print $"==> ($prov): downloading \(attempt ($attempt)/($RETRIES)\) — no resume, restarts on failure"
            let part = $"($zip).part"
            rm -f $part
            # --fail so an HTML error page is never mistaken for data.
            let res = (do { curl --fail --location --silent --show-error --output $part $url } | complete)
            if $res.exit_code == 0 {
                # A truncated transfer still leaves a file; the zip test below is
                # what actually proves it arrived whole.
                let t = (do { unzip -t $part } | complete)
                if $t.exit_code == 0 {
                    mv $part $zip
                    $ok = true
                    break
                } else {
                    print $"    corrupt archive — discarding and retrying"
                    rm -f $part
                }
            } else {
                print $"    curl failed: ($res.stderr | str trim)"
                rm -f $part
            }
        }
        if not $ok {
            error make {msg: $"($prov): download failed after ($RETRIES) attempts"}
        }
    } else {
        print $"==> ($prov): zip already present — verifying"
        let t = (do { unzip -t $zip } | complete)
        if $t.exit_code != 0 {
            rm -f $zip
            error make {msg: $"($prov): existing zip is corrupt and has been deleted — re-run to fetch it again"}
        }
    }

    # EVERY PROVINCE SHIPS THE SAME FILENAMES. All five archives contain
    # RELIEF_WALLONIE_MNT_1M_2021_2022.tif plus identically-named sidecars, so
    # unpacking them into one directory silently overwrites each province with
    # the next. (Caught 2026-08-27 with Brabant Wallon on disk and Hainaut
    # mid-download.) Unpack to a scratch dir, then move the raster out under the
    # province's own name.
    print $"==> ($prov): unpacking"
    let scratch = $"($TIF_DIR)/_unpack_($prov)"
    rm -rf $scratch
    mkdir $scratch
    unzip -q -o $zip -d $scratch

    let raw = (glob $"($scratch)/**/*.tif" | where {|f| not ($f | str ends-with ".aux.xml")} | first)
    if ($raw | is-empty) {
        rm -rf $scratch
        error make {msg: $"($prov): no .tif found inside the archive"}
    }

    # RE-TILE ON THE WAY IN. The delivered rasters are STRIPED — one block is
    # 65604 x 16, i.e. the full province width. Cutting a 2.5 km window out of
    # that forces GDAL to inflate full-width LZW strips for every window, across
    # thousands of windows. Converting once to 512x512 tiles turns each window
    # read into a handful of blocks. Costs one pass per province now, saves the
    # whole shading run later.
    print $"==> ($prov): re-tiling \(delivered striped 65604x16\)"
    let out = $"($TIF_DIR)/($prov).tif"
    (gdal_translate -q -of GTiff
      -co COMPRESS=ZSTD -co ZSTD_LEVEL=1 -co PREDICTOR=3
      -co TILED=YES -co BLOCKXSIZE=512 -co BLOCKYSIZE=512
      -co BIGTIFF=YES -co NUM_THREADS=ALL_CPUS
      $raw $"($out).tmp" o> /dev/null)
    mv $"($out).tmp" $out
    rm -rf $scratch

    touch $done
    print $"==> ($prov): done -> ($out)"
}

let tifs = (glob $"($TIF_DIR)/**/*.tif" | length)
print ""
print $"==> ($tifs) GeoTIFF tiles in ($TIF_DIR)"
print $"==> zips kept in ($ZIP_DIR) — delete once you are happy \(they are ~50 GB\)"
print $"==> next: nu ~/fm/freemap-outdoor-map/shading-be.nu"
