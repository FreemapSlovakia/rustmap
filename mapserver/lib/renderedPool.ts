import { cpus } from "os";
import genericPool, { Factory } from "generic-pool";
import { Worker } from "worker_threads";
import {
  RendererConfig,
  RenderRequest,
  RenderResponse,
  RenderResult,
} from "./renderWorker.js";
import { ImageFormat, RequestExtra } from "maprender-node";
import { config } from "./config.js";

let rendererConfig: RendererConfig = {
  connectionString: config.postgresConnectionString,
  hillshadingBase: config.dirs.hillshading ?? undefined,
  svgBase: config.dirs.svg,
  dbPriority: config.dbPriority ?? undefined,
  maskGeojsonPath: config.limits.polygon,
};

export type WorkerRenderer = {
  waitReady: () => Promise<void>;
  render: (
    bbox: [number, number, number, number],
    zoom: number,
    scales: number[],
    format: ImageFormat,
    extra?: RequestExtra
  ) => Promise<RenderResult>;
  terminate: () => Promise<void>;
};

function createWorkerRenderer(worker: Worker): WorkerRenderer {
  const pending = new Map<
    number,
    {
      resolve: (value: RenderResult) => void;
      reject: (err: Error) => void;
    }
  >();

  let nextId = 1;
  let readyResolve: (() => void) | undefined;
  let readyReject: ((err: Error) => void) | undefined;

  const readyPromise = new Promise<void>((resolve, reject) => {
    readyResolve = resolve;
    readyReject = reject;
  });

  const handleFailure = (err: Error) => {
    for (const pendingItem of pending.values()) {
      pendingItem.reject(err);
    }

    pending.clear();

    if (readyReject) {
      readyReject(err);
      readyReject = undefined;
      readyResolve = undefined;
    }
  };

  worker.on("message", (message: RenderResponse) => {
    if (message.type === "ready") {
      if (readyResolve) {
        readyResolve();
      }

      readyResolve = undefined;
      readyReject = undefined;
      return;
    }

    const pendingItem = pending.get(message.id);

    if (!pendingItem) {
      throw new Error("no such pending request: " + message.id);
    }

    pending.delete(message.id);

    if (message.type === "error") {
      const err = new Error(message.error.message);
      err.name = message.error.name || err.name;
      err.stack = message.error.stack || err.stack;
      pendingItem.reject(err);
      return;
    }

    pendingItem.resolve({
      images: message.result.images.map((image) => Buffer.from(image)),
      contentType: message.result.contentType,
    });
  });

  worker.on("error", (err) => {
    handleFailure(err);
  });

  worker.once("exit", (code) => {
    if (code !== 0) {
      handleFailure(new Error(`Render worker exited with code ${code}`));
    }
  });

  const waitReady = () => readyPromise;

  const render = async (
    bbox: [number, number, number, number],
    zoom: number,
    scales: number[],
    format: ImageFormat,
    extra?: RequestExtra
  ): Promise<RenderResult> => {
    await readyPromise;

    return new Promise<RenderResult>((resolve, reject) => {
      const id = nextId++;
      pending.set(id, { resolve, reject });

      worker.postMessage({
        id,
        bbox,
        zoom,
        scales,
        format,
        extra,
      } satisfies RenderRequest);
    });
  };

  const terminate = async () => {
    await worker.terminate();
  };

  return { waitReady, render, terminate };
}

const factory: Factory<WorkerRenderer> = {
  async create() {
    const worker = new Worker(import.meta.dirname + "/renderWorker.js", {
      workerData: rendererConfig,
    });

    const renderer = createWorkerRenderer(worker);

    await renderer.waitReady();

    return renderer;
  },

  async destroy(renderer: WorkerRenderer) {
    await renderer.terminate();
  },
};

const nCpus = cpus().length;

export const pool = genericPool.createPool(factory, {
  max: config.workers?.max ?? nCpus,
  min: config.workers?.min ?? nCpus,
});
