# RustMap

Attempt to reimplement https://github.com/FreemapSlovakia/freemap-mapnik into Rust, helping mapnik to rest in peace.

## Why?

- Mapnik is no more actively developed except for keeping it to build itself with tools of the recent versions.
- Better control of the rendering
- Improve resource demands (CPU, memory)
- I want to improve my Rust proficiency

## Technical details

- uses the same PostGIS schema as freemap-mapnik
- uses Cairo for rendering
- uses GDAL to read from GeoTIFFs

## Running

Install Rust and run:

```bash
cargo run
```

TMS URL is then `http://localhost:3050/{zoom}/{x}/{y}@2x[|.png|.svg]` (adjust your scaling).

## Land polygons

```sh
wget https://osmdata.openstreetmap.de/download/land-polygons-complete-3857.zip
unzip land-polygons-complete-3857.zip
ogr2ogr \
  -f PostgreSQL \
  PG:"host=localhost dbname=martin user=martin password=b0n0" \
  land-polygons-complete-3857 \
  -nln land_polygons_raw \
  -lco GEOMETRY_NAME=geom \
  -lco FID=osm_id \
  -lco SPATIAL_INDEX=GIST \
  -t_srs EPSG:3857 \
  -nlt PROMOTE_TO_MULTI \
  -overwrite
```
