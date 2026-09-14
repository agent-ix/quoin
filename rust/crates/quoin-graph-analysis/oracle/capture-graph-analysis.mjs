// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// One-time oracle capture: what `src/graph-analysis/` COMPUTES on a named
// corpus of in-memory inputs, and what `loadGraphAnalysisInput` decides on a
// named corpus of real files.
//
// The corpus is a `scenario ladder`: one base export/premises/audit/bindings
// quartet, mutated once per named rung. Each rung is a fact about the
// algorithms — an unresolved binding, a dangling relation, a shared shortest
// path, a tie broken on path key, a note-less affirmation, a pipe inside an
// identifier — rather than a document somebody happened to write down. A
// hand-written fixture proves whatever its author believed; a rung proves the
// two implementations agree about one named condition.
//
// Every case records, for all three views, BOTH renderings:
//   * `renderGraphAnalysisJson` — the canonical JSON, byte for byte;
//   * `renderGraphAnalysis`     — the markdown.
//
// Run from the repository root:
//   node --loader ts-node/esm \
//     rust/crates/quoin-graph-analysis/oracle/capture-graph-analysis.mjs
//
// Deleted at the cutover commit per FR-101-AC-5. The captured goldens stay;
// this script must not become a runtime oracle.

import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  analyzeChangeImpact,
  analyzeChurn,
  analyzeFanOut,
  loadGraphAnalysisInput,
  parseAcceptedAssurancePremises,
  parseAuditEnvelope,
  renderGraphAnalysis,
  renderGraphAnalysisJson,
  validateAcceptedAssurancePremises,
  validateAuditIdentity,
} from "../../../../src/graph-analysis/index.ts";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, "..", "..", "..", "..");

const DIGEST = "a".repeat(64);
const DIGEST_B = "b".repeat(64);
const REVISION = "0".repeat(40);
const OTHER_REVISION = "1".repeat(40);

const SOURCE = { repository: "agent-ix/example", revision: REVISION };

const MODULES = [
  {
    name: "example",
    version: "1.0.0",
    schemas: [
      { archetype: "FR", schema_digest: DIGEST_B },
      { archetype: "AC", schema_digest: DIGEST },
    ],
  },
  { name: "another", version: "0.2.0", schemas: [] },
];

const RELATION_KINDS = [
  "depends_on",
  "derives_from",
  "implements",
  "mitigates",
  "refines",
  "requires",
  "satisfies",
  "traces_to",
];

function locator(path, line = 1) {
  return { path, line, digest: DIGEST };
}

function artifact(id, artifactType = "FR", path = `spec/${id}.md`) {
  return { id, artifact_type: artifactType, locator: locator(path) };
}

function obligation(id, ownerPath) {
  return {
    source: "acceptance-criterion",
    id,
    document: ownerPath,
    statement: `${id} statement`,
    statement_hash: DIGEST,
    target_ids: [`TC-${id}`],
    locator: locator(ownerPath, 20),
  };
}

function corpus(source, target, edgeType, resolution = "resolved") {
  return {
    kind: "corpus",
    source,
    target,
    edge_type: edgeType,
    resolution,
    locator: locator(`spec/${source}.md`, 5),
    freshness: "not_applicable",
  };
}

function binding(obligationId, suite, affirmations) {
  return {
    obligation: obligationId,
    statementHashAtBinding: DIGEST,
    suite,
    commit: REVISION,
    symbols: ["beta", "alpha"],
    ...(affirmations === undefined ? {} : { affirmations }),
  };
}

