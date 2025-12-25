import {
  array,
  boolean,
  integer,
  minValue,
  nonEmpty,
  nullable,
  number,
  optional,
  pipe,
  strictObject,
  string,
  type InferOutput,
} from "valibot";

function nullish<T extends Parameters<typeof nullable>[0]>(a: T) {
  return optional(nullable(a));
}

export const configSchema = strictObject({
  extends: nullish(array(pipe(string(), nonEmpty()))),
  postgresConnectionString: pipe(string(), nonEmpty()),
  dbPriority: nullish(pipe(number(), integer())),
  dirs: strictObject({
    tiles: string(),
    expires: nullish(string()),
    hillshading: nullish(pipe(string(), nonEmpty())),
    svg: pipe(string(), nonEmpty()),
  }),
  server: nullish(
    strictObject({
      host: nullish(string()),
      port: number(),
    })
  ),
  // min/max are optional, otherwise use number of CPUs
  workers: nullish(
    strictObject({
      min: optional(number()),
      max: optional(number()),
    })
  ),
  forceTileRendering: optional(boolean(), false),
  mapFeatures: strictObject({
    contours: boolean(),
    shading: boolean(),
    hikingTrails: boolean(),
    bicycleTrails: boolean(),
    skiTrails: boolean(),
    horseTrails: boolean(),
  }),
  limits: strictObject({
    minZoom: pipe(number(), integer(), minValue(0)),
    maxZoom: pipe(number(), integer(), minValue(0)),
    polygon: string(),
    scales: optional(array(number()), [1]),
    cleanup: boolean(),
  }),
  // null disables pre-rendering
  prerender: nullish(
    strictObject({
      // workers is optional; when omitted use number of CPUs
      workers: nullish(number()),
      minZoom: number(),
      maxZoom: number(),
      polygon: string(),
      zoomPrio: array(number()),
    })
  ),
  rerenderOlderThanMs: nullish(number()),
  exportMapConcurrency: nullish(number()),
  format: strictObject({
    extension: string(),
    mimeType: string(),
  }),
  minExpiredBatchSize: nullish(number()), // null = delete without batching
  expiresZoom: number(),
  prerenderMaxZoom: number(),
  prerenderDelayWhenExpiring: number(),
  maskPolygon: string(),
});

export type Config = InferOutput<typeof configSchema>;
