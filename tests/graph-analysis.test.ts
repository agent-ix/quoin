/** FR-062 — read-only evidence graph projections (TC-1249..TC-1260). */

import fc from "fast-check";
import { describe, expect, it } from "vitest";

import type { Binding } from "../src/evidence/index.js";
import {
  DEFAULT_RELATION_KINDS,
  analyzeChangeImpact,
  analyzeChurn,
  analyzeFanOut,
  parseAcceptedAssurancePremises,
  parseAuditEnvelope,
  renderGraphAnalysis,
  renderGraphAnalysisJson,
  validateAcceptedAssurancePremises,
  validateAuditIdentity,
  type AuditEnvelope,
  type GraphAnalysisInput,
} from "../src/graph-analysis/index.js";
import type {
  AcceptedAssurancePremises,
  AssuranceCorpusRelation,
  AssuranceExport,
} from "../src/quire/index.js";
import { parseAssurance } from "../src/quire/index.js";

const digest = "a".repeat(64);
const revision = "b".repeat(40);
const source = { repository: "agent-ix/example", revision };
const modules = [
  {
    name: "example",
    version: "1.0.0",
    schemas: [{ archetype: "FR", schema_digest: "c".repeat(64) }],
  },
];

function locator(path: string, line = 1) {
  return { path, line, digest };
}

function artifact(id: string) {
  return {
    id,
    artifact_type: "FR",
    locator: locator(`spec/${id}.md`),
  };
}

function obligation(id: string, owner: string) {
  return {
    source: "acceptance-criterion",
    id,
    document: `spec/${owner}.md`,
    statement: `${id} statement`,
    statement_hash: digest,
    target_ids: [`TC-${id}`],
    locator: locator(`spec/${owner}.md`, 20),
  };
}

function relation(
  sourceId: string,
  target: string,
  edge_type: string,
): AssuranceCorpusRelation {
  return {
    kind: "corpus",
    source: sourceId,
    target,
    edge_type,
    resolution: "resolved",
    locator: locator(`spec/${sourceId}.md`, 5),
    freshness: "not_applicable",
  };
}

function binding(
  obligationId: string,
  suite: string,
  affirmations?: Binding["affirmations"],
): Binding {
  return {
    obligation: obligationId,
    statementHashAtBinding: digest,
    suite,
    commit: revision,
    symbols: ["test"],
    ...(affirmations === undefined ? {} : { affirmations }),
  };
}

function exportFixture(
  overrides: Partial<AssuranceExport> = {},
): AssuranceExport {
  const artifacts = [
    artifact("FR-001"),
    artifact("FR-002"),
    artifact("FR-003"),
    artifact("FR-004"),
  ];
  const obligations = [
    obligation("FR-001-AC-1", "FR-001"),
    obligation("FR-002-AC-1", "FR-002"),
    obligation("FR-003-AC-1", "FR-003"),
    obligation("FR-004-AC-1", "FR-004"),
  ];
  return {
    format: "quire-assurance",
    format_version: 1,
    source,
    modules,
    artifacts,
    obligations,
    symbols: [],
    relation_kinds: DEFAULT_RELATION_KINDS.map((kind) => ({
      kind,
      availability: "available" as const,
      sources: ["module_vocabulary" as const],
    })),
    relations: [],
    relation_observations: [],
    ...overrides,
  };
}

function premises(): AcceptedAssurancePremises {
  return { format: "quire-assurance", format_version: 1, modules };
}

function audit(
  healthy = ["FR-001-AC-1", "FR-002-AC-1", "FR-003-AC-1", "FR-004-AC-1"],
): AuditEnvelope {
  return {
    format: "quoin-audit-envelope",
    format_version: 1,
    source,
    export: premises(),
    report: { findings: [], healthy, unevaluated: [] },
  };
}

function input(
  exportValue = exportFixture(),
  bindings: Binding[] = [],
  auditValue = audit(),
): GraphAnalysisInput {
  return {
    assurance: exportValue,
    premises: premises(),
    audit: auditValue,
    bindings: { availability: "available", bindings },
  };
}

describe("FR-062 fan-out", () => {
  // Trace: FR-062-AC-1
  it("TC-1249 counts a distinct live obligation once under arbitrary input order", () => {
    fc.assert(
      fc.property(fc.boolean(), fc.boolean(), (reverse, addDuplicate) => {
        const bindings = [
          binding("FR-002-AC-1", "suite-b"),
          binding("FR-001-AC-1", "suite-b"),
          binding("FR-003-AC-1", "suite-a"),
        ];
        if (addDuplicate) bindings.push(binding("FR-001-AC-1", "suite-b"));
        if (reverse) bindings.reverse();
        expect(analyzeFanOut(input(exportFixture(), bindings)).rows).toEqual([
          {
            suite: "suite-a",
            obligations: [
              { obligation: "FR-003-AC-1", requirements: ["FR-003"] },
            ],
            obligationCount: 1,
            unresolvedBindings: [],
          },
          {
            suite: "suite-b",
            obligations: [
              { obligation: "FR-001-AC-1", requirements: ["FR-001"] },
              { obligation: "FR-002-AC-1", requirements: ["FR-002"] },
            ],
            obligationCount: 2,
            unresolvedBindings: [],
          },
        ]);
      }),
    );
  });

  // Trace: FR-062-AC-2
  it("TC-1250 names an unresolved binding without counting it", () => {
    const report = analyzeFanOut(
      input(exportFixture(), [
        binding("FR-001-AC-1", "unit"),
        binding("FR-999-AC-1", "unit"),
      ]),
    );
    expect(report.rows[0]).toMatchObject({
      obligationCount: 1,
      unresolvedBindings: ["FR-999-AC-1"],
    });
    expect(report.state).toBe("incomplete");
    expect(report.gaps).toContainEqual(
      expect.objectContaining({ kind: "unresolved-binding" }),
    );
  });
});

