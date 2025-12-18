import path from 'path';
import config from 'config';
import { stat } from 'fs/promises';
import { dirtyTiles } from './dirtyTilesRegister.js';
import { tile2key, tileRangeGenerator } from './tileCalc.js';
import { prerenderPolygon } from './config.js';
import { PrerenderConfig } from './types.js';

const rerenderOlderThanMs: number | undefined = config.get(
  'rerenderOlderThanMs',
);

const extension: string = config.get('format.extension');

const limitScales: number[] = config.get('limits.scales');

const prerenderConfig: PrerenderConfig = config.get('prerender');

const tilesDir: string = config.get('dirs.tiles');

export async function fillDirtyTilesRegister() {
  console.log('Scanning dirty tiles.');

  const { minZoom, maxZoom } = prerenderConfig;

  for (const { zoom, x, y } of tileRangeGenerator(
    prerenderPolygon,
    minZoom,
    maxZoom,
  )) {
    let mtimeMs: number;

    try {
      // find oldest
      const proms = limitScales.map((scale) =>
        stat(
          path.join(
            tilesDir,
            `${zoom}/${x}/${y}${scale === 1 ? '' : `@${scale}x`}.${extension}`,
          ),
        ),
      );

      mtimeMs = Math.min(
        ...(await Promise.all(proms)).map((stat) => stat.mtimeMs),
      );
    } catch (e) {
      const v = { zoom, x, y, ts: 0, dt: 0 };
      dirtyTiles.set(tile2key(v), v);
      continue;
    }

    if (rerenderOlderThanMs && mtimeMs < rerenderOlderThanMs) {
      const v = { zoom, x, y, ts: mtimeMs, dt: 0 };
      dirtyTiles.set(tile2key(v), v);
      continue;
    }

    try {
      const { mtimeMs } = await stat(
        path.join(tilesDir, `${zoom}/${x}/${y}.dirty`),
      );

      const v = { zoom, x, y, ts: mtimeMs, dt: mtimeMs };

      dirtyTiles.set(tile2key(v), v);
    } catch (e) {
      // fresh
    }
  }

  console.log('Dirty tiles scanned.');

  return false;
}
