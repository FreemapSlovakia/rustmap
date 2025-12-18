import fs from 'fs/promises';
import os from 'os';
import path from 'path';
import http from 'http';
import { koaBody } from 'koa-body';
import { Ajv } from 'ajv';
import crypto from 'crypto';
import config from 'config';
import Koa, { Context } from 'koa';
import Router from '@koa/router';
import send from 'koa-send';
import cors from '@koa/cors';

import { renderTile, exportMap } from './renderer.js';
import { tileOverlapsLimits } from './tileCalc.js';
import { limitPolygon } from './config.js';
import { JSONSchema7 } from 'json-schema';
import { Legend } from './types.js';

const app = new Koa();

const router = new Router();

const limitScales: number[] = config.get('limits.scales');

const serverOptions = config.get('server');

const tilesDir: string = config.get('dirs.tiles');

const notFoundAsTransparent = config.get('notFoundAsTransparent');

const mimeType: string = config.get('format.mimeType');

const minZoom: number = config.get('limits.minZoom');

const maxZoom: number = config.get('limits.maxZoom');

let legend: Legend;

async function getTileMiddleware(ctx: Context) {
  const { zz, xx, yy } = ctx.params;

  const yyMatch = /(\d+)(?:@(\d+(?:\.\d+)?)x)?/.exec(yy);

  if (!yyMatch) {
    ctx.throw(400);
  }

  const x = Number.parseInt(xx, 10);

  const y = Number.parseInt(yyMatch[1], 10);

  const zoom = Number.parseInt(zz, 10);

  const scale = yyMatch[2] ? Number.parseFloat(yyMatch[2]) : 1;

  if (
    Number.isNaN(zoom) ||
    Number.isNaN(x) ||
    Number.isNaN(y) ||
    Number.isNaN(scale)
  ) {
    ctx.throw(400);
  }

  if (
    zoom < minZoom ||
    zoom > maxZoom ||
    (limitPolygon && !tileOverlapsLimits(limitPolygon, { zoom, x, y })) ||
    !limitScales.includes(scale)
  ) {
    ctx.throw(404);

    // TODO
    //   if (!notFoundAsTransparent) {
    //   ctx.throw(404);
    // }
  }

  const file = await renderTile(zoom, x, y, scale);

  const stats = await fs.stat(file!);

  ctx.set('Last-Modified', stats.mtime.toUTCString());

  if (ctx.fresh) {
    ctx.status = 304;

    return;
  }

  ctx.mimeType = mimeType;

  await send(ctx, path.relative(tilesDir, file!), { root: tilesDir });
}

router.get('/:zz/:xx/:yy', getTileMiddleware);

function getQueryParam(ctx: Context, key: string) {
  const value = ctx.query[key];

  return Array.isArray(value) ? value[0] : value;
}

