#!/usr/bin/env python3
"""Repair 2x-replicated (pixel-doubled) patches in a DEM tile.

Italy's national 5 m HRDTM is a mosaic: some regions (the Dolomites and much of
the high Alps) are really 10 m data nearest-neighbour-upsampled into the 5 m grid,
so every 2x2 block of "5 m" pixels holds one real value. That produces staircase
contours and terraced hillshading. This tool detects those patches and, only there,
reconstructs the true 10 m surface (2x2 average -> cubic upsample), feather-blended
back into the untouched real-5 m LiDAR.

Detection keys on EXACT pixel equality, which real LiDAR essentially never produces:
  z1 = local fraction of lag-1 (5 m) neighbours exactly equal   ~0.5 in replicated
  z2 = local fraction of lag-2 (10 m) neighbours exactly equal   ~0.0 in replicated
Flag where z1 high AND z2 low. The z2 test rejects genuinely flat water/fields
(constant at every scale -> z1 AND z2 both ~1), which z1 alone would false-positive.

Single-band Float32 GeoTIFF in, same geotransform/CRS/nodata out.
"""
import argparse
import numpy as np
from scipy import ndimage
from osgeo import gdal

gdal.UseExceptions()


def local_fraction(mask_bool, win):
    """Fraction of True in a win x win box around each cell (integral image)."""
    m = mask_bool.astype(np.float32)
    # uniform_filter is a separable box mean — O(pixels), gives the local fraction.
    return ndimage.uniform_filter(m, size=win, mode="nearest")


def detect_replicated(dem, valid, win, z1_thr, z2_thr):
    """Boolean mask of cells sitting in a 2x-replicated patch."""
    # Exact-equality maps, only where both members are valid data.
    eq1 = np.zeros_like(dem, dtype=bool)
    eq1[:, :-1] |= (dem[:, 1:] == dem[:, :-1]) & valid[:, 1:] & valid[:, :-1]
    eq1[:-1, :] |= (dem[1:, :] == dem[:-1, :]) & valid[1:, :] & valid[:-1, :]

    eq2 = np.zeros_like(dem, dtype=bool)
    eq2[:, :-2] |= (dem[:, 2:] == dem[:, :-2]) & valid[:, 2:] & valid[:, :-2]
    eq2[:-2, :] |= (dem[2:, :] == dem[:-2, :]) & valid[2:, :] & valid[:-2, :]

    z1 = local_fraction(eq1, win)
    z2 = local_fraction(eq2, win)
    flag = (z1 > z1_thr) & (z2 < z2_thr) & valid

    # Morphological cleanup: opening drops speckle (isolated/coincidental equal
    # blocks that squeaked past the window), closing fills pinholes so a patch is
    # one coherent region with a smooth boundary for the feather to work on.
    flag = ndimage.binary_opening(flag, iterations=2)
    flag = ndimage.binary_closing(flag, iterations=2)
    return flag & valid


def reconstruct_10m(dem):
    """True 10 m surface (2x2 block mean) cubic-upsampled back to 5 m.

    Block-mean to 10 m is phase-agnostic: on aligned duplicated blocks it returns
    the exact real value; on a half-cell-offset patch it averages two adjacent real
    10 m values, which is merely mild smoothing. Cubic upsample removes the steps.
    """
    h, w = dem.shape
    hp, wp = h + (h & 1), w + (w & 1)          # pad to even
    pad = np.pad(dem, ((0, hp - h), (0, wp - w)), mode="edge")
    coarse = pad.reshape(hp // 2, 2, wp // 2, 2).mean(axis=(1, 3))
    up = ndimage.zoom(coarse, 2, order=3)      # cubic back to 5 m
    return up[:h, :w]


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("src")
    ap.add_argument("dst")
    ap.add_argument("--win", type=int, default=24, help="detection window, px (default 24)")
    ap.add_argument("--z1", type=float, default=0.30, help="min lag-1 equal fraction (default 0.30)")
    ap.add_argument("--z2", type=float, default=0.10, help="max lag-2 equal fraction (default 0.10)")
    ap.add_argument("--feather", type=int, default=8, help="boundary blend width, px (default 8)")
    ap.add_argument("--report", action="store_true", help="print flagged-area fraction")
    args = ap.parse_args()

    ds = gdal.Open(args.src)
    band = ds.GetRasterBand(1)
    nodata = band.GetNoDataValue()
    dem = band.ReadAsArray().astype(np.float64)
    valid = np.isfinite(dem) if nodata is None else (dem != nodata) & np.isfinite(dem)

    mask = detect_replicated(dem, valid, args.win, args.z1, args.z2)

    if mask.any():
        # Feather: distance-decay weight in [0,1], 0 outside, 1 well inside the patch.
        dist = ndimage.distance_transform_edt(mask)
        w = np.clip(dist / max(args.feather, 1), 0, 1)
        w[~valid] = 0
        fill = dem.copy()
        fill[~valid] = np.nan
        # Interpolate nodata holes cheaply so the resample doesn't smear the edge.
        if (~valid).any():
            idx = ndimage.distance_transform_edt(~valid, return_distances=False,
                                                 return_indices=True)
            fill = fill[tuple(idx)]
        repaired = reconstruct_10m(fill)
        out = dem * (1 - w) + repaired * w
    else:
        out = dem

    if nodata is not None:
        out[~valid] = nodata
    out = out.astype(np.float32)

    if args.report:
        frac = 100.0 * mask.sum() / max(valid.sum(), 1)
        print(f"{args.src}: flagged {frac:.1f}% of valid pixels as 10 m-replicated")

    drv = gdal.GetDriverByName("GTiff")
    dst = drv.Create(args.dst, ds.RasterXSize, ds.RasterYSize, 1, gdal.GDT_Float32,
                     options=["COMPRESS=DEFLATE", "PREDICTOR=1", "TILED=YES"])
    dst.SetGeoTransform(ds.GetGeoTransform())
    dst.SetProjection(ds.GetProjection())
    ob = dst.GetRasterBand(1)
    if nodata is not None:
        ob.SetNoDataValue(nodata)
    ob.WriteArray(out)
    dst.FlushCache()


if __name__ == "__main__":
    main()
