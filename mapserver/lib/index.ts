// @ts-check

import chokidar, { FSWatcher } from "chokidar";
import { prerenderer } from "./prerenderer.js";
import { fillDirtyTilesRegister } from "./dirtyTilesScanner.js";
import { processExpireFiles } from "./expireFilesProcessor.js";
import { listenHttp, closeServer } from "./httpServer.js";
import { pool } from "./renderedPool.js";
import { cleanupOutOfBoundTiles } from "./outOfBoundsCleaner.js";
import { config } from "./config.js";

let watcher: FSWatcher;

pool.on("factoryCreateError", async (error) => {
  console.error("Error creating or configuring Mapnik:", error);

  process.exitCode = 1;

  if (watcher) {
    watcher.close();
  }

  closeServer();

  await pool.drain();
  await pool.clear();
});

let depth = 0;

function processNewDirties() {
  console.info(`Processing new expire files (depth: ${depth}).`);

  depth++;

  if (depth > 1) {
    return;
  }

  global.processingExpiredTiles = true;

  processExpireFiles().then((retry) => {
    global.processingExpiredTiles = false;

    prerenderer?.resume();

    retry ||= depth > 1;

    depth = 0;

    if (retry) {
      processNewDirties();
    }
  });
}

// TODO we could maybe await
if (config.limits.cleanup) {
  cleanupOutOfBoundTiles().catch((err) => {
    console.error("Error in cleanupOutOfBoundTiles:", err);
  });
}

listenHttp();

if (config.prerender) {
  processNewDirties();

  try {
    await fillDirtyTilesRegister();

    if (config.dirs.expires) {
      watcher = chokidar.watch(config.dirs.expires);

      watcher.on("add", processNewDirties);
    }

    await prerenderer?.prerender();
  } catch (err) {
    console.error("Error filling dirty tiles register", err);

    process.exit(1);
  }
}
