/// TypeScript declarations for the Node binding built by `maprender-node`.
/// Consumers import from the compiled package entry point.

import { Buffer } from "buffer";

export type RenderFormat = "png" | "jpg" | "jpeg" | "pdf" | "svg";

export interface RenderResult {
  data: Buffer;
  contentType: string;
}

export class Renderer {
  /**
   * Create a renderer with an existing PostgreSQL connection string.
   */
  constructor(connectionString: string, hillshadingBase: string, svgBase: string);

  /**
   * Render a tile for the given bbox/zoom/scale/format.
   * Scale defaults to 1.0; format defaults to "png".
   */
  render(
    bbox: [number, number, number, number],
    zoom: number,
    scale?: number,
    format?: RenderFormat
  ): RenderResult;
}

export type RenderResources = Renderer;
