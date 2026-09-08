# England hillshading + contours — where it stands

Stopped cleanly 2026-08-14 23:0x for a reboot. Everything below resumes losslessly.

## Resume with one command

```bash
setsid nohup ~/fm/freemap-outdoor-map/watch-pipeline-en.sh \
  >/mnt/osm/en/watch-pipeline.log 2>&1 < /dev/null &

# optional, unrelated to England — Romania's source is still offline:
setsid nohup ~/fm/freemap-outdoor-map/watch-ancpi.sh --run 600 \
  >/media/martin/18TB/ro/watch-ancpi-stdout.log 2>&1 < /dev/null &
```

The supervisor re-runs `download-en.nu` (which will confirm the delivery in one
verification pass) and then starts `shading-en.nu` on its own.

Watch it:

```bash
tail -30 /mnt/osm/en/watch-pipeline.log
ls /media/martin/18TB/en/DTM1/*.tif | wc -l    # stage 1, of 5876
ls /media/martin/18TB/en/tiles/ | wc -l        # stage 2, of 23504
```

## State at shutdown

| | |
|---|---|
| source tiles | **5876 / 5876, 330 GB** — download COMPLETE |
| verification | interrupted mid-sweep; the next pass redoes it (read-only, ~340 GB) |
| shading tiles | 0 / 23504 — not started |
| contours | not started |
| scratch `_work/` | empty |
| free space | 1.8 TB on the 18TB, 398 GB on the NVMe |

The only thing outstanding for stage 1 is the final `gdalinfo -checksum` sweep,
which was running when we stopped. It is read-only and idempotent.

## Decisions already made, with the evidence

* **Product**: `lidar_composite_dtm` 2022 @ 1 m, not `national_lidar_programme_dtm`.
  The composite is one product/year/resolution key nationwide, so every tile has a
  deterministic URL and coverage is 100% by construction; NLP returns 1-4 survey
  years per tile and can have holes. Switching later is a config change in
  `build-index-en.py` only.

* **ZOOM = 16**, measured not inherited. `sample-zoom-en.nu` rendered Scafell Pike
  (NY2005, 134-978 m — the roughest ground in England) at z15/16/17 through the
  production pipeline and differenced them over 13.5 M pixels:

      z15 -> z17   mean 6.12/255   p95 23.0   31.2% of pixels off by >5
      z16 -> z17   mean 1.32/255   p95  4.0    3.2% of pixels off by >5

  z16 is indistinguishable from z17; z15 is visibly lossy. 44 GB vs 137 GB.
  Comparison page: https://claude.ai/code/artifact/1a902230-a982-4872-9b66-aee6c2954862

* **Contours are deliberately NOT chained** after shading. `contours-en.nu`
  consolidates a few hundred GB and then runs `gdal_contour` for hours; it is a
  separate call, and the shading is worth looking at first. The supervisor logs
  the command when shading finishes.

## Two hazards, both already handled in code — do not undo

1. **nodata is `-3.4028235e+38`** (Float32 lowest), and genuine English elevations
   go BELOW zero — a Devon tile bottoms out at -6.06 m of real terrain. Never
   threshold on "very negative" to find voids; only the declared sentinel counts.
   `shading-en.nu` unifies it to -9999 via `gdalbuildvrt -vrtnodata`.

2. **`/media/martin/18TB` is ntfs3.** Three appenders on one log file there wedged
   bash in `ntfs_file_write_iter` — D state, deaf to SIGKILL, inode locked until
   reboot. The supervisor log therefore lives on btrfs (`/mnt/osm/en/`) with
   exactly one writer. Do not add a `tee` back into `log()`.

## Leftover to clear by rebooting

PID 959290 (`watch-pipeline-en.sh`) is stuck unkillable in `D` state holding the
inode of the abandoned `/media/martin/18TB/en/watch-pipeline.log`. Harmless — no
children, hung before starting any work — but it will likely make **suspend fail**
("Freezing of tasks failed"), which is why a reboot was preferred. Do not tail or
delete that file; the live log is `/mnt/osm/en/watch-pipeline.log`.

## Romania, parked

