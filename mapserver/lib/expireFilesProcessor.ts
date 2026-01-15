// @ts-check

import path from "path";
import {
  readdir,
  readFile,
  unlink,
  open,
  access,
  FileHandle,
  mkdir,
} from "fs/promises";
import {
  computeZoomedTiles,
  tile2key,
  tileOverlapsLimits,
} from "./tileCalc.js";
import { dirtyTiles } from "./dirtyTilesRegister.js";
import { config } from "./config.js";
import { flock } from "fs-ext";
import { promisify } from "util";
import { Tile } from "./types.js";
import { PathLike } from "fs";
import { prerenderer } from "./prerenderer.js";

const flockAsync = promisify(
  flock as (
    fd: number,
    flags: "sh" | "ex" | "shnb" | "exnb" | "un",
    callback: (err: NodeJS.ErrnoException | null) => void
  ) => void
);

const expiresDir = config.dirs.expires;

const extension = config.format.extension;

const expiresZoom = config.expiresZoom;

export async function processExpireFiles() {
  if (!expiresDir) {
    return;
  }

  console.log("Processing expire files.");

  const dirs = await readdir(expiresDir);

  const expireFiles = (
    await Promise.all(
      dirs
        .map((dirs) => path.join(expiresDir, dirs))
        .map(async (dir) =>
          readdir(dir).then((tileFiles) =>
            tileFiles.map((tileFile) => path.join(dir, tileFile))
          )
        )
    )
  ).flat();

  const expireFilesLen = expireFiles.length;

  expireFiles.sort();

  const tiles = new Set<Tile>();

  let n = 0;

  for (const expireFile of expireFiles) {
    n++;

    const expireLines = (await readFile(expireFile, "utf8")).trim().split("\n");

    expireLines
      .map((tile) => tile.split("/").map((x) => Number(x)))
      .map(([zoom, x, y]) => ({ x, y, zoom }))
      .filter(
        (tile) =>
          !prerenderer?.prerenderPolygon ||
          tileOverlapsLimits(prerenderer.prerenderPolygon, tile)
      )
      .forEach((tile) => {
        tiles.add(tile);
      });

    if (
      config.minExpiredBatchSize != null &&
      tiles.size >= config.minExpiredBatchSize
    ) {
      break;
    }
  }

  expireFiles.splice(n, expireFiles.length - n);

  const outzoomExpiredTiles = new Set<string>();

  const collect = ({ zoom, x, y }: Tile) => {
    outzoomExpiredTiles.add(`${zoom}/${x}/${y}`);
  };

  if (prerenderer) {
    for (const tile of tiles) {
      computeZoomedTiles(
        collect,
        tile,
        config.limits.minZoom,
        prerenderer.maxZoom
      );
    }
  }

  console.log("Processing expired out-zoom tiles:", outzoomExpiredTiles.size);

  // we do it sequentially to not to kill IO
  for (const tile of outzoomExpiredTiles) {
    const tilesDir = config.dirs.tiles;

    const [zoom, x, y] = tile.split("/").map((x) => Number(x));

    const checkIfTileExists = async () => {
      const t = Date.now();

      const res = await exists(path.resolve(tilesDir, `${tile}.${extension}`));

      return res ? [Date.now() - t] : undefined;
    };

    let tt: number[] | undefined;

    if (
      prerenderer &&
      ((prerenderer.prerenderPolygon &&
        !tileOverlapsLimits(prerenderer.prerenderPolygon, { zoom, x, y })) ||
        zoom < prerenderer.minZoom ||
        zoom > prerenderer.maxZoom)
    ) {
      for (const scale of config.limits.scales) {
        const tileFile = `${tile}${
          scale === 1 ? "" : `@${scale}x`
        }.${extension}`;

        try {
          await unlink(path.resolve(tilesDir, tileFile));

          console.log("Removed expired tile:", tileFile);
        } catch (_) {
          // ignore
        }
      }
    } else if ((tt = await checkIfTileExists())) {
      const dirtyFile = `${tile}.dirty`;

      let t = Date.now();

      await mkdir(path.resolve(tilesDir, `${zoom}/${x}`), { recursive: true });

      await (await open(path.resolve(tilesDir, dirtyFile), "w")).close();

      console.log("Created dirty-file:", dirtyFile, tt[0], Date.now() - t);

      const v = { zoom, x, y, ts: Date.now(), dt: Date.now() };

      dirtyTiles.set(tile2key(v), v);
    }

    if (zoom === expiresZoom) {
      let len = 0;

      let fh: FileHandle | undefined;

      const t = Date.now();

      try {
        fh = await open(path.resolve(tilesDir, `${tile}.index`), "r+");
      } catch (err) {
        if (isNodeError(err) && err.code !== "ENOENT") {
          throw err;
        }
      }

      if (fh) {
        await flockAsync(fh.fd, "ex");

        const items = (await fh.readFile({ encoding: "utf-8" }))
          .split("\n")
          .filter((line) => line);

        len = items.length;

        for (const item of items) {
          const p = path.resolve(tilesDir, `${item}.${extension}`);

          try {
            await unlink(p);
          } catch (err) {
            console.warn("Error deleting on-demand tile: ", p, err);
          }
        }

        await fh.truncate();

        await fh.close();
      }

      console.log("Deleted on-demand dirty tiles:", len, Date.now() - t);
    }
  }

  // we do it sequentially to not to kill IO
  for (const ff of expireFiles) {
    await unlink(ff);
  }

  console.log(
    `Finished processing expire files (${expireFiles.length} of ${expireFilesLen}).`
  );

  return expireFiles.length !== expireFilesLen;
}

async function exists(file: PathLike) {
  try {
    await access(file);

    return true;
  } catch {
    return false;
  }
}

function isNodeError(err: unknown): err is NodeJS.ErrnoException {
  return err instanceof Error && "code" in err;
}
