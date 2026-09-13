// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The oracle capture for quoin#447.
//
// This runs the RETAINED TypeScript — `src/assurance/index.ts`, and nothing
// else — over `tests/golden/cases.json` and reproduces
// `tests/golden/expected.json`. The Rust suite then asserts against that file
// and never shells out to Node again, which is the whole point: a Rust test
// that asks TypeScript whether it passed is not a port (quoin#373, FR-101
// AC-5).
//
// It imports the four retained entry points directly — `requirementOf`,
// `buildCase`, `renderCase`, `parseAssuranceArgument` — and imports nothing
// from `dist/`, nothing from `src/core/reference.ts`, and nothing from Rust.
// `src/core/reference.ts` is deliberately NOT the oracle here: its assurance
// handlers are a hand-written mirror of the Rust request schema, written so
// the difftest could compare malformed input, so capturing from it would be
// capturing the port's own schema back from a second copy of itself. The
// oracle is the module the difftest calls THROUGH that mirror.
//
// It is a CAPTURE SCRIPT, not a test, and its name says so. A golden that
// regenerates itself under the test runner is a test that cannot fail; the
// `rust/**` entry in `vite.config.ts`'s exclude list keeps `pnpm test` from
// collecting it, and the `.mts` name keeps it off the `*.test.ts` suffix, so
// neither one alone is load-bearing.
//
// Verify that the committed golden still reproduces (writes nothing):
//
//   pnpm vitest run \
//     --config rust/crates/quoin-assurance/tools/vitest.oracle.config.mts
//
// Regenerate it, deliberately:
//
//   QUOIN_ORACLE_WRITE=1 pnpm vitest run \
//     --config rust/crates/quoin-assurance/tools/vitest.oracle.config.mts
//
// Regenerating is a decision, not a convenience: a golden that is cheap to
// refresh stops being a gate the first time the implementation changes. Say in
// the commit message what behaviour changed, and refresh the hashes in
// `tests/golden/PROVENANCE.md`.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  buildCase,
  parseAssuranceArgument,
  renderCase,
  requirementOf,
} from "../../../../src/assurance/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const goldenDir = join(here, "..", "tests", "golden");
const expectedPath = join(goldenDir, "expected.json");

// Writing is opt-in. Without it this script is a reproduction check: it proves
// the committed bytes are still what the retained TypeScript produces.
const write = process.env.QUOIN_ORACLE_WRITE === "1";

type Op = "requirement_of" | "build_case" | "render_case" | "parse_argument";

interface Case {
  name: string;
  /** The requirement id(s) this input carries. */
  covers: string[];
  op: Op;
  /**
   * `refused` marks an input the RUST REQUEST SCHEMA refuses before the ported
   * library is reached. The TypeScript verdict is still captured below — often
   * the retained module accepts what serde will not — so the divergence is
   * recorded rather than hidden. `tests/golden/PROVENANCE.md` counts them.
   */
  boundary?: "refused";
  /** The literal request, for every op but the overridden arguments. */
  input?: unknown;
  /** `parse_argument` only: top-level keys set on `argument_base`. */
  overrides?: Record<string, unknown>;
  /** `parse_argument` only: top-level keys DELETED from `argument_base`. */
  remove?: string[];
  /**
   * `parse_argument` only: `argument_base` carrying one assumption whose
   * `review_by` is this text.
   *
   * Its own field because the instant grammar is the densest part of the
   * contract — an impossible day, a leap second, a rolled hour and two
   * lowercase spellings each decide acceptance — and a corpus of them reads as
   * a grammar only if each row is one line.
   */
  review_by?: string;
}

interface Corpus {
  argument_base: Record<string, unknown>;
  cases: Case[];
}

const corpus = JSON.parse(
  readFileSync(join(goldenDir, "cases.json"), "utf8"),
) as Corpus;