ANCPI was taken offline after a security incident (infrastructure isolated
following DNSC intervention, e-Terra restarted 11-12 Aug 2026, other public
platforms still down with no date). `geoportal.ancpi.ro` is NXDOMAIN from its own
authoritative nameserver. `download-ro.nu` and the cached 26959-tile LAKI III
index are ready; `watch-ancpi.sh` fetches one probe tile and stops the moment the
host answers. See `/media/martin/18TB/ro/index/README.md`.

## Known limitation: ~1.9 m datum offset (accepted 2026-08-17)

`shading.tif` and `contours_en` sit about **1.9 m off true WGS84** (median; p95
3.57 m, max 4.67 m). EPSG:27700 is on OSGB36, and the correct datum shift is
OSTN15 — OS's official NTv2 grid, ~0.1 m. It was not installed when these were
built, and **PROJ does not error in that case**; it silently falls back to a
7-parameter Helmert with 2 m stated accuracy.

That is 1–3 pixels at z16 (1.34–1.54 ground m/px), it varies spatially so no
constant offset corrects it, and it shows as misregistration against OSM.
Both products carry it, so they at least agree with each other.

Unaffected: the raw DTM tiles and `england_contours.gpkg` — both native
EPSG:27700. Only reprojected output is wrong.

**Do not partially re-render.** The grid now exists at
`~/.local/share/proj/uk_os_OSTN15_NTv2_OSGBtoETRS.tif` and the geo env's PROJ
picks it up automatically, so re-rendering a few windows into the existing
`tiles/` would place them ~1.9 m from their neighbours and leave a seam. Rebuild
everything or nothing.

**OSTN15 is installed box-wide** since 2026-08-17
(`/usr/share/proj/uk_os_OSTN15_NTv2_OSGBtoETRS.tif`), verified to match pyproj
digit-for-digit from PostGIS. The machine is therefore on OSTN15 while these two
products are not, so the hazard applies to contours as well: re-running the
splitter alone would put `contours_en` ~1.9 m from `shading.tif` and they would
stop agreeing with each other.

To fix on a full rebuild (~17 h shading + ~2 h splitter) there is no setup left —
delete `tiles/` and `shading.tif`, re-run `shading-en.nu`, then re-run the
splitter from the (unaffected) EPSG:27700 GPKG. Both pick up OSTN15 by themselves.

When computing a 4326 bbox after that: OSTN15 covers only the GB landmass, so
densifying along the BNG rectangle edges returns `inf` outside its coverage —
clamp to the grid extent or densify over the data footprint instead.

## Deployment steps still outstanding

1. `contours_en` is loaded and live in PostGIS (2,033,201 rows, 2,480 MB).
2. Place the raster: mount the 14TB, then `shading.tif` -> `<MAPRENDER_HILLSHADING_BASE_PATH>/en/final.tif`.
3. Add `en` to `MAPRENDER_HILLSHADING_HIERARCHY` and `MAPRENDER_CONTOUR_COUNTRIES` in `.env`.

Until 3, the renderer does not read `contours_en`, so nothing on the live map changes.

## Transport artefacts

* `fm6:/fm/storage2/dtm/en/` — 5876 raw tiles, 330 GB, byte-exact, plus a
  relative-path `en.vrt` verified to resolve there.
* `/media/martin/18TB/en/contours_en.dump` — 2.01 GB, custom format.
  Restore with `pg_restore --no-owner --no-privileges --no-tablespaces -d <db> -j 4`.
  **`--no-tablespaces` on the RESTORE is what matters** — with a custom-format
  archive `pg_dump --no-tablespaces` does not strip it; the tablespace `osm_ext`
  is stored in the archive and omitted at restore time. Without the flag the
  restore fails with "tablespace osm_ext does not exist".

## Licensing

Environment Agency LIDAR Composite DTM 2022, 1 m, England. Open Government
Licence v3.0. Required attribution, verbatim:

> © Environment Agency copyright and/or database right 2022. All rights reserved.

The composite is a gap-filled mosaic of surveys flown 1998–2022; the 2022 is the
composite's publication right, not the survey date.