/** The base export: four requirements, one non-requirement, a small graph. */
function baseExport() {
  return {
    format: "quire-assurance",
    format_version: 1,
    source: SOURCE,
    modules: MODULES,
    artifacts: [
      artifact("FR-001"),
      artifact("FR-002"),
      artifact("FR-003"),
      artifact("FR-004"),
      artifact("NFR-001", "NFR"),
      artifact("IT-001", "IT"),
      // Two artifacts sharing one document: an obligation in that document is
      // owned by both, which is the multi-owner path through `obligationOwners`.
      artifact("US-001", "US", "spec/shared.md"),
      artifact("StR-001", "StR", "spec/shared.md"),
    ],
    obligations: [
      obligation("FR-001-AC-1", "spec/FR-001.md"),
      obligation("FR-002-AC-1", "spec/FR-002.md"),
      obligation("FR-003-AC-1", "spec/FR-003.md"),
      obligation("FR-004-AC-1", "spec/FR-004.md"),
      obligation("NFR-001-AC-1", "spec/NFR-001.md"),
      obligation("SHARED-AC-1", "spec/shared.md"),
      // Owned by nothing: `unresolved-obligation-owner`.
      obligation("ORPHAN-AC-1", "spec/nowhere.md"),
    ],
    symbols: [],
    relation_kinds: RELATION_KINDS.map((kind) => ({
      kind,
      availability: "available",
      sources: ["module_vocabulary"],
    })),
    relations: [
      // FR-002 -> FR-001, FR-003 -> FR-002: a two-hop reverse chain from FR-001.
      corpus("FR-002", "FR-001", "depends_on"),
      corpus("FR-003", "FR-002", "refines"),
      // Two ways into FR-004 at the same depth: the tie is broken on path key.
      corpus("FR-004", "FR-001", "satisfies"),
      corpus("FR-004", "FR-001", "requires"),
      // A cycle, so the frontier can revisit a node.
      corpus("FR-001", "FR-003", "traces_to"),
      // Requirement -> non-requirement: resolved and skipped, not a gap.
      corpus("IT-001", "FR-001", "traces_to"),
      // Dangling: a gap.
      corpus("FR-009", "FR-001", "depends_on", "dangling"),
      // A kind outside the default selection.
      corpus("NFR-001", "FR-001", "constrains"),
    ],
    relation_observations: [
      {
        declaration: "FR-001 requires a mitigation",
        subject: "FR-001",
        availability: "available",
        freshness: "current",
      },
      {
        declaration: "FR-002 requires a mitigation",
        availability: "missing",
        freshness: "unknown",
      },
      {
        declaration: "FR-003 requires a mitigation",
        subject: "FR-003",
        availability: "unknown",
        freshness: "unknown",
        reason: "the module vocabulary does not declare it",
      },
    ],
  };
}

function basePremises(exportValue) {
  return {
    format: exportValue.format,
    format_version: exportValue.format_version,
    modules: exportValue.modules,
  };
}

/** The audit report: findings, healthy, unevaluated, plus passthrough fields. */
function baseReport() {
  return {
    findings: [
      {
        obligation: "FR-002-AC-1",
        kind: "undischarged",
        severity: "high",
        summary: "no evidence binds this criterion",
        path: "src/cli.ts",
        line: 42,
        symbol: "parseArgs",
        subject: "the criterion",
        changeTarget: "tests/cli.test.ts",
        remedy: "bind a test",
      },
      {
        obligation: "FR-001-AC-1",
        kind: "stale-evidence",
        severity: "low",
        summary: "the statement moved",
      },
      {
        obligation: "FR-001-AC-1",
        kind: "suspect-link",
        severity: "medium",
        summary: "the hash no longer matches",
        nextDiagnosticStep: "re-run the suite",
      },
    ],
    healthy: ["SHARED-AC-1", "FR-004-AC-1", "SHARED-AC-1"],
    unevaluated: [
      {
        check: "mocked-confirmation",
        obligation: "FR-003-AC-1",
        suites: ["unit", "integration", "contract"],
        reason: "no inspection has run",
        inspectedAt: null,
      },
    ],
    // A passthrough member of the report itself, which the verdict never reads.
    independence: [{ obligation: "FR-001-AC-1", axes: ["author"] }],
  };
}

function auditEnvelope(exportValue, report) {
  return {
    format: "quoin-audit-envelope",
    format_version: 1,
    source: exportValue.source,
    export: basePremises(exportValue),
    report,
  };
}

function available(bindings) {
  return { availability: "available", bindings };
}

