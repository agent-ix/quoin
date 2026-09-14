/**
 * The locally declared evidence constants, pinned against the boundary that
 * owns them (quoin#458).
 *
 * `src/core/evidence.ts` states the adapter list, the mutation metric name, the
 * store schema version and the four store file names as plain values, because
 * they are read by pure code and by an oclif `static flags` initialiser and
 * neither may spawn a subprocess. `evidence.store_facts` serves the same values
 * from `quoin-evidence`. This file is what makes the local copies a mirror
 * rather than a second opinion: a drift on either side fails here by name.
 *
 * It needs the real binary, so it skips without `QUOIN_CORE` — `make rust-e2e`
 * is the lane that sets it.
 */

import { accessSync, constants } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  ADAPTER_NAMES,
  MUTATION_SCORE_METRIC,
  STORE_SCHEMA_VERSION,
  baselinePath,
  bindingsPath,
  inspectionsPath,
  storeFacts,
  storeRoot,
  suitesPath,
} from "../src/core/evidence.js";

function coreBinary(): string | null {
  const path = process.env.QUOIN_CORE;
  if (!path) return null;
  try {
    accessSync(path, constants.X_OK);
    return path;
  } catch {
    return null;
  }
}

describe.skipIf(coreBinary() === null)("src/core/evidence.ts constants", () => {
  // Trace: FR-101-AC-1
  it("declares exactly what evidence.store_facts serves", () => {
    const facts = storeFacts();
    // Order included: `--adapter` help text and the registry's run-shaped
    // then finding-shaped grouping are both stated by this sequence.
    expect(ADAPTER_NAMES).toEqual(facts.adapter_names);
    expect(MUTATION_SCORE_METRIC).toBe(facts.mutation_score_metric);
    expect(STORE_SCHEMA_VERSION).toBe(facts.store_schema_version);
  });

  // Trace: FR-101-AC-1
  it("resolves each store path to the boundary's own store-relative name", () => {
    const facts = storeFacts();
    const repo = "/repo";
    const root = storeRoot(repo);
    // The root itself is the boundary's too: `store_root_path` is stated
    // relative to the repository, and the four names below are relative to it.
    expect(root).toBe(join(repo, facts.store_root_path));
    expect(bindingsPath(repo)).toBe(join(root, facts.bindings_path));
    expect(baselinePath(repo)).toBe(join(root, facts.baseline_path));
    expect(suitesPath(repo)).toBe(join(root, facts.suites_path));
    expect(inspectionsPath(repo)).toBe(join(root, facts.inspections_path));
    // An anti-vacuity floor: four DISTINCT names, so a payload that answered
    // the same string four times — or an empty one — would fail here rather
    // than pass four equalities against itself.
    expect(
      new Set([
        facts.bindings_path,
        facts.baseline_path,
        facts.suites_path,
        facts.inspections_path,
      ]).size,
    ).toBe(4);
  });
});