// TODO make more configurable and less hardcoded
// TODO return better error responses
router.get('/service', async (ctx) => {
  const {
    SERVICE,
    VERSION,
    REQUEST,
    TILEMATRIXSET,
    TILEMATRIX, // zoom
    TILECOL,
    TILEROW,
    LAYER,
    FORMAT,
  } = Object.fromEntries(
    Object.entries(ctx.query).map(([key, value]) => [
      key,
      Array.isArray(value) ? value[0] : value,
    ]),
  );

  if (SERVICE !== 'WMTS' || (VERSION && VERSION !== '1.0.0')) {
    ctx.status = 400;

    return;
  }

  if (
    REQUEST === 'GetTile' &&
    LAYER === 'freemap_outdoor' &&
    TILEMATRIXSET === 'webmercator' &&
    FORMAT === 'image/jpeg'
  ) {
    ctx.params = { zz: TILEMATRIX!, xx: TILECOL!, yy: TILEROW! };

    return getTileMiddleware(ctx);
  } else if (
    REQUEST === 'GetTile' &&
    LAYER === 'freemap_outdoor_2x' &&
    TILEMATRIXSET === 'webmercator_2x' &&
    FORMAT === 'image/jpeg'
  ) {
    ctx.params = { zz: TILEMATRIX!, xx: TILECOL!, yy: TILEROW! + '@2x' };

    return getTileMiddleware(ctx);
  } else if (REQUEST === 'GetCapabilities') {
    ctx.set('Content-Type', 'application/xml');

    ctx.body = `<?xml version="1.0"?>
<Capabilities
  xmlns="http://www.opengis.net/wmts/1.0"
  xmlns:ows="http://www.opengis.net/ows/1.1"
  xmlns:xlink="http://www.w3.org/1999/xlink"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xmlns:gml="http://www.opengis.net/gml"
  xsi:schemaLocation="http://www.opengis.net/wmts/1.0 http://schemas.opengis.net/wmts/1.0/wmtsGetCapabilities_response.xsd"
  version="1.0.0"
>
  <ows:ServiceIdentification>
    <ows:Title></ows:Title>
    <ows:Abstract></ows:Abstract>
    <ows:ServiceType>OGC WMTS</ows:ServiceType>
    <ows:ServiceTypeVersion>1.0.0</ows:ServiceTypeVersion>
    <ows:Fees>none</ows:Fees>
    <ows:AccessConstraints>CC-BY 4.0 (Freemap Slovakia) a ODbL 1.0 (prispievatelia OpenStreetMap)</ows:AccessConstraints>
  </ows:ServiceIdentification>
  <ows:OperationsMetadata>
    <ows:Operation name="GetCapabilities">
      <ows:DCP>
        <ows:HTTP>
          <ows:Get xlink:href="https://outdoor.tiles.freemap.sk/service?">
            <ows:Constraint name="GetEncoding">
              <ows:AllowedValues>
                <ows:Value>KVP</ows:Value>
              </ows:AllowedValues>
            </ows:Constraint>
          </ows:Get>
        </ows:HTTP>
      </ows:DCP>
    </ows:Operation>
    <ows:Operation name="GetTile">
      <ows:DCP>
        <ows:HTTP>
          <ows:Get xlink:href="https://outdoor.tiles.freemap.sk/service?">
            <ows:Constraint name="GetEncoding">
              <ows:AllowedValues>
                <ows:Value>KVP</ows:Value>
              </ows:AllowedValues>
            </ows:Constraint>
          </ows:Get>
        </ows:HTTP>
      </ows:DCP>
    </ows:Operation>
  </ows:OperationsMetadata>
  <Contents>
    <Layer>
      <ows:Title>Freemap Slovakia Outdoor</ows:Title>
      <ows:Abstract></ows:Abstract>
      <ows:WGS84BoundingBox>
        <ows:LowerCorner>-180.0 -85.0511287798</ows:LowerCorner>
        <ows:UpperCorner>180.0 85.0511287798</ows:UpperCorner>
      </ows:WGS84BoundingBox>
      <ows:Identifier>freemap_outdoor</ows:Identifier>
      <Style>
        <ows:Identifier>default</ows:Identifier>
      </Style>
      <Format>image/jpeg</Format>
      <TileMatrixSetLink>
        <TileMatrixSet>webmercator</TileMatrixSet>
      </TileMatrixSetLink>
    </Layer>
    <Layer>
      <ows:Title>Freemap Slovakia Outdoor Hi DPI</ows:Title>
      <ows:Abstract></ows:Abstract>
      <ows:WGS84BoundingBox>
        <ows:LowerCorner>-180.0 -85.0511287798</ows:LowerCorner>
        <ows:UpperCorner>180.0 85.0511287798</ows:UpperCorner>
      </ows:WGS84BoundingBox>
      <ows:Identifier>freemap_outdoor_2x</ows:Identifier>
      <Style>
        <ows:Identifier>default</ows:Identifier>
      </Style>
      <Format>image/jpeg</Format>
      <TileMatrixSetLink>
        <TileMatrixSet>webmercator_2x</TileMatrixSet>
      </TileMatrixSetLink>
    </Layer>
    <TileMatrixSet>
      <ows:Identifier>webmercator</ows:Identifier>
      <ows:SupportedCRS>EPSG:3857</ows:SupportedCRS>
      <TileMatrix>
        <ows:Identifier>00</ows:Identifier>
        <ScaleDenominator>559082264.029</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>1</MatrixWidth>
        <MatrixHeight>1</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>01</ows:Identifier>
        <ScaleDenominator>279541132.014</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>2</MatrixWidth>
        <MatrixHeight>2</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>02</ows:Identifier>
        <ScaleDenominator>139770566.007</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>4</MatrixWidth>
        <MatrixHeight>4</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>03</ows:Identifier>
        <ScaleDenominator>69885283.0036</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>8</MatrixWidth>
        <MatrixHeight>8</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>04</ows:Identifier>
        <ScaleDenominator>34942641.5018</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>16</MatrixWidth>
        <MatrixHeight>16</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>05</ows:Identifier>
        <ScaleDenominator>17471320.7509</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>32</MatrixWidth>
        <MatrixHeight>32</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>06</ows:Identifier>
        <ScaleDenominator>8735660.37545</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>64</MatrixWidth>
        <MatrixHeight>64</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>07</ows:Identifier>
        <ScaleDenominator>4367830.18772</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>128</MatrixWidth>
        <MatrixHeight>128</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>08</ows:Identifier>
        <ScaleDenominator>2183915.09386</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>256</MatrixWidth>
        <MatrixHeight>256</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>09</ows:Identifier>
        <ScaleDenominator>1091957.54693</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>512</MatrixWidth>
        <MatrixHeight>512</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>10</ows:Identifier>
        <ScaleDenominator>545978.773466</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>1024</MatrixWidth>
        <MatrixHeight>1024</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>11</ows:Identifier>
        <ScaleDenominator>272989.386733</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>2048</MatrixWidth>
        <MatrixHeight>2048</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>12</ows:Identifier>
        <ScaleDenominator>136494.693366</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>4096</MatrixWidth>
        <MatrixHeight>4096</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>13</ows:Identifier>
        <ScaleDenominator>68247.3466832</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>8192</MatrixWidth>
        <MatrixHeight>8192</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>14</ows:Identifier>
        <ScaleDenominator>34123.6733416</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>16384</MatrixWidth>
        <MatrixHeight>16384</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>15</ows:Identifier>
        <ScaleDenominator>17061.8366708</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>32768</MatrixWidth>
        <MatrixHeight>32768</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>16</ows:Identifier>
        <ScaleDenominator>8530.9183354</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>65536</MatrixWidth>
        <MatrixHeight>65536</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>17</ows:Identifier>
        <ScaleDenominator>4265.4591677</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>131072</MatrixWidth>
        <MatrixHeight>131072</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>18</ows:Identifier>
        <ScaleDenominator>2132.72958385</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>262144</MatrixWidth>
        <MatrixHeight>262144</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>19</ows:Identifier>
        <ScaleDenominator>1066.36479192</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>256</TileWidth>
        <TileHeight>256</TileHeight>
        <MatrixWidth>524288</MatrixWidth>
        <MatrixHeight>524288</MatrixHeight>
      </TileMatrix>
    </TileMatrixSet>
    <TileMatrixSet>
      <ows:Identifier>webmercator_2x</ows:Identifier>
      <ows:SupportedCRS>EPSG:3857</ows:SupportedCRS>
      <TileMatrix>
        <ows:Identifier>00</ows:Identifier>
        <ScaleDenominator>279541132.014</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>1</MatrixWidth>
        <MatrixHeight>1</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>01</ows:Identifier>
        <ScaleDenominator>139770566.007</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>2</MatrixWidth>
        <MatrixHeight>2</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>02</ows:Identifier>
        <ScaleDenominator>69885283.0036</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>4</MatrixWidth>
        <MatrixHeight>4</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>03</ows:Identifier>
        <ScaleDenominator>34942641.5018</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>8</MatrixWidth>
        <MatrixHeight>8</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>04</ows:Identifier>
        <ScaleDenominator>17471320.7509</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>16</MatrixWidth>
        <MatrixHeight>16</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>05</ows:Identifier>
        <ScaleDenominator>8735660.37545</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>32</MatrixWidth>
        <MatrixHeight>32</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>06</ows:Identifier>
        <ScaleDenominator>4367830.18772</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>64</MatrixWidth>
        <MatrixHeight>64</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>07</ows:Identifier>
        <ScaleDenominator>2183915.09386</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>128</MatrixWidth>
        <MatrixHeight>128</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>08</ows:Identifier>
        <ScaleDenominator>1091957.54693</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>256</MatrixWidth>
        <MatrixHeight>256</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>09</ows:Identifier>
        <ScaleDenominator>545978.773466</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>512</MatrixWidth>
        <MatrixHeight>512</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>10</ows:Identifier>
        <ScaleDenominator>272989.386733</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>1024</MatrixWidth>
        <MatrixHeight>1024</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>11</ows:Identifier>
        <ScaleDenominator>136494.693366</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>2048</MatrixWidth>
        <MatrixHeight>2048</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>12</ows:Identifier>
        <ScaleDenominator>68247.3466832</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>4096</MatrixWidth>
        <MatrixHeight>4096</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>13</ows:Identifier>
        <ScaleDenominator>34123.6733416</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>8192</MatrixWidth>
        <MatrixHeight>8192</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>14</ows:Identifier>
        <ScaleDenominator>17061.8366708</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>16384</MatrixWidth>
        <MatrixHeight>16384</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>15</ows:Identifier>
        <ScaleDenominator>8530.9183354</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>32768</MatrixWidth>
        <MatrixHeight>32768</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>16</ows:Identifier>
        <ScaleDenominator>4265.4591677</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>65536</MatrixWidth>
        <MatrixHeight>65536</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>17</ows:Identifier>
        <ScaleDenominator>2132.72958385</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>131072</MatrixWidth>
        <MatrixHeight>131072</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>18</ows:Identifier>
        <ScaleDenominator>1066.36479192</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>262144</MatrixWidth>
        <MatrixHeight>262144</MatrixHeight>
      </TileMatrix>
      <TileMatrix>
        <ows:Identifier>19</ows:Identifier>
        <ScaleDenominator>533.182395962</ScaleDenominator>
        <TopLeftCorner>-20037508.3428 20037508.3428</TopLeftCorner>
        <TileWidth>512</TileWidth>
        <TileHeight>512</TileHeight>
        <MatrixWidth>524288</MatrixWidth>
        <MatrixHeight>524288</MatrixHeight>
      </TileMatrix>
    </TileMatrixSet>
  </Contents>
</Capabilities>
`;

    return;
  }

  ctx.status = 400;
});

