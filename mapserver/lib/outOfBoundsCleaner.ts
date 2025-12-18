import config from 'config';
import { readdir, unlink } from 'fs/promises';
import path from 'path';
import { tileOverlapsLimits } from './tileCalc.js';
import { limitPolygon } from './config.js';

const tilesDir: string = config.get('dirs.tiles');

const minZoom: number = config.get('limits.minZoom');

const maxZoom: number = config.get('limits.maxZoom');

export async function cleanupOutOfBoundTiles() {
  for (const zoomStr of await readdir(tilesDir)) {
    const zoom = parseInt(zoomStr, 10);

    if (isNaN(zoom)) {
      console.warn('Unexpected zoom directory:', zoomStr);

      continue;
    }

    for (const xStr of await readdir(path.resolve(tilesDir, zoomStr))) {
      const x = parseInt(xStr, 10);

      if (isNaN(x)) {
        console.warn('Unexpected X directory:', zoom, xStr);

        continue;
      }

      for (const file of await readdir(path.resolve(tilesDir, zoomStr, xStr))) {
        const m =
          /^(\d+)(?:@\d+(?:\.\d+)?x)?\.(?:webp|jpg|jpeg|png|dirty|index)$/.exec(
            file,
          ); // TODO use format.extension

        if (!m) {
          console.warn('Unexpected file:', zoom, xStr, file);

          continue;
        }

        const y = parseInt(m[1], 10);

        if (
          zoom < minZoom ||
          zoom > maxZoom ||
          (limitPolygon && !tileOverlapsLimits(limitPolygon, { zoom, x, y }))
        ) {
          const resolvedFile = path.resolve(tilesDir, zoomStr, xStr, file);

          console.info('Removing OOB file:', resolvedFile);

          await unlink(resolvedFile);
        }
      }
    }
  }
}
