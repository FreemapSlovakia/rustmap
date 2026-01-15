import { parentPort, workerData } from "worker_threads";
import type { DirtyTile } from "./types.js";

const pp = parentPort;

if (!pp) {
  throw new Error("parentPort is null");
}

pp.on("message", (value) => {
  pp.postMessage(
    !workerData.zoomPrio
      ? value
      : value.sort((a: DirtyTile, b: DirtyTile) => {
          const c = workerData.zoomPrio.indexOf(a.zoom);

          const d = workerData.zoomPrio.indexOf(b.zoom);

          return c === d ? a.ts - b.ts : c - d;
        })
  );
});
