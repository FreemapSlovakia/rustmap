// deno run --allow-net stress.ts
// Fetches 24 tiles (6x4) per “country URL”, all in parallel. Discards bodies.

type Entry = { label: string; url: string };

const base = "http://localhost:3050"; // rust
// const base = "http://localhost:4000"; // mapnik

const entries: Entry[] = [
  { label: "SI", url: base + "/15/17754/11616@2x" },
  { label: "AT", url: base + "/15/17612/11485@2x" },
  { label: "SK", url: base + "/13/4549/2813@2x" },
  { label: "CH", url: base + "/15/17156/11544@2x" },
  { label: "FR", url: base + "/13/4180/2919@2x" },
  { label: "CZ", url: base + "/14/8865/5565@2x" },
  { label: "PL", url: base + "/16/36543/22292@2x" },
  { label: "IT", url: base + "/12/2195/1507@2x" },
];

const GRID_W = 6;
const GRID_H = 4;

const parseTms = (raw: string) => {
  const u = new URL(raw);
  const m = u.pathname.match(/^\/(\d+)\/(\d+)\/(\d+)(@2x)?$/);
  if (!m) {
    throw new Error(`Unexpected URL path format: ${u.pathname}`);
  }

  const z = Number(m[1]);
  const x = Number(m[2]);
  const y = Number(m[3]);
  const retina = Boolean(m[4]);

  return { origin: u, z, x, y, retina };
};

const buildUrl = (
  origin: URL,
  z: number,
  x: number,
  y: number,
  retina: boolean
) => {
  const u = new URL(origin.toString());
  u.pathname = `/${z}/${x}/${y}${retina ? "@2x" : ""}`;
  return u.toString();
};

const fetchOne = async (label: string, url: string) => {
  const res = await fetch(url);

  if (!res.ok) {
    res.body?.cancel();
    throw new Error(`${label} ${res.status} ${res.statusText} ${url}`);
  }

  // // Discard body without buffering it into memory.
  // res.body?.cancel();

  await res.arrayBuffer();
};

const build24 = (e: Entry) => {
  const { origin, z, x, y, retina } = parseTms(e.url);

  const urls: string[] = [];
  for (let dy = 0; dy < GRID_H; dy += 1) {
    for (let dx = 0; dx < GRID_W; dx += 1) {
      urls.push(buildUrl(origin, z, x + dx, y + dy, retina));
    }
  }
  return urls.map((u) => ({ label: e.label, url: u }));
};

const main = async () => {
  const all = entries.flatMap(build24);

  const started = performance.now();
  const results = await Promise.allSettled(
    all.map((t) => fetchOne(t.label, t.url))
  );
  const elapsedMs = Math.round(performance.now() - started);

  const failed = results.filter(
    (r) => r.status === "rejected"
  ) as PromiseRejectedResult[];

  if (failed.length > 0) {
    console.error(`Failed: ${failed.length}/${results.length}`);
    for (const f of failed.slice(0, 20)) {
      console.error(String(f.reason));
    }
    Deno.exit(1);
  }

  console.log(`OK: ${results.length} requests in ${elapsedMs} ms`);
};

await main();
