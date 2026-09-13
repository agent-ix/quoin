/** FR-094 — profile-selected evidence independence (TC-303..TC-309). */

import {
  accessSync,
  constants,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { buildCase, renderCase } from "../src/core/assurance.js";
import { auditInputs, record } from "../src/core/evidence.js";
import { audit } from "../src/auditor/index.js";
import {
  assessIndependence,
  bind,
  readBindings,
  readEvidenceLineage,
  readIndependencePolicy,
  recordRun,
  requireKnownPolicyObligations,
  validateEvidenceLineage,
  validateIndependencePolicy,
  type Binding,
  type EvidenceLineage,
  type IndependencePolicy,
} from "../src/evidence/index.js";

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

describe("strict lineage and policy boundaries", () => {
  // Trace: FR-094-AC-1, FR-094-AC-2
  it("accepts stated dimensions and rejects empty or invented dimensions", () => {
    expect(validateEvidenceLineage(lineageA)).toEqual(lineageA);
    expect(() => validateEvidenceLineage({})).toThrow(/at least one/);
    expect(() => validateEvidenceLineage({ actor: "   " })).toThrow(
      /too small|at least/i,
    );
    expect(validateEvidenceLineage({ actor: " reviewer-a " })).toEqual({
      actor: "reviewer-a",
    });
    expect(() => validateEvidenceLineage({ vendorBadge: "approved" })).toThrow(
      /unrecognized key/i,
    );
    expect(validateIndependencePolicy(policy())).toEqual(policy());
    expect(() =>
      validateIndependencePolicy({
        ...policy(),
        requirements: [
          { ...policy().requirements[0], dimensions: ["actor", "actor"] },
        ],
      }),
    ).toThrow(/duplicate dimensions/);

    const duplicated = policy();
    duplicated.requirements.push({
      ...duplicated.requirements[0],
      id: "IR-2",
    });
    expect(() => validateIndependencePolicy(duplicated)).toThrow(
      /duplicates another obligation requirement/,
    );

    duplicated.requirements[1] = {
      ...duplicated.requirements[1],
      id: "IR-1",
      obligation: "FR-002-AC-1",
    };
    expect(() => validateIndependencePolicy(duplicated)).toThrow(
      /duplicates another requirement id/,
    );
  });

  // Trace: FR-094-AC-2
  it("reads a normalized profile projection and refuses stale obligation ids", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-independence-"));
    const path = join(root, "policy.json");
    writeFileSync(path, JSON.stringify(policy()));
    const selected = readIndependencePolicy(path);
    expect(() =>
      requireKnownPolicyObligations(selected, [OBLIGATION]),
    ).not.toThrow();
    expect(() =>
      requireKnownPolicyObligations(selected, ["FR-999-AC-1"]),
    ).toThrow(OBLIGATION);

    const twoStale = policy();
    twoStale.requirements = [
      { ...twoStale.requirements[0], obligation: "FR-003-AC-1" },
      {
        ...twoStale.requirements[0],
        id: "IR-2",
        obligation: "FR-002-AC-1",
      },
    ];
    expect(() => requireKnownPolicyObligations(twoStale, [])).toThrow(
      /FR-002-AC-1, FR-003-AC-1/,
    );

    const lineagePath = join(root, "lineage.json");
    writeFileSync(lineagePath, JSON.stringify(lineageA));
    expect(readEvidenceLineage(lineagePath)).toEqual(lineageA);

    writeFileSync(lineagePath, "not-json");
    expect(() => readEvidenceLineage(lineagePath)).toThrow(/not readable JSON/);
  });
});

