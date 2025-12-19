import path from "path";
import { rename, mkdir, unlink, stat, writeFile, open } from "fs/promises";
import { flock } from "fs-ext";
import { promisify } from "util";
import {
  bbox4326To3857,
  tile2key,
  tile2bbox3859,
  tileOverlapsLimits,
} from "./tileCalc.js";
import { dirtyTiles } from "./dirtyTilesRegister.js";
import { pool } from "./renderedPool.js";
import { Tile } from "./types.js";
import { ImageFormat } from "maprender-node";
import { prerenderer } from "./prerenderer.js";
import { config } from "./config.js";

const flockAsync = promisify(
  flock as (
    fd: number,
    flags: "sh" | "ex" | "shnb" | "exnb" | "un",
    callback: (err: NodeJS.ErrnoException | null) => void
  ) => void
);

const rerenderOlderThanMs = config.rerenderOlderThanMs;

let tilesDir = config.dirs.tiles;

const extension = config.format.extension;

let cnt = 0;

// TODO if out of prerender area and reqScale is provided then render only that scale
export async function renderTile(
  zoom: number,
  x: number,
  y: number,
  reqScale?: number
): Promise<string | undefined> {
  const frags = [tilesDir, zoom.toString(10), x.toString(10)];

  const pathnameBase = path.join(...frags, y.toString(10));

  const reasons: string[] = [];

  if (config.forceTileRendering) {
    reasons.push("forced");
  } else if (!reqScale) {
    reasons.push("noReqScale");
  } else {
    await shouldRender(pathnameBase, { zoom, x, y, reqScale }, reasons);
  }

  if (reasons.length) {
    await mkdir(path.join(...frags), { recursive: true });

    await renderScales(
      pathnameBase,
      zoom,
      x,
      y,
      reqScale ? [reqScale] : config.limits.scales,
      !reqScale,
      reasons
    );

    if (!reqScale) {
      try {
        await unlink(pathnameBase + ".dirty");
      } catch (_) {
        // ignore
      }

      dirtyTiles.delete(tile2key({ zoom, x, y }));
    }
  }

  return reqScale
    ? `${pathnameBase}${reqScale === 1 ? "" : `@${reqScale}x`}.${extension}`
    : undefined;
}

let coolDownPromise: Promise<void> | null;

function toScaleSpec(scale: number) {
  return scale === 1 ? "" : `@${scale}x`;
}

async function renderScales(
  pathnameBase: string,
  zoom: number,
  x: number,
  y: number,
  scales: number[],
  prerender: boolean,
  reasons: string[]
) {
  const prerenderDelayWhenExpiring = config.prerenderDelayWhenExpiring;

  if (
    prerender &&
    global.processingExpiredTiles &&
    prerenderDelayWhenExpiring
  ) {
    if (coolDownPromise) {
      await coolDownPromise;
    } else {
      coolDownPromise = new Promise<void>((resolve) => {
        setTimeout(() => {
          coolDownPromise = null;
          resolve();
        }, prerenderDelayWhenExpiring);
      });
    }
  }

  const logPrefix =
    (prerender ? "Pre-rendering" : "Rendering") + ` tile ${zoom}/${x}/${y}: `;

  let dirtyTile;

  if (prerender) {
    dirtyTile = dirtyTiles.get(tile2key({ zoom, x, y }));

    if (!dirtyTile) {
      console.warn(`${logPrefix}no dirty meta found`);

      return;
    }
  }

  const scales2: number[] = [];

  if (dirtyTile) {
    reasons.push("dirty");

    for (const scale of scales) {
      const scaleSpec = toScaleSpec(scale);

      try {
        const { mtimeMs } = await stat(
          `${pathnameBase}${scaleSpec}.${extension}`
        );

        if (
          mtimeMs > dirtyTile.dt &&
          (rerenderOlderThanMs == null || mtimeMs > rerenderOlderThanMs)
        ) {
          console.log(`${logPrefix}fresh`);

          continue;
        }
      } catch {
        // nothing
      }

      scales2.push(scale);
    }
  } else {
    scales2.push(...scales); // NOTE should be only one - on demand
  }

  console.log(`${logPrefix}rendering`, reasons);

  const renderer = await pool.acquire(prerender ? 1 : 0);

  let buffers: Buffer[];

  let t: number;

  try {
    t = Date.now();

    const result = await renderer.render(
      tile2bbox3859(x, y, zoom),
      zoom,
      scales2,
      (
        {
          jpg: "Jpeg",
          jpeg: "Jpeg",
          png: "Png",
          svg: "Svg",
          pdf: "Pdf",
        } as Record<string, ImageFormat>
      )[extension] ?? ("Jpeg" as ImageFormat)
    );

    buffers = result.images;

    measure("render", Date.now() - t);
  } finally {
    pool.release(renderer);
    // TODO release image pool on error
  }

  const tmpNames: [string, string][] = [];

  for (let i = 0; i < scales2.length; i++) {
    const scale = scales2[i];

    const tmpName = `${pathnameBase}${toScaleSpec(scale)}_${cnt++}_tmp.${extension}`;

    t = Date.now();

    await writeFile(tmpName, buffers[i]);

    tmpNames.push([
      tmpName,
      `${pathnameBase}${toScaleSpec(scale)}.${extension}`,
    ]);
  }

  if (zoom > config.prerenderMaxZoom) {
    const expiresZoom = config.expiresZoom;

    const div = 2 ** (zoom - expiresZoom);

    await mkdir(
      path.resolve(tilesDir, String(expiresZoom), String(Math.floor(x / div))),
      { recursive: true }
    );

    const fh = await open(
      path.resolve(
        tilesDir,
        String(expiresZoom),
        String(Math.floor(x / div)),
        Math.floor(y / div) + ".index"
      ),
      "a"
    );

    fh.write(
      scales2
        .map((scale) => `${zoom}/${x}/${y}${toScaleSpec(scale)}\n`)
        .join("")
    );

    await flockAsync(fh.fd, "sh");

    await fh.close();
  }

  await Promise.all(tmpNames.map(([from, to]) => rename(from, to)));

  measure("write", Date.now() - t);
}

