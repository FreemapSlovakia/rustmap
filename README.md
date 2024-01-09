# RustMap

Attempt to reimplement https://github.com/FreemapSlovakia/freemap-mapnik into Rust, helping mapnik to rest in peace.

## Why?

- Mapnik is no more actively developed except for keeping it to build itself with tools of the recent versions.
- Better control of the rendering
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

TMS URL is then http://localhost:3050/{zoom}/{x}/{y}@2x (adjust your scaling)

## TODO

Almost everything.