describe("relationship independence", () => {
  // Trace: FR-094-AC-3, FR-094-CON-2
  it("requires two distinct evidence relationships that differ on every selected dimension", () => {
    const one = assessIndependence("AP-001", policy().requirements[0], [
      binding("SUITE-A", lineageA),
    ]);
    expect(one.status).toBe("insufficient");

    const two = assessIndependence("AP-001", policy().requirements[0], [
      binding("SUITE-A", lineageA),
      binding("SUITE-B", lineageB),
    ]);
    expect(two.status).toBe("satisfied");
    expect(two.satisfiedBy).toEqual(["SUITE-A", "SUITE-B"]);

    const sameSuite = assessIndependence("AP-001", policy().requirements[0], [
      binding("SUITE-A", lineageA),
      binding("SUITE-A", lineageB),
    ]);
    expect(sameSuite.status).toBe("insufficient");

    const actorRequirement = policy(["actor"]).requirements[0];
    const missingFirst = assessIndependence("AP-001", actorRequirement, [
      binding("SUITE-A", { technique: "inspection" }),
      binding("SUITE-B", lineageB),
    ]);
    const missingSecond = assessIndependence("AP-001", actorRequirement, [
      binding("SUITE-A", lineageA),
      binding("SUITE-B", { technique: "inspection" }),
    ]);
    const sharedActor = assessIndependence("AP-001", actorRequirement, [
      binding("SUITE-A", lineageA),
      binding("SUITE-B", { ...lineageB, actor: lineageA.actor }),
    ]);
    expect([
      missingFirst.status,
      missingSecond.status,
      sharedActor.status,
    ]).toEqual(["insufficient", "insufficient", "insufficient"]);

    const allDimensions = assessIndependence(
      "AP-001",
      policy([
        "actor",
        "implementation-toolchain",
        "technique",
        "data-source",
        "review-path",
      ]).requirements[0],
      [binding("SUITE-A", lineageA), binding("SUITE-B", lineageB)],
    );
    expect(allDimensions.status).toBe("satisfied");
  });

  // Trace: FR-094-AC-4
  it("keeps common actor and missing parser lineage visible", () => {
    const sharedActor = { ...lineageB, actor: lineageA.actor };
    const result = assessIndependence("AP-001", policy().requirements[0], [
      binding("SUITE-A", lineageA),
      binding("SUITE-B", sharedActor),
      binding("SUITE-C", { actor: "reviewer-c" }),
    ]);
    expect(result.status).toBe("insufficient");
    expect(result.summary).toContain("actor: 2 distinct");
    expect(result.summary).toContain("implementation-toolchain: 2 distinct");
    expect(result.summary).toContain("missing on SUITE-C");

    const sharedCorpus = assessIndependence(
      "AP-001",
      policy(["data-source"]).requirements[0],
      [
        binding("SUITE-A", lineageA),
        binding("SUITE-B", { ...lineageB, dataSource: lineageA.dataSource }),
      ],
    );
    expect(sharedCorpus.status).toBe("insufficient");
    expect(sharedCorpus.dimensions[0].values).toEqual(["corpus-a"]);
  });

  // Trace: FR-094-AC-5
  it("does nothing when no profile requests independence", () => {
    const report = audit(auditInput([binding("SUITE-A", lineageA)]));
    expect(report).toEqual({
      findings: [],
      healthy: [OBLIGATION],
      unevaluated: [],
    });
  });
});

describe("lineage persistence", () => {
  // Trace: FR-094-AC-7
  it("records lineage on the binding and clears old lineage when a later run omits it", () => {
    const repo = mkdtempSync(join(tmpdir(), "quoin-lineage-"));
    recordRun({
      repo,
      suite: "SUITE-A",
      commit: COMMIT,
      tool: "fixture",
      timestamp: "2026-08-21T20:00:00Z",
      lineage: lineageA,
      entries: [
        { symbol: "tests::a", outcome: "pass", traceIds: [OBLIGATION] },
      ],
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
    expect(readBindings(repo).bindings[0].lineage).toEqual(lineageA);

    const updated = bind(readBindings(repo).bindings, {
      ...binding("SUITE-A"),
      commit: "c".repeat(40),
    });
    expect(updated.bindings[0].lineage).toBeUndefined();

    const command = readFileSync(
      join(
        dirname(fileURLToPath(import.meta.url)),
        "..",
        "src/commands/evidence/record.ts",
      ),
      "utf8",
    );
    expect(command).toContain("lineage: Flags.string");
    // The flag still reaches the validator, which is now `quoin-evidence`
    // behind `evidence.parse_lineage` (quoin#458). Asserted on the command
    // source because what this criterion is about is the WIRING: a `--lineage`
    // the command accepts and never passes on would leave every other
    // assertion in this file green.
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

describe.skipIf(coreBinary() === null)("profile-selected independence", () => {
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

  /** The auditor over a real store, with the policy selected. */
  function auditRecorded(lineages: Record<string, EvidenceLineage>) {
    const repo = mkdtempSync(join(tmpdir(), "quoin-independence-store-"));
    for (const [suite, lineage] of Object.entries(lineages)) {
      recorded(repo, suite, lineage);
    }
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

  // Trace: FR-094-AC-6
  it("opens the obligation when selected separation is absent and clears with diverse lineage", () => {
    const insufficient = auditRecorded({
      "SUITE-A": lineageA,
      "SUITE-B": { ...lineageB, actor: lineageA.actor },
    });
    expect(insufficient.findings.map((item) => item.kind)).toEqual([
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
    expect(satisfied.findings).toEqual([]);
    expect(satisfied.healthy).toEqual([OBLIGATION]);
    expect(satisfied.independence?.[0].status).toBe("satisfied");
  });
});

describe.skipIf(coreBinary() === null)("TC-309 assurance context", () => {
  // Trace: FR-094-AC-8
  it("renders satisfied dimensions as context without manufacturing claim support", () => {
    const assessment = assessIndependence("AP-001", policy().requirements[0], [
      binding("SUITE-A", lineageA),
      binding("SUITE-B", lineageB),
    ]);
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