const BASE_BINDINGS = [
  binding("FR-001-AC-1", "unit", [
    { who: "bob", commit: REVISION, note: "reaffirmed" },
    { who: "alice", commit: REVISION },
    // The same (who, commit, note) as the first, so the pair dedupes and the
    // suites merge.
    { who: "bob", commit: REVISION, note: "reaffirmed" },
  ]),
  binding("FR-001-AC-1", "integration", [
    { who: "bob", commit: REVISION, note: "reaffirmed" },
  ]),
  binding("FR-002-AC-1", "unit", []),
  binding("FR-004-AC-1", "contract", [
    { who: "carol", commit: OTHER_REVISION, note: "ported" },
  ]),
  binding("SHARED-AC-1", "unit"),
  // Bound to an obligation the export does not carry: `unresolved-binding`.
  binding("GONE-AC-1", "unit", [{ who: "dave", commit: REVISION }]),
  binding("ALSO-GONE-AC-1", "unit"),
];

/** One scenario: a named rung with its four inputs and its impact arguments. */
function scenarios() {
  const cases = [];
  const add = (id, input, requested, relations) =>
    cases.push({ id, input, requested, relations });

  const base = () => {
    const exportValue = baseExport();
    return {
      assurance: exportValue,
      premises: basePremises(exportValue),
      audit: auditEnvelope(exportValue, baseReport()),
      bindings: available(BASE_BINDINGS),
    };
  };

  const seeds = ["FR-001", "FR-002", "FR-001"];

  add("base/default-relations", base(), seeds, undefined);
  add("base/no-seeds", base(), [], undefined);
  add("base/unknown-seed", base(), ["FR-999", "FR-001"], undefined);
  add("base/non-requirement-seed", base(), ["IT-001"], undefined);
  add("base/single-relation", base(), seeds, ["depends_on"]);
  add("base/duplicate-relation-selection", base(), seeds, [
    "refines",
    "depends_on",
    "refines",
  ]);
  add("base/empty-relation-selection", base(), seeds, []);
  add("base/unknown-relation-kind", base(), seeds, ["depends_on", "invented"]);
  add("base/seed-at-cycle", base(), ["FR-003"], undefined);

  {
    const input = base();
    input.bindings = {
      availability: "absent",
      reason: "bindings.json is absent",
    };
    add("bindings/absent", input, seeds, undefined);
  }
  {
    const input = base();
    input.bindings = {
      availability: "unreadable",
      reason:
        "bindings.json is valid JSON but does not match the retained bindings schema",
    };
    add("bindings/unreadable", input, seeds, undefined);
  }
  {
    const input = base();
    input.bindings = available([]);
    add("bindings/empty", input, seeds, undefined);
  }
  {
    const input = base();
    input.bindings = available(
      BASE_BINDINGS.filter(({ obligation: id }) => !id.endsWith("GONE-AC-1")),
    );
    add("bindings/all-resolved", input, seeds, undefined);
  }
  {
    // Every gap cleared: the only route to `state: "complete"`.
    const exportValue = baseExport();
    exportValue.artifacts = [artifact("FR-001"), artifact("FR-002")];
    exportValue.obligations = [
      obligation("FR-001-AC-1", "spec/FR-001.md"),
      obligation("FR-002-AC-1", "spec/FR-002.md"),
    ];
    exportValue.relations = [corpus("FR-002", "FR-001", "depends_on")];
    exportValue.relation_observations = [];
    const report = {
      findings: [],
      healthy: ["FR-001-AC-1", "FR-002-AC-1"],
      unevaluated: [],
    };
    add(
      "clean/complete",
      {
        assurance: exportValue,
        premises: basePremises(exportValue),
        audit: auditEnvelope(exportValue, report),
        bindings: available([
          binding("FR-001-AC-1", "unit", [{ who: "alice", commit: REVISION }]),
          binding("FR-002-AC-1", "unit"),
        ]),
      },
      ["FR-001"],
      undefined,
    );
  }
  {
    // Nothing at all: every collection empty.
    const exportValue = baseExport();
    exportValue.artifacts = [];
    exportValue.obligations = [];
    exportValue.relations = [];
    exportValue.relation_observations = [];
    exportValue.modules = [];
    add(
      "clean/empty-export",
      {
        assurance: exportValue,
        premises: basePremises(exportValue),
        audit: auditEnvelope(exportValue, {
          findings: [],
          healthy: [],
          unevaluated: [],
        }),
        bindings: available([]),
      },
      [],
      undefined,
    );
  }
  {
    // Markdown escaping, and UTF-16 versus scalar ordering. `\u{1F600}` is an
    // astral pair whose first unit (D83D) sorts BELOW U+FFFD in UTF-16 and
    // ABOVE it in Rust's scalar order.
    const exportValue = baseExport();
    exportValue.artifacts = [
      artifact("FR-001"),
      artifact("FR|002", "FR", "spec/pipe.md"),
      artifact("FR-\u{1F600}", "FR", "spec/astral.md"),
      artifact("FR-�", "FR", "spec/replacement.md"),
      artifact("FR-`tick`", "FR", "spec/tick.md"),
    ];
    exportValue.obligations = [
      obligation("AC|1", "spec/pipe.md"),
      obligation("AC-\u{1F600}", "spec/astral.md"),
      obligation("AC-�", "spec/replacement.md"),
      obligation("AC-newline", "spec/tick.md"),
      obligation("FR-001-AC-1", "spec/FR-001.md"),
    ];
    exportValue.relations = [
      corpus("FR|002", "FR-001", "depends_on"),
      corpus("FR-\u{1F600}", "FR-001", "depends_on"),
      corpus("FR-�", "FR-001", "depends_on"),
    ];
    exportValue.relation_observations = [
      {
        declaration: "a | pipe and a `tick`",
        availability: "missing",
        freshness: "unknown",
        reason: "a reason with a | pipe",
      },
    ];
    add(
      "text/escapes-and-utf16-order",
      {
        assurance: exportValue,
        premises: basePremises(exportValue),
        audit: auditEnvelope(exportValue, {
          findings: [
            {
              obligation: "AC|1",
              kind: "undischarged",
              severity: "high",
              summary: "a summary with a | pipe and a\nnewline",
            },
          ],
          healthy: ["AC-\u{1F600}", "AC-�", "AC-newline"],
          unevaluated: [],
        }),
        bindings: available([
          binding("AC|1", "suite|with|pipes", [
            { who: "a|b", commit: REVISION, note: "a\nnote" },
          ]),
          binding("AC-\u{1F600}", "z-suite"),
          binding("AC-�", "a-suite"),
          binding("AC-newline", "m-suite", [
            { who: "\u{1F600}", commit: REVISION },
            { who: "�", commit: REVISION },
          ]),
        ]),
      },
      ["FR-001"],
      undefined,
    );
  }
  {
    // Churn ordering: equal event counts fall back to the obligation id, and
    // a note-less affirmation must not collide with a noted one.
    const exportValue = baseExport();
    const report = baseReport();
    add(
      "churn/ties-and-missing-notes",
      {
        assurance: exportValue,
        premises: basePremises(exportValue),
        audit: auditEnvelope(exportValue, report),
        bindings: available([
          binding("FR-002-AC-1", "b", [
            { who: "alice", commit: REVISION },
            { who: "alice", commit: REVISION, note: "" },
            { who: "alice", commit: REVISION, note: "x" },
          ]),
          binding("FR-001-AC-1", "a", [
            { who: "alice", commit: REVISION },
            { who: "alice", commit: REVISION, note: "x" },
            { who: "alice", commit: OTHER_REVISION },
          ]),
          binding("FR-003-AC-1", "c", []),
          binding("FR-004-AC-1", "d"),
        ]),
      },
      ["FR-001"],
      undefined,
    );
  }
  {
    // A deeper chain, so a path of length three is reached and the shortest
    // of two candidate routes wins.
    const exportValue = baseExport();
    exportValue.artifacts = [
      artifact("FR-001"),
      artifact("FR-002"),
      artifact("FR-003"),
      artifact("FR-004"),
      artifact("FR-005"),
    ];
    exportValue.obligations = [obligation("FR-005-AC-1", "spec/FR-005.md")];
    exportValue.relations = [
      corpus("FR-002", "FR-001", "depends_on"),
      corpus("FR-003", "FR-002", "depends_on"),
      corpus("FR-004", "FR-003", "depends_on"),
      corpus("FR-005", "FR-004", "depends_on"),
      // A shortcut making FR-005 reachable in two hops as well as four.
      corpus("FR-005", "FR-002", "requires"),
    ];
    exportValue.relation_observations = [];
    add(
      "impact/shortest-of-two-routes",
      {
        assurance: exportValue,
        premises: basePremises(exportValue),
        audit: auditEnvelope(exportValue, {
          findings: [],
          healthy: ["FR-005-AC-1"],
          unevaluated: [],
        }),
        bindings: available([binding("FR-005-AC-1", "unit")]),
      },
      ["FR-001"],
      undefined,
    );
  }
  {
    // The obligation reached by a seed has no auditor verdict at all.
    const exportValue = baseExport();
    add(
      "impact/missing-auditor-verdict",
      {
        assurance: exportValue,
        premises: basePremises(exportValue),
        audit: auditEnvelope(exportValue, {
          findings: [],
          healthy: [],
          unevaluated: [],
        }),
        bindings: available(BASE_BINDINGS),
      },
      ["FR-001", "FR-002", "FR-004"],
      undefined,
    );
  }
  {
    // Multi-owner obligations: the shared document is owned by US-001 and
    // StR-001, both requirement artifacts.
    const exportValue = baseExport();
    add(
      "impact/multi-owner-document",
      {
        assurance: exportValue,
        premises: basePremises(exportValue),
        audit: auditEnvelope(exportValue, baseReport()),
        bindings: available(BASE_BINDINGS),
      },
      ["US-001", "StR-001"],
      RELATION_KINDS,
    );
  }
  {
    // Two bindings on the same (obligation, suite): `groupBindings` keeps the
    // first and drops the second.
    const exportValue = baseExport();
    add(
      "impact/duplicate-obligation-suite",
      {
        assurance: exportValue,
        premises: basePremises(exportValue),
        audit: auditEnvelope(exportValue, baseReport()),
        bindings: available([
          binding("FR-001-AC-1", "unit"),
          binding("FR-001-AC-1", "unit", [{ who: "z", commit: REVISION }]),
          binding("FR-001-AC-1", "a-suite"),
        ]),
      },
      ["FR-001"],
      undefined,
    );
  }
  return cases;
}