// router.get('/legend', async (ctx) => {
//   const language = getQueryParam(ctx, 'language');

//   function msg(messages: Record<string, string>) {
//     return (
//       messages[
//         language || ctx.acceptsLanguages(Object.keys(messages)) || 'en'
//       ] || messages['en']
//     );
//   }

//   ctx.body = {
//     categories: legend.categories.map((item) => ({
//       id: item.id,
//       name: msg(item.name),
//     })),
//     items: legend.items.map((item) => ({
//       categoryId: item.categoryId,
//       name: msg(item.name),
//     })),
//   };
// });

// router.get('/legend-image/:id', async (ctx) => {
//   // TODO schema validation

//   const legendItem = legend.items[Number(ctx.params.id)];

//   ctx.set('Content-Type', 'image/png');

//   // 360 = 256 * 2^zoom

//   const scale = (ctx.query.scale && Number(ctx.query.scale)) || 1;

//   ctx.body =
//     legendItem &&
//     (await exportMap(
//       undefined,
//       generateMapnikConfig({ legendLayers: legendItem.layers }),
//       legendItem.zoom,
//       legendItem.bbox,
//       scale,
//       scale *
//         (legendItem.bbox[2] - legendItem.bbox[0]) *
//         Math.pow(2, legendItem.zoom),
//       undefined,
//       'png',
//     ));
// });

