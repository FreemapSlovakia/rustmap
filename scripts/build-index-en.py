#!/usr/bin/env python3
"""Enumerate England's LIDAR Composite DTM 1 m tiles from the EA survey API.

Writes /media/martin/18TB/en/index/composite_dtm_1m_tiles.json, which
download-en.nu consumes. Re-run to refresh (e.g. after the EA republishes);
the output is deterministic apart from tile ordering.

WHY A SWEEP AND NOT ONE CALL. The Environment Agency's survey search takes a
GeoJSON polygon and returns the tiles under it, but the UI — and, it turns out,
the backend — refuses a selection of 48 or more tiles. So England is swept with
a grid of 20 km cells in EPSG:27700, each covering 16 tiles of 5 km, comfortably
under the cap. 957 cells; the ones that are entirely sea come back empty and
cost a single request.

    POST https://environment.data.gov.uk/backend/catalog/api/tiles/collections/survey/search
    Content-Type: application/geo+json          <- application/json is rejected 415

Each result carries product/year/resolution/tile, from which the download URL is
built. `subscription-key=dspui` is not a credential: it is hardcoded in the
site's own JavaScript bundle as the public UI key.

Licence is OGL v3 — free commercial use with attribution to the Environment
Agency. Verified 2026-08-14: 5876 tiles, all 957 cells resolved.

Failed cells are retried in later passes rather than silently dropped — a
missing cell would leave a 20 km hole in the country that nothing downstream
would notice.
"""
import json
import os
import sys
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor

from pyproj import Transformer

API = ("https://environment.data.gov.uk/backend/catalog/api"
       "/tiles/collections/survey/search")
DL = "https://environment.data.gov.uk/tiles/collections/survey"

PRODUCT = "lidar_composite_dtm"
YEAR = "2022"
RESOLUTION = "1"

OUT_DIR = "/media/martin/18TB/en/index"
OUT = os.path.join(OUT_DIR, "composite_dtm_1m_tiles.json")

STEP = 20_000                 # 20 km -> 16 tiles per query; the cap is 48
E0, E1 = 80_000, 660_000      # British National Grid extent of England,
N0, N1 = 0, 660_000           # generous on every side
WORKERS = 6                   # polite to a public government API
PASSES = 4                    # retry passes over cells that errored

to_wgs = Transformer.from_crs("EPSG:27700", "EPSG:4326", always_xy=True)


def cell_polygon(e, n):
    pts = [(e, n), (e + STEP, n), (e + STEP, n + STEP), (e, n + STEP), (e, n)]
    ll = [to_wgs.transform(x, y) for x, y in pts]
    return {"type": "Polygon",
            "coordinates": [[[round(x, 6), round(y, 6)] for x, y in ll]]}


def search(poly, attempts=3):
    body = json.dumps(poly).encode()
    for a in range(attempts):
        try:
            req = urllib.request.Request(
                API, data=body, headers={"Content-Type": "application/geo+json"})
            with urllib.request.urlopen(req, timeout=120) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            # A too-large selection will never succeed by retrying — surface it
            # so STEP can be reduced, rather than burning attempts.
            if e.code in (400, 413):
                return {"_oversize": f"HTTP {e.code}"}
            time.sleep(2 * (a + 1))
        except Exception:
            time.sleep(2 * (a + 1))
    return None


def sweep(cells):
    """Return (tiles_by_id, cells_that_failed, cells_that_were_oversize)."""
    tiles, failed, oversize = {}, [], []

    def work(cell):
        return cell, search(cell_polygon(*cell))

    with ThreadPoolExecutor(max_workers=WORKERS) as ex:
        for k, (cell, d) in enumerate(ex.map(work, cells)):
            if d is None:
                failed.append(cell)
            elif "_oversize" in d:
                oversize.append([cell, d["_oversize"]])
            else:
                for r in d.get("results", []):
                    if (r["product"]["id"] == PRODUCT
                            and r["year"]["id"] == YEAR
                            and r["resolution"]["id"] == RESOLUTION):
                        tid = r["tile"]["id"]
                        tiles[tid] = {
                            "tile": tid,
                            "label": r["tile"]["label"],
                            "url": (f"{DL}/{PRODUCT}/{YEAR}/{RESOLUTION}/{tid}"
                                    f"?subscription-key=dspui"),
                        }
            if (k + 1) % 100 == 0:
                print(f"  {k + 1}/{len(cells)} cells, {len(tiles)} tiles",
                      file=sys.stderr)

    return tiles, failed, oversize


def main():
    os.makedirs(OUT_DIR, exist_ok=True)

    cells = [(e, n)
             for e in range(E0, E1, STEP)
             for n in range(N0, N1, STEP)]
    print(f"sweeping {len(cells)} cells of {STEP // 1000} km", file=sys.stderr)

    tiles, pending, oversize = sweep(cells)

    for p in range(2, PASSES + 1):
        if not pending:
            break
        print(f"pass {p}: retrying {len(pending)} failed cell(s)", file=sys.stderr)
        more, pending, over2 = sweep(pending)
        tiles.update(more)
        oversize += over2

    out = {
        "product": PRODUCT,
        "year": YEAR,
        "resolution": RESOLUTION,
        "tiles": sorted(tiles.values(), key=lambda t: t["tile"]),
    }

    # A partial index is worse than none: it looks complete to download-en.nu
    # and would leave holes nothing downstream can detect.
    if pending or oversize:
        print(f"\n!! {len(pending)} cell(s) never answered, "
              f"{len(oversize)} were oversize — index is INCOMPLETE",
              file=sys.stderr)
        json.dump({"failed": pending, "oversize": oversize},
                  open(os.path.join(OUT_DIR, "index_gaps.json"), "w"), indent=1)
        print("   see index_gaps.json; re-run, or reduce STEP for oversize cells",
              file=sys.stderr)
        sys.exit(1)

    json.dump(out, open(OUT, "w"), indent=1)
    gaps = os.path.join(OUT_DIR, "index_gaps.json")
    if os.path.exists(gaps):
        os.remove(gaps)

    print(f"\n{len(out['tiles'])} tiles -> {OUT}", file=sys.stderr)


if __name__ == "__main__":
    main()
