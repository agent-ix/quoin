/** FR-045 — bidirectional trace graph analyses (TC-291..TC-298). */

import { describe, expect, it } from "vitest";

import type { BundleDocument } from "../src/completeness/index.js";
import type { Binding } from "../src/evidence/index.js";
import {
  analyzeChangeImpact,
  analyzeChurn,
  analyzeFanOut,
  buildTraceGraph,
  renderGraphAnalysis,
} from "../src/graph-analysis/index.js";
import type { Obligation } from "../src/quire/index.js";

function document(
  id: string,
  relationships: Array<{ type: string; target: string }> = [],
): BundleDocument {
  return {
    path: `${id}.md`,
    frontmatter: { id, type: id.split("-")[0], relationships },
    body: "",
  };
}

function master(org = "agent-ix", name = "demo"): BundleDocument {
  return {
    path: "spec.md",
    frontmatter: { type: "master-requirements", org, name },
    body: "",
  };
}

function obligation(id: string): Obligation {
  return {
    id,
    source: "Acceptance Criteria",
    document: `${id.split("-AC-")[0]}.md`,
    statement: id,
    statement_hash: "a".repeat(64),
  };
}

function binding(
  obligationId: string,
  suite: string,
  affirmations?: Binding["affirmations"],
): Binding {
  return {
    obligation: obligationId,
    suite,
    commit: "1".repeat(40),
    statementHashAtBinding: "a".repeat(64),
    symbols: ["test"],
    ...(affirmations ? { affirmations } : {}),
  };
}

function completeGraph() {
  const documents = [
    master(),
    document("StR-001"),
    document("FR-001", [
      { type: "traces_to", target: "ix://agent-ix/demo/StR-001" },
    ]),
    document("FR-002", [{ type: "derives_from", target: "FR-001" }]),
    document("FR-003", [{ type: "traces_to", target: "StR-001" }]),
  ];
  const obligations = [
    obligation("FR-001-AC-1"),
    obligation("FR-002-AC-1"),
    obligation("FR-003-AC-1"),
  ];
  const bindings = [
    binding("FR-001-AC-1", "unit"),
    binding("FR-002-AC-1", "integration"),
    binding("FR-003-AC-1", "integration"),
  ];
  return {
    graph: buildTraceGraph({ documents, obligations, bindings }),
    bindings,
  };
}

describe("FR-045 trace graph construction", () => {
  // Trace: FR-045-AC-1
  it("normalizes child-authored edges into upstream-to-downstream direction", () => {
    const { graph } = completeGraph();
    expect(graph.documentEdges).toEqual([
      { from: "FR-001", to: "FR-002", relationship: "derives_from" },
      { from: "StR-001", to: "FR-001", relationship: "traces_to" },
      { from: "StR-001", to: "FR-003", relationship: "traces_to" },
    ]);
    expect(graph.complete).toBe(true);

    const constraint = buildTraceGraph({
      documents: [
        document("FR-010"),
        document("NFR-001", [{ type: "constrains", target: "FR-010" }]),
      ],
      obligations: [],
      bindings: [],
    });
    expect(constraint.documentEdges).toEqual([
      { from: "NFR-001", to: "FR-010", relationship: "constrains" },
    ]);

    const external = buildTraceGraph({
      documents: [
        master(),
        document("FR-053"),
        document("FR-020", [
          { type: "traces_to", target: "ix://agent-ix/quire-rs/FR-053" },
        ]),
      ],
      obligations: [],
      bindings: [],
    });
    expect(external.documentEdges).toEqual([]);
    expect(external.limitations[0]).toMatchObject({
      kind: "unresolved-relationship",
      target: "ix://agent-ix/quire-rs/FR-053",
    });
  });

  // Trace: FR-045-AC-2
  it("names every reason the graph cannot claim completeness", () => {
    const graph = buildTraceGraph({
      documents: [
        document("FR-001", [
          { type: "publishes", target: "Event-001" },
          { type: "traces_to", target: "StR-404" },
        ]),
        { ...document("FR-001"), path: "duplicate.md" },
      ],
      obligations: [obligation("FR-002-AC-1")],
      bindings: [binding("FR-999-AC-1", "old-suite")],
      unreadable: [{ path: "broken.md", reason: "bad YAML" }],
    });

    expect(graph.complete).toBe(false);
    expect(graph.limitations.map(({ kind }) => kind)).toEqual([
      "duplicate-document-id",
      "orphan-binding",
      "orphan-obligation",
      "unreadable-document",
      "unresolved-relationship",
      "unsupported-relationship",
    ]);
  });
});

