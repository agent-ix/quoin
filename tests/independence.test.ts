/**
 * FR-094 — profile-selected evidence independence (TC-303..TC-309).
 *
 * Lineage and policy validation, and the relationship assessment itself, are
 * `quoin-evidence`'s and are asserted there (quoin#458). What stays here is
 * what only this tree carries: the pure auditor's silence when no profile asks
 * for independence, the command flag that reaches the validator, and the
 * profile-selected path end to end through the real binary.
 */

import { accessSync, constants, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { buildCase, renderCase } from "../src/core/assurance.js";
import { auditInputs, record } from "../src/core/evidence.js";
import { audit } from "../src/core/auditor.js";
import type {
  Binding,
  EvidenceLineage,
  IndependencePolicy,
} from "../src/core/evidence.js";

const COMMIT = "a".repeat(40);
const HASH = "b".repeat(64);
const OBLIGATION = "FR-001-AC-1";

const lineageA: EvidenceLineage = {
  actor: "reviewer-a",
  implementationToolchain: "engine-a/parser-a",
  technique: "example-based-test",
  dataSource: "corpus-a",
  reviewPath: "pull-request-a",
};
const lineageB: EvidenceLineage = {
  actor: "reviewer-b",
  implementationToolchain: "engine-b/parser-b",
  technique: "property-test",
  dataSource: "corpus-b",
  reviewPath: "release-review-b",
};

function binding(suite: string, lineage?: EvidenceLineage): Binding {
  return {
    obligation: OBLIGATION,
    statementHashAtBinding: HASH,
    suite,
    commit: COMMIT,
    symbols: [`tests::${suite.toLowerCase()}`],
    lineage,
  };
}

function policy(
  dimensions: IndependencePolicy["requirements"][number]["dimensions"] = [
    "actor",
    "implementation-toolchain",
  ],
): IndependencePolicy {
  return {
    schemaVersion: 1,
    profile: "AP-001",
    requirements: [
      {
        id: "IR-1",
        obligation: OBLIGATION,
        dimensions,
        rationale: "A common parser failure could conceal the defect.",
      },
    ],
  };
}

function auditInput(
  bindings: Binding[],
  independencePolicy?: IndependencePolicy,
) {
  return {
    obligations: [
      {
        source: "acceptance-criterion" as const,
        id: OBLIGATION,
        document: "spec/functional/FR-001.md",
        statement: "The parser rejects malformed input.",
        statement_hash: HASH,
      },
    ],
    bindings,
    runs: bindings.map((item) => ({
      schemaVersion: 1,
      suite: item.suite,
      commit: COMMIT,
      tool: item.lineage?.implementationToolchain ?? "unknown",
      timestamp: "2026-08-21T20:00:00Z",
      entries: item.symbols.map((symbol) => ({
        symbol,
        outcome: "pass" as const,
      })),
    })),
    // #204's mocked-confirmation check reports an obligation as unevaluated
    // until each suite has a current mock inspection. Declare them, as
    // auditor.test.ts and finding-record.test.ts do, so this file exercises
    // independence rather than the missing-inspection path.
    mockInspectionSuites: bindings.map((item) => item.suite),
    independencePolicy,
  };
}

describe("the auditor without a policy", () => {
  // Trace: FR-094-AC-5
  it("does nothing when no profile requests independence", () => {
    const answer = audit(auditInput([binding("SUITE-A", lineageA)]));
    expect(answer.report).toEqual({
      findings: [],
      healthy: [OBLIGATION],
      unevaluated: [],
    });
    expect(answer.independence).toBeUndefined();
  });
});

describe("lineage persistence", () => {
  // Trace: FR-094-AC-7
  it("passes the --lineage flag on to the validator", () => {
    const command = readFileSync(
      join(
        dirname(fileURLToPath(import.meta.url)),
        "..",
        "src/commands/evidence/record.ts",
      ),
      "utf8",
    );
    expect(command).toContain("lineage: Flags.string");
    // What this criterion is about here is the WIRING: the validator itself is
    // `quoin-evidence` behind `evidence.parse_lineage` (quoin#458, traced on
    // its own test), and a `--lineage` the command accepts and never passes on
    // would leave every other assertion in this file green.
    expect(command).toContain(
      'parseLineage(readFileSync(flags.lineage, "utf8"))',
    );
  });
});

/**
 * The assurance case is built by `quoin-core` (quoin#447) and the evidence
 * store by `quoin-evidence` (quoin#458), so the blocks below need the binary.
 * They follow `tests/core-exec-e2e.test.ts`: `QUOIN_CORE` names an executable
 * or the block skips, and `make rust-e2e` is the lane that sets it. Everything
 * above runs under a plain `vitest run`.
 */
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

/**
 * One suite recorded into a real store, with its lineage.
 *
 * Through `evidence.record` rather than by writing `bindings.json`: the
 * binding, its lineage and the run behind it are what the assessment reads,
 * and a test that wrote the binding file itself would be asserting against
 * its own fixture rather than against what recording a run produces.
 */
function recorded(repo: string, suite: string, lineage: EvidenceLineage) {
  record({
    repo,
    suite,
    commit: COMMIT,
    tool: lineage.implementationToolchain ?? "fixture",
    timestamp: "2026-08-21T20:00:00Z",
    adapter: "entries",
    lineage,
    results: JSON.stringify({
      entries: [
        {
          symbol: `tests::${suite.toLowerCase()}`,
          outcome: "pass",
          traceIds: [OBLIGATION],
        },
      ],
    }),
    obligations: [
      {
        source: "acceptance-criterion",
        id: OBLIGATION,
        document: "spec/functional/FR-001.md",
        statement: "The parser rejects malformed input.",
        statement_hash: HASH,
      },
    ],
  });
}

/** A store with one recorded suite per named lineage. */
function recordedStore(lineages: Record<string, EvidenceLineage>): string {
  const repo = mkdtempSync(join(tmpdir(), "quoin-independence-store-"));
  for (const [suite, lineage] of Object.entries(lineages)) {
    recorded(repo, suite, lineage);
  }
  return repo;
}

/** The auditor over a real store, with the policy selected. */
function auditRecorded(lineages: Record<string, EvidenceLineage>) {
  const repo = recordedStore(lineages);
  const selected = policy();
  const inputs = auditInputs(repo, undefined, selected);
  return audit({
    obligations: [
      {
        source: "acceptance-criterion" as const,
        id: OBLIGATION,
        document: "spec/functional/FR-001.md",
        statement: "The parser rejects malformed input.",
        statement_hash: HASH,
      },
    ],
    bindings: inputs.bindings,
    runs: inputs.runs,
    scans: inputs.scans,
    independence: inputs.independence,
    // As above: #204's check reports the obligation unevaluated until each
    // suite carries a current mock inspection, and this file is about
    // independence.
    mockInspectionSuites: inputs.bindings.map((item) => item.suite),
    independencePolicy: selected,
  });
}

describe.skipIf(coreBinary() === null)("profile-selected independence", () => {
  // Trace: FR-094-AC-6
  it("opens the obligation when selected separation is absent and clears with diverse lineage", () => {
    const insufficient = auditRecorded({
      "SUITE-A": lineageA,
      "SUITE-B": { ...lineageB, actor: lineageA.actor },
    });
    expect(insufficient.report.findings.map((item) => item.kind)).toEqual([
      "insufficient-independence",
    ]);
    expect(insufficient.independence?.[0].dimensions[0]).toMatchObject({
      dimension: "actor",
      values: ["reviewer-a"],
    });

    const satisfied = auditRecorded({
      "SUITE-A": lineageA,
      "SUITE-B": lineageB,
    });
    expect(satisfied.report.findings).toEqual([]);
    expect(satisfied.report.healthy).toEqual([OBLIGATION]);
    expect(satisfied.independence?.[0].status).toBe("satisfied");
  });
});

describe.skipIf(coreBinary() === null)("TC-309 assurance context", () => {
  // Trace: FR-094-AC-8
  it("renders satisfied dimensions as context without manufacturing claim support", () => {
    // The assessment comes back from the boundary over a real store rather
    // than being hand-written here: a fixture the test wrote would render
    // whatever this test decided the renderer should see.
    const assessment = auditInputs(
      recordedStore({ "SUITE-A": lineageA, "SUITE-B": lineageB }),
      undefined,
      policy(),
    ).independence[0];
    expect(assessment.status).toBe("satisfied");
    const assurance = buildCase({
      documents: [],
      obligations: [],
      findings: [],
      evidence_independence: [assessment],
    }) as { claims: unknown[] };
    expect(assurance.claims).toEqual([]);
    const rendered = renderCase(assurance);
    expect(rendered).toContain("## Evidence independence");
    expect(rendered).toContain("AP-001 / IR-1 / FR-001-AC-1");
    expect(rendered).toContain("implementation-toolchain");
    expect(rendered).toContain("do not make a claim supported");
  });

  // Trace: FR-094-AC-7
  it("does not change the assurance-case JSON shape when no policy is selected", () => {
    const assurance = buildCase({
      documents: [],
      obligations: [],
      findings: [],
    });
    expect(assurance).not.toHaveProperty("evidenceIndependence");
  });
});