function analysisCase(entry) {
  const { input, requested, relations } = entry;
  const views = {
    "fan-out": analyzeFanOut(input),
    churn: analyzeChurn(input),
    "change-impact": analyzeChangeImpact(input, requested, relations),
  };
  return {
    id: entry.id,
    input,
    requested,
    ...(relations === undefined ? {} : { relations }),
    views: Object.fromEntries(
      Object.entries(views).map(([view, analysis]) => [
        view,
        {
          json: renderGraphAnalysisJson(analysis),
          markdown: renderGraphAnalysis(analysis),
        },
      ]),
    ),
  };
}

/** Input-contract verdicts: accepted or refused, per named text. */
function contractCases() {
  const exportValue = baseExport();
  const premises = basePremises(exportValue);
  const envelope = auditEnvelope(exportValue, baseReport());
  const out = [];

  const premisesTexts = [
    ["premises/valid", JSON.stringify(premises)],
    ["premises/not-json", "{"],
    ["premises/not-an-object", "[]"],
    ["premises/wrong-format", JSON.stringify({ ...premises, format: "other" })],
    [
      "premises/wrong-version",
      JSON.stringify({ ...premises, format_version: 2 }),
    ],
    ["premises/extra-member", JSON.stringify({ ...premises, extra: 1 })],
    [
      "premises/short-digest",
      JSON.stringify({
        ...premises,
        modules: [
          {
            name: "m",
            version: "1",
            schemas: [{ archetype: "FR", schema_digest: "abc" }],
          },
        ],
      }),
    ],
    [
      "premises/uppercase-digest",
      JSON.stringify({
        ...premises,
        modules: [
          {
            name: "m",
            version: "1",
            schemas: [{ archetype: "FR", schema_digest: "A".repeat(64) }],
          },
        ],
      }),
    ],
    [
      "premises/empty-module-name",
      JSON.stringify({
        ...premises,
        modules: [{ name: "", version: "1", schemas: [] }],
      }),
    ],
    [
      "premises/unsorted-modules",
      JSON.stringify({
        format: "quire-assurance",
        format_version: 1,
        modules: [
          { name: "z", version: "2", schemas: [] },
          { name: "z", version: "1", schemas: [] },
          {
            name: "a",
            version: "1",
            schemas: [
              { archetype: "z", schema_digest: DIGEST },
              { archetype: "a", schema_digest: DIGEST_B },
              { archetype: "a", schema_digest: DIGEST },
            ],
          },
        ],
      }),
    ],
  ];
  for (const [id, text] of premisesTexts) {
    const parsed = parseAcceptedAssurancePremises(text);
    out.push({
      id,
      contract: "premises",
      text,
      accepted: parsed.ok,
      ...(parsed.ok ? { canonical: JSON.stringify(parsed.value) } : {}),
    });
  }

  const auditTexts = [
    ["audit/valid", JSON.stringify(envelope)],
    ["audit/not-json", "not json at all"],
    ["audit/wrong-format", JSON.stringify({ ...envelope, format: "other" })],
    ["audit/extra-member", JSON.stringify({ ...envelope, extra: true })],
    [
      "audit/bad-revision",
      JSON.stringify({
        ...envelope,
        source: { repository: "r", revision: "nope" },
      }),
    ],
    [
      "audit/unknown-severity",
      JSON.stringify({
        ...envelope,
        report: {
          ...baseReport(),
          findings: [
            {
              obligation: "FR-001-AC-1",
              kind: "undischarged",
              severity: "critical",
              summary: "s",
            },
          ],
        },
      }),
    ],
    [
      "audit/finding-passthrough-preserved",
      JSON.stringify({
        ...envelope,
        report: {
          findings: [
            {
              obligation: "FR-001-AC-1",
              kind: "undischarged",
              severity: "low",
              summary: "s",
              invented: { deeply: ["nested", 1, true, null] },
            },
          ],
          healthy: [],
          unevaluated: [],
          alsoInvented: 7,
        },
      }),
    ],
    // Two findings alike but for passthrough members whose names are
    // ECMAScript array indices. `stableKey` is `JSON.stringify(canonical(v))`,
    // and a JS object emits integer-like names first in ascending numeric
    // order whatever order they were inserted in, so its key text is NOT the
    // RFC 8785 member order. The two orders disagree here, and disagree in the
    // direction that swaps these two findings. This rung exists so that the
    // divergence declared in `tests/tc_385_parity.rs` has something to fire
    // on; without it the declaration would be a claim about nothing.
    [
      "audit/array-index-passthrough-order",
      JSON.stringify({
        ...envelope,
        report: {
          findings: [
            {
              obligation: "FR-001-AC-1",
              kind: "undischarged",
              severity: "low",
              summary: "s",
              10: 1,
              2: 0,
            },
            {
              obligation: "FR-001-AC-1",
              kind: "undischarged",
              severity: "low",
              summary: "s",
              10: 0,
              2: 1,
            },
          ],
          healthy: [],
          unevaluated: [],
        },
      }),
    ],
    [
      "audit/empty-healthy-entry",
      JSON.stringify({
        ...envelope,
        report: { findings: [], healthy: [""], unevaluated: [] },
      }),
    ],
    [
      "audit/missing-report",
      JSON.stringify({ ...envelope, report: undefined }),
    ],
    [
      "audit/unsorted-report",
      JSON.stringify({
        ...envelope,
        report: {
          findings: [
            { obligation: "b", kind: "k", severity: "low", summary: "s" },
            { obligation: "a", kind: "k", severity: "low", summary: "s" },
            { obligation: "a", kind: "j", severity: "high", summary: "s" },
          ],
          healthy: ["z", "a", "m"],
          unevaluated: [
            {
              check: "mocked-confirmation",
              obligation: "b",
              suites: ["z", "a"],
              reason: "r",
            },
            {
              check: "mocked-confirmation",
              obligation: "a",
              suites: ["m"],
              reason: "r",
            },
          ],
        },
      }),
    ],
  ];
  for (const [id, text] of auditTexts) {
    const parsed = parseAuditEnvelope(text);
    out.push({
      id,
      contract: "audit",
      text,
      accepted: parsed.ok,
      ...(parsed.ok ? { canonical: JSON.stringify(parsed.value) } : {}),
    });
  }

  // Cross-checks: premises-versus-export and audit-identity-versus-export.
  const matchCases = [
    ["match/premises-identical", premises, true],
    [
      "match/premises-reordered",
      {
        format: "quire-assurance",
        format_version: 1,
        modules: [...MODULES].reverse().map((module) => ({
          ...module,
          schemas: [...module.schemas].reverse(),
        })),
      },
      true,
    ],
    [
      "match/premises-missing-module",
      { ...premises, modules: [MODULES[0]] },
      false,
    ],
    [
      "match/premises-different-digest",
      {
        ...premises,
        modules: MODULES.map((module) => ({
          ...module,
          schemas: module.schemas.map((schema) => ({
            ...schema,
            schema_digest: DIGEST,
          })),
        })),
      },
      false,
    ],
  ];
  for (const [id, accepted] of matchCases) {
    const violation = validateAcceptedAssurancePremises(exportValue, accepted);
    out.push({
      id,
      contract: "premises-match",
      text: JSON.stringify(accepted),
      // The cross-checks are about a candidate AND the export it is checked
      // against, so the export travels with the case rather than being
      // recovered from a scenario that happens to share it.
      against_export: JSON.stringify(exportValue),
      accepted: violation === null,
    });
  }

  const identityCases = [
    ["identity/matching", envelope, true],
    [
      "identity/other-revision",
      { ...envelope, source: { ...SOURCE, revision: OTHER_REVISION } },
      false,
    ],
    [
      "identity/other-repository",
      { ...envelope, source: { ...SOURCE, repository: "other/repo" } },
      false,
    ],
    [
      "identity/other-premises",
      { ...envelope, export: { ...premises, modules: [MODULES[0]] } },
      false,
    ],
    [
      "identity/reordered-premises",
      {
        ...envelope,
        export: {
          ...premises,
          modules: [...MODULES].reverse().map((module) => ({
            ...module,
            schemas: [...module.schemas].reverse(),
          })),
        },
      },
      true,
    ],
  ];
  for (const [id, candidate] of identityCases) {
    const violation = validateAuditIdentity(candidate, exportValue);
    out.push({
      id,
      contract: "audit-identity",
      text: JSON.stringify(candidate),
      against_export: JSON.stringify(exportValue),
      accepted: violation === null,
    });
  }
  return out;
}

