/** FR-047 — profile-selected evidence independence (TC-303..TC-309). */

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { buildCase, renderCase } from "../src/assurance/index.js";
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
    independencePolicy,
  };
}

describe("TC-303 strict lineage and policy boundaries", () => {
  // Trace: FR-047-AC-1
  // Trace: FR-047-AC-2
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

  // Trace: FR-047-AC-2
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

describe("TC-304..TC-307 relationship independence", () => {
  // Trace: FR-047-AC-3
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

  // Trace: FR-047-AC-4
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

  // Trace: FR-047-AC-5
  it("does nothing when no profile requests independence", () => {
    const report = audit(auditInput([binding("SUITE-A", lineageA)]));
    expect(report).toEqual({ findings: [], healthy: [OBLIGATION] });
  });

  // Trace: FR-047-AC-6
  it("opens the obligation when selected separation is absent and clears with diverse lineage", () => {
    const shared = { ...lineageB, actor: lineageA.actor };
    const insufficient = audit(
      auditInput(
        [binding("SUITE-A", lineageA), binding("SUITE-B", shared)],
        policy(),
      ),
    );
    expect(insufficient.findings.map((item) => item.kind)).toEqual([
      "insufficient-independence",
    ]);
    expect(insufficient.independence?.[0].dimensions[0]).toMatchObject({
      dimension: "actor",
      values: ["reviewer-a"],
    });

    const satisfied = audit(
      auditInput(
        [binding("SUITE-A", lineageA), binding("SUITE-B", lineageB)],
        policy(),
      ),
    );
    expect(satisfied.findings).toEqual([]);
    expect(satisfied.healthy).toEqual([OBLIGATION]);
    expect(satisfied.independence?.[0].status).toBe("satisfied");
  });
});

describe("TC-308 lineage persistence and TC-309 assurance context", () => {
  // Trace: FR-047-AC-7
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
    expect(command).toContain("readEvidenceLineage(flags.lineage)");
  });

  // Trace: FR-047-AC-8
  it("renders satisfied dimensions as context without manufacturing claim support", () => {
    const assessment = assessIndependence("AP-001", policy().requirements[0], [
      binding("SUITE-A", lineageA),
      binding("SUITE-B", lineageB),
    ]);
    const assurance = buildCase({
      documents: [],
      obligations: [],
      findings: [],
      evidenceIndependence: [assessment],
    });
    expect(assurance.claims).toEqual([]);
    const rendered = renderCase(assurance);
    expect(rendered).toContain("## Evidence independence");
    expect(rendered).toContain("AP-001 / IR-1 / FR-001-AC-1");
    expect(rendered).toContain("implementation-toolchain");
    expect(rendered).toContain("do not make a claim supported");
  });

  // Trace: FR-047-AC-7
  it("does not change the assurance-case JSON shape when no policy is selected", () => {
    const assurance = buildCase({
      documents: [],
      obligations: [],
      findings: [],
    });
    expect(assurance).not.toHaveProperty("evidenceIndependence");
  });
});
