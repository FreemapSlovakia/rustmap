import { ImageFormat, Renderer, RequestExtra } from "maprender-node";
import { parentPort, workerData } from "worker_threads";

export type RendererConfig = {
  connectionString: string;
  hillshadingBase?: string;
  svgBase: string;
  dbPriority?: number;
  maskGeojsonPath: string;
};

export type RenderResult = ReturnType<Renderer["render"]>;

export type RenderRequest = {
  id: number;
  bbox: [number, number, number, number];
  zoom: number;
  scales: number[];
  format: ImageFormat;
  extra?: RequestExtra;
};

export type SerializedError = {
  message: string;
  name?: string;
  stack?: string;
};

export type RenderResponse =
  | { type: "ready" }
  | {
      type: "error";
      id: number;
      error: SerializedError;
    }
  | {
      type: "success";
      id: number;
      result: {
        images: Uint8Array[];
        contentType: string;
      };
    };

const pp = parentPort;

if (!pp) {
  throw new Error("parentPort is null");
}

const wd = workerData as RendererConfig;

const renderer = new Renderer(
  wd.connectionString,
  wd.hillshadingBase,
  wd.svgBase,
  wd.dbPriority,
  wd.maskGeojsonPath
);

pp.postMessage({ type: "ready" } satisfies RenderResponse);

pp.on("message", (message: RenderRequest) => {
  try {
    const result: RenderResult = renderer.render(
      message.bbox,
      message.zoom,
      message.scales,
      message.format,
      message.extra
    );

    const images = result.images.map((image) => Uint8Array.from(image));

    pp.postMessage(
      {
        type: "success",
        id: message.id,
        result: {
          images,
          contentType: result.contentType,
        },
      } satisfies RenderResponse,
      images.map((image) => image.buffer)
    );
  } catch (err) {
    pp.postMessage({
      type: "error",
      id: message.id,
      error:
        err instanceof Error
          ? { message: err.message, name: err.name, stack: err.stack }
          : { message: String(err) },
    } satisfies RenderResponse);
  }
});