const measureMap = new Map<string, { count: number; duration: number }>();

let lastMeasureResult = Date.now();

function measure(operation: string, duration: number) {
  let a = measureMap.get(operation);

  if (!a) {
    a = { count: 0, duration: 0 };

    measureMap.set(operation, a);
  }

  a.duration += duration;

  a.count++;

  if (Date.now() - lastMeasureResult > 60000) {
    console.log(
      "Measurement:",
      [...measureMap]
        .map(
          ([operation, { count, duration }]) =>
            `${operation}: ${count}x ${duration / count}`
        )
        .sort()
    );

    measureMap.clear();

    lastMeasureResult = Date.now();
  }
}

// used for requested single scale
async function shouldRender(
  pathnameBase: string,
  tile: Tile & { reqScale: number },
  reasons: string[]
) {
  let s;
  try {
    s = await stat(
      `${pathnameBase}${tile.reqScale === 1 ? "" : `@${tile.reqScale}x`}.${extension}`
    );
  } catch {
    reasons.push("doesntExist");
    return;
  }

  // return prerenderPolygon && isOld && !tileOverlapsLimits(prerenderPolygon, tile)
  //   || prerender && (isOld || dirtyTiles.has(tile2key(tile)));

  if (prerenderer?.prerenderPolygon) {
    if (
      rerenderOlderThanMs != null &&
      s.mtimeMs < rerenderOlderThanMs &&
      !tileOverlapsLimits(prerenderer.prerenderPolygon, tile)
    ) {
      reasons.push("shouldRender");
    }
  } else {
    // reasons.push('???');
  }
}

let exportMapCount = 0;
const exportMapUnlocks: (() => void)[] = [];

// scale: my screen is 96 dpi, pdf is 72 dpi; 72 / 96 = 0.75
export async function exportMap(
  destFile: string | undefined,
  zoom: number,
  bbox: [number, number, number, number],
  scale = 1,
  features: any,
  cancelHolder: { cancelled: boolean } | undefined,
  format: ImageFormat
) {
  if (config.exportMapConcurrency == null) {
    throw new Error("exporting disabled");
  }

  if (exportMapCount >= config.exportMapConcurrency) {
    await new Promise<void>((unlock) => {
      exportMapUnlocks.push(unlock);
    });
  }

  if (cancelHolder && cancelHolder.cancelled) {
    throw new Error("Cancelled");
  }

  exportMapCount++;

  const renderer = await pool.acquire(1);

  try {
    const result = await renderer.render(
      bbox4326To3857(bbox),
      zoom,
      [scale],
      format,
      {
        shading: features.shading,
        contours: features.contours,
        bicycleRoutes: features.bicycleTrails,
        horseRoutes: features.horseTrails,
        hikingRoutes: features.hikingTrails,
        skiRoutes: features.skiTrails,
        featureCollection: JSON.stringify(features.featureCollection),
      }
    );

    if (!destFile) {
      return result.images[0];
    }

    await writeFile(destFile, result.images[0]);
  } finally {
    pool.release(renderer);

    const unlock = exportMapUnlocks.shift();

    if (unlock) {
      unlock();
    }

    exportMapCount--;

    if (global.gc) {
      global.gc();
    }
  }
}
