// @ts-check

import chokidar, { FSWatcher } from 'chokidar';
import config from 'config';
import { prerender, resume } from './prerenderer.js';
import { fillDirtyTilesRegister } from './dirtyTilesScanner.js';
import { processExpireFiles } from './expireFilesProcessor.js';
import { listenHttp, closeServer } from './httpServer.js';
import { pool } from './renderedPool.js';
import { cleanupOutOfBoundTiles } from './outOfBoundsCleaner.js';
import { PrerenderConfig } from './types.js';

const cleanup: boolean = config.get('limits.cleanup');

const prerenderConfig: PrerenderConfig = config.get('prerender');
const tilesDir: string = config.get('dirs.tiles');
const expiresDir: string = config.get('dirs.expires');

let watcher: FSWatcher;

pool.on('factoryCreateError', async (error) => {
  console.error('Error creating or configuring Mapnik:', error);

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

  processExpireFiles(tilesDir).then((retry) => {
    global.processingExpiredTiles = false;

    resume();

    retry ||= depth > 1;

    depth = 0;

    if (retry) {
      processNewDirties();
    }
  });
}

// TODO we could maybe await
if (cleanup) {
  cleanupOutOfBoundTiles().catch((err) => {
    console.error('Error in cleanupOutOfBoundTiles:', err);
  });
}

if (prerenderConfig) {
  processNewDirties();

  fillDirtyTilesRegister()
    .then(() => {
      listenHttp();

      watcher = chokidar.watch(expiresDir);

      watcher.on('add', processNewDirties);

      return prerender();
    })
    .catch((err) => {
      console.error('Error filling dirty tiles register', err);

      process.exit(1);
    });
} else {
  listenHttp();
}
