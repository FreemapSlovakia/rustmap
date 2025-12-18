import { cpus } from "os";
import { dirtyTiles } from "./dirtyTilesRegister.js";
import { renderTile } from "./renderer.js";
import { Worker } from "worker_threads";
import { Tile } from "./types.js";
import { config } from "./config.js";
import { Polygon } from "geojson";
import { readFileSync } from "fs";

export const prerenderer = createPrerenderer();

function createPrerenderer() {
  const prerenderConfig = config.prerender;

  if (!prerenderConfig) {
    return;
  }

  const sortWorker = new Worker(
    import.meta.dirname + "/dirtyTilesSortWorker.js",
    {
      workerData: {
        zoomPrio: prerenderConfig.zoomPrio,
      },
    }
  );

  const resumes = new Set<() => void>();

  function resume() {
    console.log("Resuming pre-rendering. Dirty tiles:", dirtyTiles.size);

    for (const rf of resumes) {
      rf();
    }

    resumes.clear();
  }

  const workers = prerenderConfig.workers ?? cpus().length;

  async function prerender() {
    console.log("Starting pre-renderer.");

    const tiles = findTilesToRender();

    await Promise.all(
      Array(workers)
        .fill(0)
        .map(() => worker(tiles))
    );

    throw new Error("unexpected");
  }

  async function* findTilesToRender() {
    let restart = false;

    function setRestartFlag() {
      restart = true;
    }

    main: for (;;) {
      resumes.add(setRestartFlag);

      console.log("(Re)starting pre-rendering worker.");

      const tiles = await new Promise<Tile[]>((resolve) => {
        sortWorker.once("message", (value: Tile[]) => {
          resolve(value);
        });

        sortWorker.postMessage([...dirtyTiles.values()]);
      });

      for (const t of tiles) {
        if (restart) {
          restart = false;

          continue main;
        }

        yield t;
      }

      resumes.delete(setRestartFlag);

      console.log("Putting pre-rendering worker to sleep.");

      await new Promise<void>((resolve) => {
        resumes.add(resolve);
      });
    }
  }

  async function worker(tiles: AsyncIterableIterator<Tile>) {
    for await (const { x, y, zoom } of tiles) {
      await renderTile(zoom, x, y);
    }
  }

  const prerenderPolygonFile = prerenderConfig.polygon;

  const prerenderPolygon: Polygon | undefined =
    prerenderPolygonFile &&
    JSON.parse(readFileSync(prerenderPolygonFile, "utf-8"));

  return {
    resume,
    prerender,
    prerenderPolygon,
    minZoom: prerenderConfig.minZoom,
    maxZoom: prerenderConfig.maxZoom,
  };
}
