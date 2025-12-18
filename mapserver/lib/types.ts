declare global {
  var processingExpiredTiles: boolean;
}

export type Tile = { x: number; y: number; zoom: number };

export type DirtyTile = Tile & { zoom: number; ts: number; dt: number };

export type Legend = {
  categories: { id: unknown; name: Record<string, string> }[];
  items: {
    categoryId: unknown;
    name: Record<string, string>;
    layers: unknown;
    zoom: number;
    bbox: [number, number, number, number];
  }[];
};