describe("FR-062 change impact", () => {
  const graphRelations = [
    relation("FR-002", "FR-001", "depends_on"),
    relation("FR-003", "FR-001", "derives_from"),
    relation("FR-004", "FR-002", "requires"),
    relation("FR-004", "FR-003", "refines"),
    relation("FR-001", "FR-004", "depends_on"),
  ];
  const selected = ["depends_on", "derives_from", "requires", "refines"];
  const bindings = [
    binding("FR-001-AC-1", "unit"),
    binding("FR-002-AC-1", "integration"),
    binding("FR-004-AC-1", "integration"),
  ];

  // Trace: FR-062-AC-3
  it("TC-1251 closes cycles and selects the lexicographically first shortest path", () => {
    fc.assert(
      fc.property(
        fc.shuffledSubarray(graphRelations, {
          minLength: graphRelations.length,
          maxLength: graphRelations.length,
        }),
        (relations) => {
          const report = analyzeChangeImpact(
            input(exportFixture({ relations }), bindings),
            ["FR-001"],
            selected,
          );
          expect(
            report.rows.map(({ requirement, depth }) => ({
              requirement,
              depth,
            })),
          ).toEqual([
            { requirement: "FR-001", depth: 0 },
            { requirement: "FR-002", depth: 1 },
            { requirement: "FR-003", depth: 1 },
            { requirement: "FR-004", depth: 2 },
          ]);
          expect(
            report.rows.find(({ requirement }) => requirement === "FR-004")
              ?.path,
          ).toEqual({
            seed: "FR-001",
            edges: [
              {
                source: "FR-002",
                target: "FR-001",
                relationship: "depends_on",
              },
              { source: "FR-004", target: "FR-002", relationship: "requires" },
            ],
          });
          expect(
            analyzeChangeImpact(input(exportFixture({ relations }), bindings), [
              "FR-001",
            ]).relationKinds,
          ).toEqual([...DEFAULT_RELATION_KINDS]);
        },
      ),
    );
  });

  // Trace: FR-062-AC-4
  it("TC-1252 joins every reached requirement and isolates an unknown seed", () => {
    const report = analyzeChangeImpact(
      input(exportFixture({ relations: graphRelations }), bindings),
      ["FR-404", "FR-001"],
      selected,
    );
    expect(
      report.rows.some(({ requirement }) => requirement === "FR-404"),
    ).toBe(false);
    expect(
      report.rows.find(({ requirement }) => requirement === "FR-002")
        ?.obligations[0],
    ).toMatchObject({
      obligation: "FR-002-AC-1",
      requirements: ["FR-002"],
      bindings: [{ suite: "integration" }],
    });
    expect(report.gaps).toContainEqual(
      expect.objectContaining({
        kind: "unknown-requirement",
        subject: "FR-404",
      }),
    );
  });

  // Trace: FR-062-AC-5
  it("TC-1253 copies the existing auditor verdict apart from exposure", () => {
    const finding = {
      kind: "stale-evidence" as const,
      obligation: "FR-002-AC-1",
      severity: "medium" as const,
      summary: "The retained run is behind the accepted revision.",
    };
    const auditValue = audit(["FR-001-AC-1", "FR-003-AC-1", "FR-004-AC-1"]);
    auditValue.report.findings.push(finding);
    const report = analyzeChangeImpact(
      input(exportFixture({ relations: graphRelations }), bindings, auditValue),
      ["FR-001"],
      selected,
    );
    const verdict = report.rows.find(
      ({ requirement }) => requirement === "FR-002",
    )?.obligations[0]?.bindings[0]?.auditorVerdict;
    expect(verdict).toEqual({
      findings: [finding],
      healthy: [],
      unevaluated: [],
    });
    expect(JSON.stringify(report)).toContain("stale-evidence");
    expect(JSON.stringify(report)).not.toContain("suspectObligations");
  });
});