describe("FR-045 fan-out", () => {
  // Trace: FR-045-AC-3
  it("counts distinct obligation-to-suite edges without imposing a threshold", () => {
    const { graph } = completeGraph();
    graph.obligationSuites.push({
      obligation: "FR-002-AC-1",
      suite: "integration",
    });
    expect(analyzeFanOut(graph).rows).toEqual([
      {
        suite: "integration",
        obligationCount: 2,
        obligations: ["FR-002-AC-1", "FR-003-AC-1"],
      },
      {
        suite: "unit",
        obligationCount: 1,
        obligations: ["FR-001-AC-1"],
      },
    ]);
  });
});

describe("FR-045 change-impact closure", () => {
  // Trace: FR-045-AC-4
  it("walks downstream to suspect evidence and upstream to review context", () => {
    const { graph } = completeGraph();
    expect(analyzeChangeImpact(graph, ["FR-001"])).toMatchObject({
      changed: ["FR-001"],
      unknown: [],
      downstreamDocuments: ["FR-001", "FR-002"],
      upstreamDocuments: ["StR-001"],
      suspectObligations: ["FR-001-AC-1", "FR-002-AC-1"],
      affectedSuites: ["integration", "unit"],
      sharedSuiteExposure: ["FR-003-AC-1"],
    });
    expect(analyzeChangeImpact(graph, ["FR-002-AC-1"])).toMatchObject({
      downstreamDocuments: [],
      upstreamDocuments: ["FR-001", "StR-001"],
      suspectObligations: ["FR-002-AC-1"],
      affectedSuites: ["integration"],
      sharedSuiteExposure: ["FR-003-AC-1"],
    });
  });

  // Trace: FR-045-AC-5
  it("treats a changed suite as making its bound obligations suspect", () => {
    const { graph } = completeGraph();
    expect(analyzeChangeImpact(graph, ["integration"])).toMatchObject({
      downstreamDocuments: [],
      upstreamDocuments: ["FR-001", "StR-001"],
      suspectObligations: ["FR-002-AC-1", "FR-003-AC-1"],
      affectedSuites: ["integration"],
      sharedSuiteExposure: [],
    });
  });

  // Trace: FR-045-AC-6
  it("reports unknown changed ids instead of returning an apparently empty closure", () => {
    const { graph } = completeGraph();
    expect(analyzeChangeImpact(graph, ["missing"])).toMatchObject({
      changed: ["missing"],
      unknown: ["missing"],
      complete: false,
      downstreamDocuments: [],
      suspectObligations: [],
      affectedSuites: [],
    });
  });
});

describe("FR-045 churn", () => {
  // Trace: FR-045-AC-7
  it("deduplicates one obligation-level affirmation copied across suites", () => {
    const event = {
      who: "reviewer",
      commit: "2".repeat(40),
      note: "still fits",
    };
    const bindings = [
      binding("FR-001-AC-1", "unit", [event]),
      binding("FR-001-AC-1", "integration", [event]),
      binding("FR-002-AC-1", "integration", [
        event,
        { who: "reviewer", commit: "3".repeat(40) },
      ]),
    ];
    const graph = buildTraceGraph({
      documents: [document("FR-001"), document("FR-002")],
      obligations: [obligation("FR-001-AC-1"), obligation("FR-002-AC-1")],
      bindings,
    });
    expect(analyzeChurn(graph, bindings).rows).toMatchObject([
      { obligation: "FR-002-AC-1", affirmationCount: 2 },
      { obligation: "FR-001-AC-1", affirmationCount: 1 },
    ]);
  });

  // Trace: FR-045-AC-8
  it("renders byte-identically and labels incomplete data", () => {
    const graph = buildTraceGraph({
      documents: [document("FR-001")],
      obligations: [obligation("FR-001-AC-1")],
      bindings: [],
      unreadable: [{ path: "bad.md", reason: "bad YAML" }],
    });
    const analysis = analyzeFanOut(graph);
    const first = renderGraphAnalysis(analysis);
    expect(renderGraphAnalysis(analysis)).toBe(first);
    expect(first).toContain("Graph completeness: **incomplete**");
    expect(first).toContain("**unreadable-document** `bad.md`: bad YAML");

    const { graph: populated, bindings } = completeGraph();
    expect(renderGraphAnalysis(analyzeFanOut(populated))).toContain(
      "| integration | 2 |",
    );
    expect(
      renderGraphAnalysis(analyzeChangeImpact(populated, ["FR-001"])),
    ).toContain("## Suspect obligations\n\n- `FR-001-AC-1`");
    expect(
      renderGraphAnalysis(
        analyzeChurn(populated, [
          ...bindings,
          binding("FR-001-AC-1", "unit", [
            { who: "reviewer", commit: "2".repeat(40) },
          ]),
        ]),
      ),
    ).toContain("| `FR-001-AC-1` | 1 |");
  });
});