const ajv = new Ajv();

const schema: JSONSchema7 = {
  type: 'object',
  required: ['zoom', 'bbox'],
  properties: {
    zoom: { type: 'integer', minimum: 0, maximum: 20 },
    bbox: {
      type: 'array',
      minItems: 4,
      maxItems: 4,
      items: { type: 'number' },
    },
    format: { type: 'string', enum: ['pdf', 'svg', 'jpeg', 'png'] },
    features: {},
    scale: { type: 'number', minimum: 0.1, maximum: 10 },
    width: { type: 'number', minimum: 1, maximum: 10000 },
    custom: {
      type: 'object',
      required: ['layers', 'styles'],
      properties: {
        layers: {
          type: 'array',
          items: {
            type: 'object',
            required: ['styles', 'geojson'],
            properties: {
              styles: { type: 'array', items: { type: 'string' } },
              geojson: {
                type: 'object',
                // TODO geojson schema
              },
            },
          },
        },
        styles: {
          type: 'array',
          items: {
            type: 'object',
            required: ['Style'],
            properties: {
              Style: {
                type: 'object',
                required: ['@name'],
                properties: {
                  '@name': { type: 'string' },
                  // TODO other mapnik props
                },
              },
            },
          },
        },
      },
    },
  },
};

const validate = ajv.compile(schema);

