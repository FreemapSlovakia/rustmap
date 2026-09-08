#!/usr/bin/env python3
"""Distinguish state-border nodata from genuine interior voids in the NRW DGM1.

gdal_fillnodata only matters for voids INSIDE the delivered footprint. Nodata
that is connected to the edge of the sampled window is almost always just the
state boundary (the VRT bbox is a rectangle; NRW is not), and filling it would
invent terrain outside the state. So: flood-fill nodata from the window border,
and report only what is left -- those are real interior holes.
"""
import random
import numpy as np
from osgeo import gdal

gdal.UseExceptions()

VRT = "/run/media/martin/2190983A5767510F/DGM1/North Rhine-Westphalia/all.vrt"
WIN = 1200
N = 60
NODATA = -9999.0

random.seed(20260908)  # same windows as nw_nodata.py

ds = gdal.Open(VRT)
band = ds.GetRasterBand(1)
gt = ds.GetGeoTransform()
W, H = ds.RasterXSize, ds.RasterYSize


def edge_connected(mask):
    """Binary propagation from the window border, 4-connected."""
    reach = np.zeros_like(mask)
    reach[0, :] = mask[0, :]
    reach[-1, :] = mask[-1, :]
    reach[:, 0] = mask[:, 0]
    reach[:, -1] = mask[:, -1]
    while True:
        grow = reach.copy()
        grow[1:, :] |= reach[:-1, :]
        grow[:-1, :] |= reach[1:, :]
        grow[:, 1:] |= reach[:, :-1]
        grow[:, :-1] |= reach[:, 1:]
        grow &= mask
        if np.array_equal(grow, reach):
            return reach
        reach = grow


tot_px = tot_interior = 0
rows = []

for _ in range(N):
    x = random.randint(0, W - WIN - 1)
    y = random.randint(0, H - WIN - 1)
    a = band.ReadAsArray(x, y, WIN, WIN)
    if a is None:
        continue
    mask = (a == NODATA)
    nod = int(mask.sum())
    if nod / mask.size > 0.5:
        continue  # outside the footprint
    tot_px += mask.size
    if nod == 0:
        continue
    border = int(edge_connected(mask).sum())
    interior = nod - border
    tot_interior += interior
    ex = gt[0] + (x + WIN / 2) * gt[1]
    ny = gt[3] + (y + WIN / 2) * gt[5]
    rows.append((int(ex), int(ny), nod, border, interior))

print(f"inland sampled: {tot_px/1e6:.1f} Mpx")
print(f"interior (non-border) nodata: {tot_interior} px "
      f"= {100.0*tot_interior/tot_px:.4f}%")
print()
print("  easting  northing   nodata   border  interior")
for r in sorted(rows, key=lambda t: -t[4]):
    print(f"  {r[0]:8d} {r[1]:9d} {r[2]:8d} {r[3]:8d} {r[4]:9d}")
