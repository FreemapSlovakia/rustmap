#!/usr/bin/env python3
"""Count place=* objects per European country via Overpass, one request per country."""

import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

ENDPOINTS = [
    "https://overpass-api.de/api/interpreter",
    "https://overpass.kumi.systems/api/interpreter",
    "https://overpass.private.coffee/api/interpreter",
]

COUNTRIES = """AL AD AT BY BE BA BG HR CY CZ DK EE FI FR DE GR HU IS IE IT XK LV LI LT LU
MT MD MC ME NL MK NO PL PT RO SM RS SK SI ES SE CH TR UA GB VA""".split()

# label -> filter appended to nwr(area.a)
QUERIES = [
    ("city", "[place=city]"),
    ("town", "[place=town]"),
    ("village", "[place=village]"),
    ("village_pop", "[place=village][population]"),
    ("hamlet", "[place=hamlet]"),
    ("hamlet_pop", "[place=hamlet][population]"),
    ("isolated_dwelling", "[place=isolated_dwelling]"),
    ("farm", "[place=farm]"),
]

OUT = "/tmp/place-stats.json"


def build(cc):
    lines = ["[out:json][timeout:600];", f'area["ISO3166-1"="{cc}"]["admin_level"="2"]->.a;']
    lines += [f"nwr(area.a){f};out count;" for _, f in QUERIES]
    return "\n".join(lines)


def fetch(cc):
    last = None
    for attempt in range(6):
        url = ENDPOINTS[attempt % len(ENDPOINTS)]
        try:
            req = urllib.request.Request(
                url,
                data=urllib.parse.urlencode({"data": build(cc)}).encode(),
                headers={"User-Agent": "freemap-outdoor-map place stats (m.zdila@gmail.com)"},
            )
            with urllib.request.urlopen(req, timeout=700) as resp:
                data = json.load(resp)
            counts = [int(e["tags"]["total"]) for e in data.get("elements", []) if e.get("type") == "count"]
            if len(counts) != len(QUERIES):
                raise ValueError(f"got {len(counts)} counts, want {len(QUERIES)}")
            return dict(zip([label for label, _ in QUERIES], counts))
        except Exception as err:  # noqa: BLE001 - report and retry
            last = err
            print(f"  {cc} attempt {attempt + 1} on {url}: {err}", file=sys.stderr, flush=True)
            time.sleep(20 * (attempt + 1))
    print(f"  {cc} FAILED: {last}", file=sys.stderr, flush=True)
    return None


results = {}
if os.path.exists(OUT):
    with open(OUT) as f:
        results = json.load(f)

for cc in COUNTRIES:
    if cc in results:
        continue
    print(f"{cc} …", flush=True)
    got = fetch(cc)
    if got is None:
        continue
    results[cc] = got
    with open(OUT, "w") as f:
        json.dump(results, f, indent=1, sort_keys=True)
    print(f"  {cc} {got}", flush=True)
    time.sleep(3)

print(f"done: {len(results)}/{len(COUNTRIES)} countries -> {OUT}")
