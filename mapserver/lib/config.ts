import fs from "fs";
import path from "path";
import { Polygon } from "geojson";
import { deepmerge } from "@fastify/deepmerge";
import JSON5 from "json5";
import { safeParse } from "valibot";
import { configSchema } from "./configSchema.js";

const configPath = process.argv[2];

if (!configPath) {
  console.error("Missing config file parameter");

  process.exit(1);
}

const merge = deepmerge({ all: true });

const loadedConfig = loadConfig(configPath);

const parsedConfig = safeParse(configSchema, loadedConfig);

if (!parsedConfig.success) {
  console.error("Invalid config file:", parsedConfig.issues);

  process.exit(1);
}

export const config = parsedConfig.output;

export const limitPolygon: Polygon | undefined =
  config.limits.polygon &&
  JSON.parse(fs.readFileSync(config.limits.polygon).toString());

function loadConfig(
  configFilePath: string,
  seen = new Set<string>()
): Record<string, unknown> {
  const absPath = path.resolve(configFilePath);

  if (seen.has(absPath)) {
    console.error("Circular config extends detected:", absPath);

    process.exit(1);
  }

  seen.add(absPath);

  let configText: string;

  try {
    configText = fs.readFileSync(absPath, "utf-8");
  } catch (err) {
    console.error("Can't read config file:", errToString(err));

    process.exit(1);
  }

  const parsed = parseConfigFile(configText);

  let extendFiles;

  if (Array.isArray(parsed.extends)) {
    extendFiles = parsed.extends;
    delete parsed.extends;
  } else {
    extendFiles = [];
  }

  const mergedExtends = merge(
    ...extendFiles.map((relPath) =>
      loadConfig(path.resolve(path.dirname(absPath), relPath), seen)
    )
  );

  return merge(mergedExtends, parsed);
}

function errToString(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function parseConfigFile(text: string): Record<string, unknown> {
  let parsed;

  try {
    parsed = JSON5.parse(text);
  } catch (err) {
    console.error("Config file is not valid JSON5:", errToString(err));

    process.exit(1);
  }

  if (!parsed || typeof parsed !== "object") {
    console.error("Config must be an object");

    process.exit(1);
  }

  return parsed;
}
