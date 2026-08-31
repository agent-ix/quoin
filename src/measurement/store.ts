import { randomUUID } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";

import { canonicalJson, storeRoot } from "../evidence/store.js";
import type { MeasurementCollection } from "./types.js";
import {
  validateMeasurementCollection,
  validateStoredMeasurementCollection,
} from "./validate.js";
import { loadMeasurementPlans } from "./plans.js";

export function measurementsRoot(repo: string): string {
  return join(storeRoot(repo), "measurements");
}

export function measurementPath(repo: string, collectionId: string): string {
  return join(measurementsRoot(repo), `${safeId(collectionId)}.json`);
}

/** Persist a complete invocation by same-directory atomic rename. */
export function writeMeasurementCollection(
  repo: string,
  candidate: unknown,
): string {
  const plans = loadMeasurementPlans(repo);
  validateMeasurementCollection(candidate, plans);
  const path = measurementPath(repo, candidate.collectionId);
  const bytes = canonicalJson(candidate);
  if (existsSync(path)) {
    if (readFileSync(path, "utf8") === bytes) return path;
    throw new Error(
      `${path}: collection id already exists with different content`,
    );
  }
  mkdirSync(dirname(path), { recursive: true });
  const temporary = `${path}.tmp-${process.pid}-${randomUUID()}`;
  try {
    writeFileSync(temporary, bytes, { encoding: "utf8", flag: "wx" });
    renameSync(temporary, path);
  } finally {
    if (existsSync(temporary)) unlinkSync(temporary);
  }
  return path;
}

export function readMeasurementCollections(
  repo: string,
): MeasurementCollection[] {
  return readMeasurementCollectionResults(repo)
    .map((result) => {
      if (result.collection) return result.collection;
      throw new Error(
        `${result.path}: unreadable measurement collection: ${result.error ?? "unknown error"}`,
      );
    })
    .sort(collectionOrder);
}

export interface MeasurementCollectionReadResult {
  path: string;
  collection?: MeasurementCollection;
  error?: string;
}

/** Read each collection independently for failure-isolating portfolio views. */
export function readMeasurementCollectionResults(
  repo: string,
): MeasurementCollectionReadResult[] {
  const root = measurementsRoot(repo);
  if (!existsSync(root)) return [];
  return readdirSync(root)
    .filter((name) => name.endsWith(".json"))
    .sort(compare)
    .map((name) => {
      const path = join(root, name);
      try {
        const value = JSON.parse(readFileSync(path, "utf8")) as unknown;
        validateStoredMeasurementCollection(value);
        return { path, collection: value };
      } catch (error) {
        const detail = error instanceof Error ? error.message : String(error);
        return { path, error: detail };
      }
    });
}

function collectionOrder(
  a: MeasurementCollection,
  b: MeasurementCollection,
): number {
  return (
    compare(a.timestamp, b.timestamp) || compare(a.collectionId, b.collectionId)
  );
}

function safeId(value: string): string {
  if (!/^[A-Za-z0-9._-]+$/.test(value)) {
    throw new Error(`unsafe measurement collection id \`${value}\``);
  }
  return value;
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