describe("FR-062 churn, premises, and rendering", () => {
  const event = { who: "@reviewer", commit: revision, note: "still valid" };

  // Trace: FR-062-AC-6
  it("TC-1254 deduplicates one affirmation while retaining all affected suites", () => {
    fc.assert(
      fc.property(fc.boolean(), (reverse) => {
        const bindings = [
          binding("FR-001-AC-1", "unit", [event]),
          binding("FR-001-AC-1", "integration", [event]),
        ];
        if (reverse) bindings.reverse();
        const row = analyzeChurn(input(exportFixture(), bindings)).rows[0];
        expect(row.eventCount).toBe(1);
        expect(row.events[0]).toEqual({
          ...event,
          suites: ["integration", "unit"],
        });
      }),
    );
  });

  // Trace: FR-062-AC-7
  it("TC-1255 retains zero-event rows and gaps orphan affirmation history", () => {
    const report = analyzeChurn(
      input(exportFixture(), [
        binding("FR-002-AC-1", "unit", [event]),
        binding("FR-999-AC-1", "old", [event]),
      ]),
    );
    expect(
      report.rows.map(({ obligation, eventCount }) => ({
        obligation,
        eventCount,
      })),
    ).toEqual([
      { obligation: "FR-002-AC-1", eventCount: 1 },
      { obligation: "FR-001-AC-1", eventCount: 0 },
      { obligation: "FR-003-AC-1", eventCount: 0 },
      { obligation: "FR-004-AC-1", eventCount: 0 },
    ]);
    expect(report.gaps).toContainEqual(
      expect.objectContaining({ subject: "old:FR-999-AC-1" }),
    );
  });

  // Trace: FR-062-AC-8
  it("TC-1256 preserves source and accepted premises across every view", () => {
    const value = input(exportFixture(), [binding("FR-001-AC-1", "unit")]);
    const reports = [
      analyzeFanOut(value),
      analyzeChurn(value),
      analyzeChangeImpact(value, ["FR-001"]),
    ];
    for (const report of reports) {
      expect(report.source).toEqual(source);
      expect(report.premises).toEqual(premises());
      expect(report.export).toEqual({
        format: "quire-assurance",
        format_version: 1,
      });
    }
  });

  // Trace: FR-062-AC-9
  it("TC-1257 rejects invalid premises/audit identity and distinguishes unavailable bindings", () => {
    expect(parseAcceptedAssurancePremises("not json").ok).toBe(false);
    expect(parseAuditEnvelope(JSON.stringify({})).ok).toBe(false);
    expect(parseAssurance(JSON.stringify({})).ok).toBe(false);
    const exported = exportFixture();
    expect(
      validateAcceptedAssurancePremises(exported, {
        ...premises(),
        modules: [],
      })?.input,
    ).toBe("premises");
    expect(
      validateAuditIdentity(
        { ...audit(), source: { ...source, revision: "d".repeat(40) } },
        exported,
      )?.input,
    ).toBe("audit");
    const absent = analyzeFanOut({
      ...input(),
      bindings: { availability: "absent", reason: "missing" },
    });
    const unreadable = analyzeFanOut({
      ...input(),
      bindings: { availability: "unreadable", reason: "bad JSON" },
    });
    const empty = analyzeFanOut(input());
    const unsupported = analyzeChangeImpact(
      input(
        exportFixture({
          relation_kinds: exportFixture().relation_kinds.filter(
            ({ kind }) => kind !== "depends_on",
          ),
        }),
      ),
      ["FR-001"],
    );
    expect(absent).toMatchObject({ state: "not_computed", rows: [] });
    expect(unreadable).toMatchObject({ state: "not_computed", rows: [] });
    expect(empty).toMatchObject({ state: "incomplete", rows: [] });
    expect(empty.gaps).toContainEqual(
      expect.objectContaining({ kind: "empty-bindings-store" }),
    );
    expect(unsupported).toMatchObject({ state: "not_computed", rows: [] });
    expect(absent.gaps[0].kind).not.toBe(unreadable.gaps[0].kind);
  });

  // Trace: FR-062-AC-10
  it("TC-1258 renders equivalent permutations as identical canonical JSON", () => {
    const bindings = [binding("FR-002-AC-1", "z"), binding("FR-001-AC-1", "a")];
    const left = analyzeFanOut(input(exportFixture(), bindings));
    const right = analyzeFanOut(
      input(
        exportFixture({
          artifacts: [...exportFixture().artifacts].reverse(),
          obligations: [...exportFixture().obligations].reverse(),
        }),
        [...bindings].reverse(),
      ),
    );
    expect(renderGraphAnalysisJson(left)).toBe(renderGraphAnalysisJson(right));
    expect(renderGraphAnalysis(left)).toContain(left.source.revision);
    expect(renderGraphAnalysis(left)).toContain("Suite");
  });

  // Trace: FR-062-AC-12
  it("TC-1260 emits structural facts without scores or threshold labels", () => {
    const json = renderGraphAnalysisJson(
      analyzeChurn(
        input(exportFixture(), [binding("FR-001-AC-1", "unit", [event])]),
      ),
    );
    expect(json).not.toMatch(/trust|quality|releaseReadiness|threshold|score/i);
    expect(json).toContain("eventCount");
  });
});
