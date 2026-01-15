import path from "path";
import { stat } from "fs/promises";
import { dirtyTiles } from "./dirtyTilesRegister.js";
import { tile2key, tileRangeGenerator } from "./tileCalc.js";
import { config, prerenderPolygon } from "./config.js";
import { prerenderer } from "./prerenderer.js";

const extension = config.format.extension;

export async function fillDirtyTilesRegister() {
  console.log("Scanning dirty tiles.");

  if (!config.prerender) {
    throw new Error("missing prerenderConfig");
  }

  if (!prerenderPolygon) {
    throw new Error("missing prerenderPolygon");
  }

  const { minZoom, maxZoom } = config.prerender;

  for (const { zoom, x, y } of tileRangeGenerator(
    prerenderPolygon,
    minZoom,
    maxZoom
  )) {
    let mtimeMs: number;

    try {
      // find oldest
      const proms = config.limits.scales.map((scale) =>
        stat(
          path.join(
            config.dirs.tiles,
            `${zoom}/${x}/${y}${scale === 1 ? "" : `@${scale}x`}.${extension}`
          )
        )
      );

      mtimeMs = Math.min(
        ...(await Promise.all(proms)).map((stat) => stat.mtimeMs)
      );
    } catch {
      const v = { zoom, x, y, ts: 0, dt: 0 };
      dirtyTiles.set(tile2key(v), v);
      continue;
    }

    if (
      config.rerenderOlderThanMs != null &&
      mtimeMs < config.rerenderOlderThanMs
    ) {
      const v = { zoom, x, y, ts: mtimeMs, dt: 0 };
      dirtyTiles.set(tile2key(v), v);
      continue;
    }

    try {
      const { mtimeMs } = await stat(
        path.join(config.dirs.tiles, `${zoom}/${x}/${y}.dirty`)
      );

      const v = { zoom, x, y, ts: mtimeMs, dt: mtimeMs };

      dirtyTiles.set(tile2key(v), v);
    } catch {
      // fresh
    }
  }

  console.log("Dirty tiles scanned.");

  return false;
}
