# Freemap Mapserver

## Features

- Map tile serving (TMS)
- PDF output
- Configurable map pre-rendering
- On-demand tile rendering (if tile is not rendered yet)
- Detection of dirty tiles (based on changes reported by imposm3) and rendering scheduling
- Easy style development and debugging (save and reload)
- Many features are configurable

## Usage

```js
import { startMapserver } from "freemap-mapserver";
import { mapnikConfig, generateFreemapStyle } from "./style.js";
import { legend } from "./legend.js";

startMapserver(mapnikConfig, generateFreemapStyle, legend);
```

- `mapnikConfig` - stringified Mapnik XML
- `generateFreemapStyle` - function returning stringified Mapnik XML; used for PDF export. Parameters:
  - shading (bool)
  - contours (bool)
  - hikingTrails (bool)
  - bicycleTrails (bool)
  - skiTrails (bool)

## Configuration

Your app must use `node-config` library with configuration of the following structure:

```json5
{
  dirs: {
    tiles: "tiles",
    expires: "expires",
    fonts: "fonts",
  },
  server: {
    port: 4000,
  },
  workers: {
    // min: 8, commented out = use num of cpus
    // max: 8, commented out = use num of cpus
  },
  forceTileRendering: false, // useful for style development
  limits: {
    minZoom: 0,
    maxZoom: 19,
    polygon: "limits.geojson",
    scales: [1, 1.5, 2, 3],
    cleanup: true, // to delete cached tiles out of limits on startup
  },
  prerender: {
    // set to null to disable pre-rendering
    // workers: 8, commented out = use num of cpus
    minZoom: 8,
    maxZoom: 16,
    polygon: "limits.geojson",
    zoomPrio: [12, 13, 14, 15, 11, 16, 10, 9, 8],
  },
  rerenderOlderThanMs: null,
  exportMapConcurrency: 1,
  format: {
    extension: "png",
    mimeType: "image/png",
    codec: "png",
  },
  minExpiredBatchSize: 500, // batch size when deleting expired files; set to null to delete without batching,
  expiresZoom: 14, // on which zoom does imposm3 marks tiles as expired,
  prerenderMaxZoom: 14, // we need to know it even in on-demand only instance for creating index files
  // how long to delay rendering a tile when expiring tiles
  // this is to prevent drastical fi i/o slowdown when prerendering eats 100% cpu
  // set to zero to prevent this feature
  prerenderDelayWhenExpiring: 50,
}
```

## Rendering

### If `prerender` is `null`:

- On request
  - renders IF tile is missing OR tile is older than `rerenderOlderThanMs` OR `forceTileRendering`
  - caches result to `tiles` dir

### If `prerender` is NOT `null`:

- On startup all tiles out of `limits` are deleted

- On startup scans all scale-1 tiles within `prerender` limits and adds tile to _Dirty Tiles Register_ if:
  - scale-1 tile is missing
  - scale-1 tile is older than `rerenderOlderThanMs`
  - for tile exists a _dirty-file_

- On request render tile IF tile is missing OR (is older than `rerenderOlderThanMs` AND is out of `prerender` limits)

Dirty tiles marker:

- reads from `expires` dir and computes tiles of all zooms from `limits`
- every such tile out of `prerender` limits is deleted
- if tile exists then it creates its _dirty-file_ and stores it to _Dirty Tiles Register_

Prerenderer:

- loops over _Dirty Tiles Register_
- for every dirty tile pre-render it
  - TODO if _dirty-file_ render all scales else only missing scales
  - TODO
