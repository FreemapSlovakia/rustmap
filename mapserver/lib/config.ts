import fs from 'fs';
import config from 'config';
import { Polygon } from 'geojson';

export let limitPolygon: Polygon | undefined = config.get('limits.polygon');

if (typeof limitPolygon === 'string') {
  limitPolygon = JSON.parse(fs.readFileSync(limitPolygon).toString());
}

const prerender: { polygon: unknown } | undefined = config.get('prerender');

export let prerenderPolygon: Polygon;

if (prerender && typeof prerender.polygon === 'string') {
  prerenderPolygon = JSON.parse(fs.readFileSync(prerender.polygon).toString());
}
