/**
 * Canonical serialization for retained store bytes (FR-030).
 *
 * Every write is **canonical**: key-sorted, stable ordering, two-space JSON
 * with a trailing newline. That is what makes a PR diff of the store *be* the
 * per-PR delta — a store whose serialization wobbled would produce noise diffs
 * that reviewers learn to skip, which is how a review artifact stops being
 * read.
 */

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

/**
 * Canonical JSON: keys sorted at every level, two-space indent, trailing
 * newline. Deterministic by construction, so two runs over identical inputs
 * produce identical bytes.
 */
export function canonicalJson(value: unknown): string {
  return `${JSON.stringify(sortKeys(value), null, 2)}\n`;
}

function sortKeys(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortKeys);
  if (value === null || typeof value !== "object") return value;
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(value as Record<string, unknown>).sort()) {
    out[key] = sortKeys((value as Record<string, unknown>)[key]);
  }
  return out;
}

export function writeCanonical(path: string, value: unknown): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, canonicalJson(value), "utf8");
}
