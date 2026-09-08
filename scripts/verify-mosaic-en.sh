#!/usr/bin/env bash
# End-to-end check: does the finished mosaic actually have data everywhere the
# window tiles say it should?
#
# WHY THIS EXISTS. Three separate silent holes reached a "finished" mosaic during
# the England build, each by a different mechanism:
#   NU2020-00  3 bands            -> gdalbuildvrt skipped it
#   SU5520-11  all-Undefined ci   -> gdalbuildvrt skipped it
#   SU5555-00  transient gdalinfo -> wrongly marked .empty, never rendered
# Each was caught by a check aimed at that specific mechanism, and the third only
# because a stale smooth2m tile happened to disprove the .empty marker. Checking
# the steps is whack-a-mole; this checks the artifact being shipped, so it is
# blind to the mechanism and catches causes not yet encountered.
#
# Method: sample the centre of every non-empty window tile, read the mosaic's
# mask there, and report any point where the finished raster has no data.
#
# Usage: ./verify-mosaic-en.sh [samples]   (default: every tile)
set -u
TIF=/media/martin/18TB/en/shading.tif
TILES=/mnt/osm/en/tiles
PY=$HOME/miniforge3/envs/geo/bin/python
N="${1:-0}"

[ -f "$TIF" ] || { echo "no $TIF yet"; exit 1; }

"$PY" - "$TIF" "$TILES" "$N" <<'PYEOF'
import glob, os, random, sys
import numpy as np
from osgeo import gdal
gdal.UseExceptions()
tif, tiles, n = sys.argv[1], sys.argv[2], int(sys.argv[3])

ds   = gdal.Open(tif)
inv  = gdal.InvGeoTransform(ds.GetGeoTransform())
mmask = ds.GetRasterBand(1).GetMaskBand()

files = sorted(glob.glob(os.path.join(tiles, "*.tif")))
if n and n < len(files):
    random.seed(0); files = random.sample(files, n)
print(f"checking {len(files)} tiles against {os.path.basename(tif)}")

# A tile centre is NOT a valid probe: coastal tiles are legitimately nodata in
# the middle. Probe a pixel the TILE itself marks as valid, then demand the
# mosaic have data at that same ground position.
holes, skipped = [], 0
for i, f in enumerate(files):
    t = gdal.Open(f)
    tgt = t.GetGeoTransform()
    sub = t.GetRasterBand(1).GetMaskBand().ReadAsArray(
        buf_xsize=min(64, t.RasterXSize), buf_ysize=min(64, t.RasterYSize))
    if sub is None or not sub.any():
        skipped += 1                      # tile is entirely nodata; nothing to assert
        continue
    ry, rx = np.argwhere(sub > 0)[len(np.argwhere(sub > 0)) // 2]
    fx = (rx + 0.5) / sub.shape[1] * t.RasterXSize
    fy = (ry + 0.5) / sub.shape[0] * t.RasterYSize
    gx = tgt[0] + tgt[1] * fx
    gy = tgt[3] + tgt[5] * fy
    px, py = gdal.ApplyGeoTransform(inv, gx, gy)
    px, py = int(px), int(py)
    if not (0 <= px < ds.RasterXSize and 0 <= py < ds.RasterYSize):
        holes.append((os.path.basename(f), "outside mosaic")); continue
    v = mmask.ReadAsArray(px, py, 1, 1)
    if v is None or int(v[0][0]) == 0:
        holes.append((os.path.basename(f), "tile has data here, mosaic does not"))
    if (i + 1) % 5000 == 0:
        print(f"  {i+1}/{len(files)}")

print(f"  ({skipped} tiles were entirely nodata — nothing to assert)")
if holes:
    print(f"\nFAIL: {len(holes)} tile(s) missing from the mosaic:")
    for name, why in holes[:20]:
        print(f"  {name}: {why}")
    sys.exit(1)
print(f"\nOK: every sampled tile's data is present in the mosaic")
PYEOF

# LIMITATION, stated so nobody trusts this further than it goes: this compares
# the mosaic against the TILES. A window wrongly marked .empty has no tile, so it
# cannot be sampled and this check cannot see it (that was SU5555-00). That class
# is closed upstream instead — has-data now throws on a gdalinfo failure rather
# than writing a permanent .empty.
