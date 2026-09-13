// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The store-oracle capture for quoin#456, half 2.
//
// This runs the RETAINED TypeScript — `src/evidence/store.ts`, `trust.ts`,
// `independence.ts`, `assurance-records.ts` and `mock-inspection.ts` — over
// `tests/golden/store-cases.json` and reproduces
// `tests/golden/store-expected.json`. The Rust suite asserts against that file
// and never shells out to Node: a Rust test that asks TypeScript whether it
// passed is not a port (FR-101-AC-5).
//
// It is a CAPTURE SCRIPT, not a test, and its name says so — it does not end
// in `*.test.ts`, so `pnpm test` cannot collect it even if the `rust/**`
// exclude in `vite.config.ts` were dropped.
//
// Verify that the committed golden still reproduces (writes nothing), from the
// repository root:
//
//   pnpm vitest run \
//     --config rust/crates/quoin-evidence/tools/vitest.store-oracle.config.mts
//
// Regenerate it, deliberately:
//
//   QUOIN_ORACLE_WRITE=1 pnpm vitest run \
//     --config rust/crates/quoin-evidence/tools/vitest.store-oracle.config.mts
//
// Regenerating is a decision, not a convenience. Say in the commit message what
// behaviour changed, and refresh the hashes in
// `tests/golden/STORE-PROVENANCE.md`.

import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  affirm,
  bind,
  scanIsVacuous,
  writeBaseline,
  writeBindings,
  writeMockInspection,
  writeRun,
  writeScan,
  writeTrustDecision,
} from "../../../../src/evidence/store.js";
import { storeRoot } from "../../../../src/store/paths.js";
import {
  assessTrust,
  validateTrustDecision,
} from "../../../../src/evidence/trust.js";
import { assessIndependence } from "../../../../src/evidence/independence.js";
import {
  parseExperimentRecordInput,
  parseOperationalEvidenceRecordInput,
} from "../../../../src/evidence/assurance-records.js";
import { inspectMockInjections } from "../../../../src/evidence/mock-inspection.js";

const here = dirname(fileURLToPath(import.meta.url));
const goldenDir = join(here, "..", "tests", "golden");
const expectedPath = join(goldenDir, "store-expected.json");

const write = process.env.QUOIN_ORACLE_WRITE === "1";

interface Case {
  name: string;
  kind: string;
  input: Record<string, unknown>;
}

/* eslint-disable @typescript-eslint/no-explicit-any */

function runCase(spec: Case): unknown {
  const input = spec.input as any;
  switch (spec.kind) {
    case "trust": {
      const decision = validateTrustDecision(input);
      return assessTrust(decision);
    }
    case "independence":
      return assessIndependence(
        input.profile,
        input.requirement,
        input.bindings,
      );
    case "bind":
      return bind(input.existing, input.next);
    case "affirm":
      return affirm(
        input.existing,
        input.obligation,
        input.currentHash,
        input.who,
        input.commit,
        input.note,
        input.suite,
      );
    case "canonical":
      return writeInTemporaryRepository(input.record, input.value);
    case "vacuity":
      return { vacuous: scanIsVacuous(input) };
    case "assurance":
      return input.record === "experiment"
        ? parseExperimentRecordInput(input.value)
        : parseOperationalEvidenceRecordInput(input.value);
    case "mock":
      return inspectInTemporaryRepository(input);
    default:
      throw new Error(`case ${spec.name} names unknown kind ${spec.kind}`);
  }
}

// Capturing the bytes the retained writer actually puts on disk, rather than
// canonicalising a hand-built object: NFR-025 is a statement about the store's
// files, so the oracle has to be the file.
function writeInTemporaryRepository(record: string, value: any): unknown {
  const repo = mkdtempSync(join(tmpdir(), "quoin-456-store-"));
  try {
    switch (record) {
      case "run":
        writeRun(repo, value);
        break;
      case "scan":
        writeScan(repo, value);
        break;
      case "mockInspection":
        writeMockInspection(repo, value);
        break;
      case "bindings":
        writeBindings(repo, value);
        break;
      case "baseline":
        writeBaseline(repo, value);
        break;
      case "trustDecision":
        writeTrustDecision(repo, value);
        break;
      default:
        throw new Error(`unknown canonical record ${record}`);
    }
    const root = storeRoot(repo);
    const written = listFilesUnder(root).sort();
    return {
      record,
      files: written.map((path) => ({
        path: path
          .slice(root.length + 1)
          .split(sep)
          .join("/"),
        text: readFileSync(path, "utf8"),
      })),
    };
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
}

function listFilesUnder(directory: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const full = join(directory, entry.name);
    if (entry.isDirectory()) out.push(...listFilesUnder(full));
    else out.push(full);
  }
  return out;
}

function inspectInTemporaryRepository(input: {
  suite: string;
  files: Record<string, string>;
}): unknown {
  const root = mkdtempSync(join(tmpdir(), "quoin-456-mock-"));
  try {
    for (const [relative, source] of Object.entries(input.files)) {
      const target = join(root, relative);
      mkdirSync(dirname(target), { recursive: true });
      writeFileSync(target, source);
    }
    return inspectMockInjections(root, input.suite);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

describe("quoin#456 store golden oracle capture", () => {
  it("reproduces the results recorded in store-expected.json", () => {
    const corpus = JSON.parse(
      readFileSync(join(goldenDir, "store-cases.json"), "utf8"),
    ) as { cases: Case[] };

    const expected: unknown[] = [];
    const kinds = new Set<string>();
    let accepted = 0;
    let refused = 0;
    for (const spec of corpus.cases) {
      kinds.add(spec.kind);
      try {
        const ok = runCase(spec);
        accepted += 1;
        expected.push({ name: spec.name, kind: spec.kind, ok });
      } catch (error) {
        refused += 1;
        expected.push({
          name: spec.name,
          kind: spec.kind,
          err: (error as Error).message,
        });
      }
    }

    // Anti-vacuity: a corpus that only accepts, or only refuses, carries no
    // refusal criterion at all. Both populations must be non-empty, and every
    // behaviour family this oracle covers must be present.
    expect(accepted).toBeGreaterThan(0);
    expect(refused).toBeGreaterThan(0);
    expect([...kinds].sort()).toStrictEqual([
      "affirm",
      "assurance",
      "bind",
      "canonical",
      "independence",
      "mock",
      "trust",
      "vacuity",
    ]);
    expect(expected.length).toBe(corpus.cases.length);

    const serialised = `${JSON.stringify({ cases: expected }, null, 2)}\n`;

    if (write) {
      writeFileSync(expectedPath, serialised);
      return;
    }

    expect(readFileSync(expectedPath, "utf8")).toBe(serialised);
  });
});