/**
 * The request this case hands to the retained module.
 *
 * A `parse_argument` case is a base plus an override rather than a whole
 * document because an authored argument is twelve required keys deep and a
 * corpus of forty full copies would hide the one field each case is about. An
 * entry in `remove` DELETES the key, which a merge of `overrides` could not
 * express: `null` is itself under test here — the retained code treats an
 * explicit `null` differently in `expires_at` and in `resolution_refs`, so
 * "set to null" and "delete" must stay distinguishable.
 */
function requestFor(spec: Case): unknown {
  if (spec.op !== "parse_argument" || spec.input !== undefined) {
    return spec.input;
  }
  const argument = structuredClone(corpus.argument_base);
  for (const key of spec.remove ?? []) delete argument[key];
  if (spec.review_by !== undefined) {
    argument.assumptions = [
      {
        id: "ASM-900",
        statement: "The reviewed clause set is stable.",
        owner: "release-owner",
        status: "accepted",
        review_by: spec.review_by,
      },
    ];
  }
  Object.assign(argument, spec.overrides ?? {});
  return argument;
}

/**
 * `build_case`'s request is `snake_case` and the retained `CaseInput` is
 * `camelCase`, so the capture renames — and does nothing else.
 *
 * The rename is the boundary's, not this script's invention: the payload is
 * quoin's existing contract and stays as the retained module returns it, while
 * the request is surface minted at the boundary and follows the boundary's own
 * convention. Optional keys are forwarded only when present, because the
 * retained code distinguishes an absent `evidenceIndependence` from an empty
 * one.
 */
function caseInputFor(fields: Record<string, unknown>): unknown {
  return {
    documents: fields.documents,
    obligations: fields.obligations,
    findings: fields.findings,
    ...(fields.claim_types ? { claimTypes: fields.claim_types } : {}),
    ...(fields.unreadable ? { unreadable: fields.unreadable } : {}),
    ...(fields.producer_trust ? { producerTrust: fields.producer_trust } : {}),
    ...(fields.evidence_independence
      ? { evidenceIndependence: fields.evidence_independence }
      : {}),
  };
}

/** What the retained module answers, whichever way it answers. */
function answer(spec: Case): unknown {
  const request = requestFor(spec);
  switch (spec.op) {
    case "requirement_of":
      return {
        requirement: requirementOf(
          (request as { obligation_id: string }).obligation_id,
        ),
      };
    case "build_case":
      return buildCase(
        caseInputFor(request as Record<string, unknown>) as never,
      );
    case "render_case":
      return { rendered: renderCase(request as never) };
    case "parse_argument":
      return parseAssuranceArgument(request);
  }
}

describe("quoin#447 golden oracle capture", () => {
  it("reproduces the verdicts recorded in expected.json", () => {
    const expected: unknown[] = [];
    for (const spec of corpus.cases) {
      // A throw IS a verdict here: `parseAssuranceArgument` refuses by
      // throwing, and `renderCase` throws on an absent `producerTrust`. Both
      // are behaviour the port has to reproduce, so both are recorded rather
      // than being allowed to abort the capture.
      try {
        expected.push({ name: spec.name, ok: true, value: answer(spec) });
      } catch (cause) {
        expected.push({
          name: spec.name,
          ok: false,
          // Recorded for the reader, and deliberately NOT asserted by the Rust
          // suite: quoin#373 records that verdicts are contractual and error
          // text is not, so comparing prose would make every reworded sentence
          // a false parity failure.
          error: (cause as Error).message,
        });
      }
    }

    expect(expected.length).toBe(corpus.cases.length);
    const serialised = `${JSON.stringify({ cases: expected }, null, 2)}\n`;

    if (write) {
      writeFileSync(expectedPath, serialised);
      return;
    }

    // Byte comparison, not a structural one: the committed file is what the
    // Rust suite reads, so "equivalent JSON" is not the property under test.
    expect(serialised).toBe(readFileSync(expectedPath, "utf8"));
  });
});