const jobMap = new Map();

const exportRouter = new Router();

exportRouter.post('/', koaBody({ jsonLimit: '16mb' }), async (ctx) => {
  if (!ctx.accepts('application/json')) {
    ctx.throw(406);
  }

  if (!validate(ctx.request.body)) {
    ctx.throw(400, ajv.errorsText(validate.errors));
  }

  const { zoom, bbox, format = 'pdf', scale } = ctx.request.body as any;

  const token = crypto.randomBytes(16).toString('hex');

  const filename = `export-${token}.${format}`;

  const exportFile = path.resolve(os.tmpdir(), filename);

  const cancelHolder = { cancelled: false };

  const cancelHandler = () => {
    cancelHolder.cancelled = true;
  };

  ctx.req.on('close', cancelHandler);

  try {
    jobMap.set(token, {
      exportFile,
      filename,
      cancelHandler,
      promise: exportMap(exportFile, zoom, bbox, scale, cancelHolder, format),
    });
  } finally {
    ctx.req.off('close', cancelHandler);
  }

  ctx.body = { token };

  // TODO periodically delete old temp files
});

exportRouter.head('/', async (ctx) => {
  const job = jobMap.get(ctx.query.token);

  if (!job) {
    ctx.throw(404);
  }

  await job.promise;

  ctx.status = 200;
});

exportRouter.get('/', async (ctx) => {
  const job = jobMap.get(ctx.query.token);

  if (!job) {
    ctx.throw(404);
  }

  await job.promise;

  await send(ctx, job.filename, { root: os.tmpdir() });
});

exportRouter.delete('/', async (ctx) => {
  const job = jobMap.get(ctx.query.token);

  if (!job) {
    ctx.throw(404);
  }

  job.cancelHandler();

  await fs.unlink(job.exportFile);

  ctx.status = 204;
});

router.use('/export', exportRouter.routes(), exportRouter.allowedMethods());

app.use(cors()).use(router.routes()).use(router.allowedMethods());

const server = http.createServer(app.callback());

export function listenHttp() {
  if (serverOptions) {
    server.listen(serverOptions, () => {
      console.log(`HTTP server listening.`, serverOptions);
    });
  }
}

export function closeServer() {
  server.close();
}