/** Loader verdicts, over real files in a throwaway tree. */
function loadCases() {
  const root = mkdtempSync(join(tmpdir(), "quoin-graph-oracle-"));
  const out = [];
  let counter = 0;
  const write = (name, text) => {
    const path = join(root, `${counter++}-${name}`);
    writeFileSync(path, text, "utf8");
    return path;
  };

  const exportValue = baseExport();
  const premises = basePremises(exportValue);
  const envelope = auditEnvelope(exportValue, baseReport());
  const exportText = JSON.stringify(exportValue);
  const premisesText = JSON.stringify(premises);
  const auditText = JSON.stringify(envelope);

  const repoWith = (name, bindingsText) => {
    const repo = join(root, `repo-${name}`);
    mkdirSync(join(repo, "spec", "evidence"), { recursive: true });
    if (bindingsText !== undefined) {
      writeFileSync(
        join(repo, "spec", "evidence", "bindings.json"),
        bindingsText,
        "utf8",
      );
    }
    return repo;
  };

  const storeText = (bindings) =>
    JSON.stringify({ schemaVersion: 1, bindings });

  const entries = [
    {
      id: "load/ok-bindings-available",
      repo: repoWith("available", storeText(BASE_BINDINGS)),
      export: exportText,
      premises: premisesText,
      audit: auditText,
      bindingsFile: storeText(BASE_BINDINGS),
    },
    {
      id: "load/ok-bindings-absent",
      repo: repoWith("absent", undefined),
      export: exportText,
      premises: premisesText,
      audit: auditText,
      bindingsFile: null,
    },
    {
      id: "load/ok-bindings-unreadable-schema",
      repo: repoWith("badschema", storeText([{ obligation: "x" }])),
      export: exportText,
      premises: premisesText,
      audit: auditText,
      bindingsFile: storeText([{ obligation: "x" }]),
    },
    {
      id: "load/ok-bindings-unreadable-json",
      repo: repoWith("badjson", "{not json"),
      export: exportText,
      premises: premisesText,
      audit: auditText,
      bindingsFile: "{not json",
    },
    {
      id: "load/ok-bindings-null",
      repo: repoWith("null", "null"),
      export: exportText,
      premises: premisesText,
      audit: auditText,
      bindingsFile: "null",
    },
    {
      id: "load/ok-bindings-wrong-schema-version",
      repo: repoWith(
        "version",
        JSON.stringify({ schemaVersion: 2, bindings: [] }),
      ),
      export: exportText,
      premises: premisesText,
      audit: auditText,
      bindingsFile: JSON.stringify({ schemaVersion: 2, bindings: [] }),
    },
    {
      id: "load/export-missing",
      repo: repoWith("m1", storeText([])),
      exportMissing: true,
      export: exportText,
      premises: premisesText,
      audit: auditText,
      bindingsFile: storeText([]),
    },
    {
      id: "load/export-not-json",
      repo: repoWith("m2", storeText([])),
      export: "{",
      premises: premisesText,
      audit: auditText,
      bindingsFile: storeText([]),
    },
    {
      id: "load/export-schema-invalid",
      repo: repoWith("m3", storeText([])),
      export: JSON.stringify({ ...exportValue, format: "nope" }),
      premises: premisesText,
      audit: auditText,
      bindingsFile: storeText([]),
    },
    {
      id: "load/premises-invalid",
      repo: repoWith("m4", storeText([])),
      export: exportText,
      premises: "{}",
      audit: auditText,
      bindingsFile: storeText([]),
    },
    {
      id: "load/premises-mismatch",
      repo: repoWith("m5", storeText([])),
      export: exportText,
      premises: JSON.stringify({ ...premises, modules: [MODULES[0]] }),
      audit: auditText,
      bindingsFile: storeText([]),
    },
    {
      id: "load/audit-invalid",
      repo: repoWith("m6", storeText([])),
      export: exportText,
      premises: premisesText,
      audit: "{}",
      bindingsFile: storeText([]),
    },
    {
      id: "load/audit-identity-mismatch",
      repo: repoWith("m7", storeText([])),
      export: exportText,
      premises: premisesText,
      audit: JSON.stringify({
        ...envelope,
        source: { ...SOURCE, revision: OTHER_REVISION },
      }),
      bindingsFile: storeText([]),
    },
  ];

  for (const entry of entries) {
    const exportPath = entry.exportMissing
      ? join(root, "no-such-export.json")
      : write("export.json", entry.export);
    const premisesPath = write("premises.json", entry.premises);
    const auditPath = write("audit.json", entry.audit);
    const result = loadGraphAnalysisInput({
      repo: entry.repo,
      exportPath,
      premisesPath,
      auditPath,
    });
    out.push({
      id: entry.id,
      export_text: entry.exportMissing ? null : entry.export,
      premises_text: entry.premises,
      audit_text: entry.audit,
      bindings_file: entry.bindingsFile ?? null,
      ok: result.ok,
      ...(result.ok
        ? { bindings_availability: result.value.bindings.availability }
        : { error_kind: result.error.kind, error_input: result.error.input }),
    });
  }
  rmSync(root, { recursive: true, force: true });
  return out;
}

const revision = execFileSync("git", ["rev-parse", "HEAD"], {
  cwd: repoRoot,
  encoding: "utf8",
}).trim();

const goldens = {
  provenance: {
    producer:
      "rust/crates/quoin-graph-analysis/oracle/capture-graph-analysis.mjs",
    oracle: "src/graph-analysis/{analysis,input,load,render}.ts",
    node_version: process.version,
    quoin_revision: revision,
  },
  cases: scenarios().map(analysisCase),
  contract: contractCases(),
  load: loadCases(),
};

const out = join(here, "..", "tests", "goldens", "graph-analysis.json");
writeFileSync(out, `${JSON.stringify(goldens, null, 2)}\n`, "utf8");
process.stdout.write(
  `wrote ${out}: ${goldens.cases.length} analysis cases, ${goldens.contract.length} contract cases, ${goldens.load.length} load cases\n`,
);
