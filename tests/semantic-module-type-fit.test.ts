import { createHash } from "node:crypto";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  buildSemanticAudit,
  createArtifactFiles,
  freshCensus,
  isAllowedAuditPath,
  parseMarkdownRecord,
  serializeCanonical,
  verifyArtifactFiles,
  writeArtifactFiles,
} from "../scripts/lib/semantic-module-type-fit.mjs";

const SHA_A = "a".repeat(40);
const SHA_B = "b".repeat(40);

interface Denominator {
  source: number;
  inventoried: number;
  reconciled: boolean;
}

interface AxisAssessment {
  status: string;
  confidence: string;
  evidence: string[];
  explanation?: string;
}

interface AuditFinding {
  id: string;
  kind: string;
  severity: string;
  status: string;
  confidence: string;
  evidence: string[];
  rationale: string;
  nextBoundary: string;
  qualifiedTypes: string[];
  concept?: string;
  reconciliation: Record<string, string>;
}

interface SemanticAudit {
  snapshot: {
    timestamp: string;
    defaultModulesDigest: string;
    modules: Array<{
      name: string;
      resolvedSha: string;
      canonicalRepository: string;
      inspected: Record<string, unknown>;
    }>;
  };
  inventory: {
    declarations: Array<{
      module: string;
      name: string;
      qualifiedName: string;
      surfaces: Record<string, { present: boolean }>;
      instances: string[];
      observations: string[];
    }>;
    documents: Array<{
      module: string;
      path: string;
      state: string;
      reason?: string;
      declaredType?: string;
      documentId?: string | null;
      signals: string[];
    }>;
    denominators: Record<string, Denominator> & {
      modules: Denominator;
      documents: Denominator;
    };
  };
  typeFit: {
    axes: string[];
    assessments: Array<{
      qualifiedName: string;
      disposition: string;
      flags: string[];
      axes: Record<string, AxisAssessment> & {
        generatedCode: AxisAssessment;
        roundTrip: AxisAssessment;
        structure: AxisAssessment;
        definitionOccurrence: AxisAssessment;
      };
    }>;
  };
  conflicts: AuditFinding[];
  missingTypes: AuditFinding[];
  repositoryImpact: Array<{
    impact: string;
    effort: string;
    risk: string;
    wave: string;
    confidence: string;
    rationale: string;
  }>;
  reconciliation: {
    coreData: Array<{ disposition: string }>;
    shadowContracts: unknown[];
    quire: Record<string, unknown>;
    followUpBoundaries: Array<{
      majorInterference: boolean;
      gate: string | null;
    }>;
  };
  summary: { verdict: string };
}

function manifest(name: string, types: string): string {
  return `manifest_version: 1.0.0\nname: ${name}\nversion: 1.0.0\n${types}`;
}

function auditInput() {
  return {
    timestamp: "2026-08-30T20:00:00.000Z",
    quoin: { commit: SHA_A, clean: true, version: "0.9.0" },
    defaultModulesText: `schemaVersion: 1\nname: fixture\nentries:\n  - name: alpha\n    version: 1.0.0\n    source: { type: git-subdir, url: agent-ix/alpha, path: alpha, ref: ${SHA_A} }\n  - name: beta\n    version: 1.0.0\n    source: { type: git-subdir, url: agent-ix/beta, path: beta, ref: v1.0.0 }\n`,
    tools: {
      quire: {
        cliVersion: "0.31.0",
        cliCommit: "4f6ed024",
        engineVersion: "0.46.0",
        engineCommit: "ca7362d4",
      },
    },
    externalEvidence: {
      quireCorpusRevision: SHA_A,
      coreDataCensusRevision: SHA_B,
    },
    modules: [
      {
        declarationIndex: 0,
        resolvedSha: SHA_A,
        installed: {
          sourcePath: "modules/alpha",
          sourceCommit: SHA_A,
          clean: true,
          contentDigest: `sha256:${"1".repeat(64)}`,
        },
        manifestText: manifest(
          "alpha",
          `artifact_types:\n  - name: Foo\n    frontmatter_schema_ref: schemas/foo.json\n    allowed_links: [references]\nobject_types:\n  - name: event\n    data_schema: { type: object }\n    roles: [event-like]\n`,
        ),
        moduleFiles: [
          {
            path: "schemas/foo.json",
            content:
              '{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}',
          },
          {
            path: "skeletons/Foo.md",
            content: "---\nid: FOO-{next}\ntype: Foo\n---\n",
          },
        ],
        corpusFiles: [
          {
            path: "spec/foo.md",
            content:
              "---\nid: FOO-001\ntitle: One\ntype: Foo\nversion: 1\nrelationships: []\n---\n# One\n",
          },
          {
            path: "spec/event.md",
            content:
              "---\nid: EVT-001\ntype: event\ntimestamp: 2026-08-30\nresult: passed\nevidence: run-1\n---\n# Event\n",
          },
          { path: "spec/bad.md", content: "---\ntype: [broken\n---\n" },
          { path: "README.md", content: "# Alpha\n" },
          { path: "spec/unreadable.md", content: null },
          { path: "vendor/excluded.md", content: "---\ntype: Foo\n---\n" },
        ],
        excludedMarkdown: { "vendor/excluded.md": "vendored fixture" },
      },
      {
        declarationIndex: 1,
        resolvedSha: SHA_B,
        installed: {
          sourcePath: "modules/beta",
          sourceCommit: SHA_B,
          clean: true,
          contentDigest: `sha256:${"2".repeat(64)}`,
        },
        manifestText: manifest(
          "beta",
          `artifact_types:\n  - name: Foo\n    frontmatter_schema_ref: schemas/foo.json\nobject_types:\n  - name: opaque\n    data_schema: { type: object, properties: { payload: { type: string } } }\n    body_extraction:\n      yield_pattern:\n        match:\n          schema_json: { from: code_block, language: json, required: true }\n`,
        ),
        moduleFiles: [
          {
            path: "schemas/foo.json",
            content:
              '{"type":"object","properties":{"name":{"type":"number"}}}',
          },
        ],
        corpusFiles: [],
      },
    ],
    architecture: {
      revision: SHA_A,
      planes: ["meta", "definition", "execution-observation", "presentation"],
      authority: "docs/semantic-module-architecture/planes-and-authority.md",
      ownership:
        "docs/semantic-module-architecture/ownership-and-boundaries.md",
      decision: "docs/semantic-module-architecture/decision-ledger.md",
    },
    repositoryBoundaries: [
      "alpha",
      "beta",
      "quoin",
      "quire",
      "filament-core-data",
      "compiler",
      "generated-packages",
      "database",
      "api",
      "cli",
      "ui",
    ],
  };
}

function built(): SemanticAudit {
  return buildSemanticAudit(auditInput()) as SemanticAudit;
}

describe("semantic module type-fit audit", () => {
  // Trace: FR-051-AC-1
  // TC-1156
  it("TC-1156", "captures Quoin and manifest identity", () => {
    const { snapshot } = built();
    expect(snapshot).toMatchObject({
      timestamp: auditInput().timestamp,
      quoin: auditInput().quoin,
    });
    expect(snapshot.defaultModulesDigest).toMatch(/^sha256:[0-9a-f]{64}$/);
  });

  // Trace: FR-051-AC-2
  // TC-1157
  it("TC-1157", "captures every declared module source and full SHA", () => {
    const rows = built().snapshot.modules;
    expect(rows.map((row) => row.name)).toEqual(["alpha", "beta"]);
    expect(rows.every((row) => /^[0-9a-f]{40}$/.test(row.resolvedSha))).toBe(
      true,
    );
    expect(rows[0].canonicalRepository).toBe("agent-ix/alpha");
  });

  // Trace: FR-051-AC-3
  // TC-1158
  it("TC-1158", "captures inspected content and manifest identity", () => {
    expect(built().snapshot.modules[0].inspected).toMatchObject({
      manifestName: "alpha",
      manifestVersion: "1.0.0",
      sourcePath: "modules/alpha",
      sourceCommit: SHA_A,
      clean: true,
    });
  });

  // Trace: FR-051-AC-4
  // TC-1159
  it("TC-1159", "pins tool and external evidence identities", () => {
    expect(built().snapshot).toMatchObject({
      tools: auditInput().tools,
      externalEvidence: auditInput().externalEvidence,
    });
  });

  // Trace: FR-051-AC-5
  // TC-1160
  it("TC-1160", "retains provenance disagreement and blocks clean", () => {
    const input = auditInput();
    input.modules[0].resolvedSha = SHA_B;
    const result = buildSemanticAudit(input) as SemanticAudit;
    expect(
      result.conflicts.some((row) => row.kind === "provenance-conflict"),
    ).toBe(true);
    expect(result.summary.verdict).not.toBe("clean");
  });

  // Trace: FR-051-AC-6
  // TC-1161
  it("TC-1161", "serializes equivalent inputs identically", () => {
    expect(serializeCanonical(built())).toBe(
      serializeCanonical(buildSemanticAudit(auditInput())),
    );
  });

  // Trace: FR-052-AC-1
  // TC-1162
  it("TC-1162", "keeps the module denominator equal to declarations", () => {
    expect(built().inventory.denominators.modules).toEqual({
      source: 2,
      inventoried: 2,
      reconciled: true,
    });
    const missing = auditInput();
    missing.modules.splice(1, 1);
    const retained = buildSemanticAudit(missing) as SemanticAudit;
    expect(retained.inventory.denominators.modules).toEqual({
      source: 2,
      inventoried: 2,
      reconciled: true,
    });
    expect(
      retained.conflicts.some((row) => row.kind === "provenance-conflict"),
    ).toBe(true);
  });

  // Trace: FR-052-AC-2
  // TC-1163
  it(
    "TC-1163",
    "keeps every declaration including qualified duplicates",
    () => {
      const declarations = built().inventory.declarations;
      expect(declarations).toHaveLength(4);
      expect(declarations.filter((row) => row.name === "Foo")).toHaveLength(2);
    },
  );

  // Trace: FR-052-AC-3
  // TC-1164
  it("TC-1164", "records every contract surface as present or absent", () => {
    for (const row of built().inventory.declarations) {
      expect(Object.keys(row.surfaces).sort()).toEqual([
        "mappings",
        "projections",
        "relationships",
        "schema",
        "skeleton",
      ]);
      expect(
        Object.values(row.surfaces).every(
          (surface) => typeof surface.present === "boolean",
        ),
      ).toBe(true);
    }
  });

  // Trace: FR-052-AC-4
  // TC-1165
  it("TC-1165", "assigns one parse state and reasons to failures", () => {
    const docs = built().inventory.documents.filter(
      (row) => row.module === "alpha",
    );
    expect(docs.map((row) => row.state).sort()).toEqual(
      ["excluded", "invalid", "io-error", "parsed", "parsed", "untyped"].sort(),
    );
    expect(
      docs.filter((row) => row.state !== "parsed").every((row) => row.reason),
    ).toBe(true);
  });

  // Trace: FR-052-AC-5
  // TC-1166
  it("TC-1166", "retains identity and occurrence signals", () => {
    const event = built().inventory.documents.find(
      (row) => row.path === "spec/event.md",
    );
    expect(event).toMatchObject({
      declaredType: "event",
      documentId: "EVT-001",
    });
    expect(event.signals).toEqual(
      expect.arrayContaining(["timestamp", "result", "evidence"]),
    );
  });

  // Trace: FR-052-AC-6
  // TC-1167
  it("TC-1167", "keeps explicit no-instance observations", () => {
    const betaFoo = built().inventory.declarations.find(
      (row) => row.qualifiedName === "beta::Foo",
    );
    expect(betaFoo.instances).toEqual([]);
    expect(betaFoo.observations).toContain("no-instance");
  });

  // Trace: FR-052-AC-7
  // TC-1168
  it("TC-1168", "reconciles every denominator", () => {
    expect(
      Object.values(built().inventory.denominators).every(
        (row) => row.reconciled,
      ),
    ).toBe(true);
    const malformed = auditInput();
    malformed.modules[1].manifestText =
      malformed.modules[1].manifestText.replace(
        "artifact_types:\n",
        "artifact_types:\n  - frontmatter_schema_ref: schemas/nameless.json\n",
      );
    const incomplete = buildSemanticAudit(malformed) as SemanticAudit;
    expect(incomplete.inventory.denominators.declarations).toMatchObject({
      source: 5,
      inventoried: 4,
      reconciled: false,
    });
    expect(incomplete.summary.verdict).not.toBe("clean");
  });

  // Trace: FR-053-AC-1
  // TC-1169
  it("TC-1169", "assesses every required axis", () => {
    const names = built().typeFit.axes;
    expect(names).toEqual([
      "vocabulary",
      "structure",
      "definitionOccurrence",
      "identity",
      "versioning",
      "provenance",
      "lifecycle",
      "relationships",
      "roundTrip",
      "generatedCode",
      "accumulation",
    ]);
    expect(
      built().typeFit.assessments.every(
        (row) => Object.keys(row.axes).length === names.length,
      ),
    ).toBe(true);
  });

  // Trace: FR-053-AC-2
  // TC-1170
  it(
    "TC-1170",
    "uses closed axis and confidence vocabularies with evidence",
    () => {
      for (const assessment of built().typeFit.assessments)
        for (const axis of Object.values(assessment.axes)) {
          expect([
            "supported",
            "partial",
            "conflict",
            "missing",
            "not-applicable",
          ]).toContain(axis.status);
          expect(["high", "medium", "low"]).toContain(axis.confidence);
          expect(
            axis.evidence.length + Number(Boolean(axis.explanation)),
          ).toBeGreaterThan(0);
        }
    },
  );

  // Trace: FR-053-AC-3
  // TC-1171
  it("TC-1171", "assigns exactly one closed disposition", () => {
    for (const row of built().typeFit.assessments)
      expect([
        "fits",
        "fits-with-mapping",
        "incomplete",
        "conflict",
        "representation-local",
        "deferred",
      ]).toContain(row.disposition);
    const incomplete = auditInput();
    incomplete.modules[1].manifestText = manifest(
      "beta",
      "object_types:\n  - name: Placeholder\n    data_schema: { type: object }\n",
    );
    incomplete.modules[1].moduleFiles = [];
    expect(
      (buildSemanticAudit(incomplete) as SemanticAudit).summary.verdict,
    ).toBe("findings");
  });

  // Trace: FR-053-AC-4
  // TC-1172
  it("TC-1172", "does not promote placeholder schemas", () => {
    const event = built().typeFit.assessments.find(
      (row) => row.qualifiedName === "alpha::event",
    );
    expect(event.flags).toContain("placeholder-schema");
    expect(event.axes.generatedCode.status).not.toBe("supported");
    expect(event.axes.roundTrip.status).not.toBe("supported");
  });

  // Trace: FR-053-AC-5
  // TC-1173
  it("TC-1173", "retains qualified duplicate conflicts", () => {
    expect(
      built().conflicts.some(
        (row) =>
          row.kind === "duplicate-type" && row.qualifiedTypes.length === 2,
      ),
    ).toBe(true);
  });

  // Trace: FR-053-AC-6
  // TC-1174
  it("TC-1174", "flags schema encoded blobs", () => {
    const opaque = built().typeFit.assessments.find(
      (row) => row.qualifiedName === "beta::opaque",
    );
    expect(opaque.flags).toContain("encoded-structure-blob");
    expect(opaque.axes.structure.status).not.toBe("supported");
  });

  // Trace: FR-053-AC-7
  // TC-1175
  it("TC-1175", "records definition occurrence plane confusion", () => {
    const event = built().typeFit.assessments.find(
      (row) => row.qualifiedName === "alpha::event",
    );
    expect(event.flags).toContain("occurrence-signals");
    expect(event.axes.definitionOccurrence.status).toBe("conflict");
  });

  // Trace: FR-053-AC-8
  // TC-1176
  it("TC-1176", "evaluates the required missing concepts", () => {
    expect(built().missingTypes.map((row) => row.concept)).toEqual([
      "run",
      "result",
      "evidence",
      "report",
      "relationship",
      "identity",
      "version",
      "provenance",
      "lifecycle",
    ]);
  });

  // Trace: FR-054-AC-1
  // TC-1177
  it("TC-1177", "creates the complete artifact family", () => {
    expect([...createArtifactFiles(built()).keys()].sort()).toEqual(
      [
        "conflicts.json",
        "inventory.json",
        "manifest.json",
        "missing-types.json",
        "report.md",
        "repository-impact.json",
        "review.md",
        "snapshot.json",
        "type-fit.json",
      ].sort(),
    );
  });

  // Trace: FR-054-AC-2
  // TC-1178
  it("TC-1178", "gives every ledger row stable complete fields", () => {
    for (const row of [...built().conflicts, ...built().missingTypes])
      expect(row).toMatchObject({
        id: expect.any(String),
        severity: expect.any(String),
        status: expect.any(String),
        confidence: expect.any(String),
        evidence: expect.any(Array),
        rationale: expect.any(String),
        nextBoundary: expect.any(String),
      });
  });

  // Trace: FR-054-AC-3
  // TC-1179
  it("TC-1179", "closes all named repository boundaries", () => {
    expect(built().repositoryImpact).toHaveLength(
      auditInput().repositoryBoundaries.length,
    );
    for (const row of built().repositoryImpact)
      expect(row).toMatchObject({
        impact: expect.stringMatching(/^(none|candidate|required|unknown)$/),
        effort: expect.any(String),
        risk: expect.any(String),
        wave: expect.any(String),
        confidence: expect.any(String),
        rationale: expect.any(String),
      });
  });

  // Trace: FR-054-AC-4
  // TC-1180
  it("TC-1180", "renders report and SpecReview from canonical ids", () => {
    const files = createArtifactFiles(built());
    for (const row of [...built().conflicts, ...built().missingTypes]) {
      expect(files.get("report.md")).toContain(row.id);
      expect(files.get("review.md")).toContain(row.id);
    }
    expect(files.get("report.md")).not.toMatch(/[ \t]+\n/);
    expect(files.get("review.md")).not.toMatch(/[ \t]+\n/);
  });

  // Trace: FR-054-AC-5
  // TC-1181
  it("TC-1181", "rejects missing stale and count disagreeing artifacts", () => {
    const files = createArtifactFiles(built());
    expect(verifyArtifactFiles(files).valid).toBe(true);
    files.set(
      "inventory.json",
      files.get("inventory.json")!.replace('"source": 2', '"source": 3'),
    );
    expect(() => verifyArtifactFiles(files)).toThrow(/digest|count|canonical/i);

    const identityFiles = createArtifactFiles(built());
    const manifest = JSON.parse(identityFiles.get("manifest.json")!) as {
      contentIdentity: string;
    };
    manifest.contentIdentity = `sha256:${"0".repeat(64)}`;
    identityFiles.set("manifest.json", serializeCanonical(manifest));
    expect(() => verifyArtifactFiles(identityFiles)).toThrow(
      /content identity disagrees/,
    );

    const duplicateManifestFiles = createArtifactFiles(built());
    const duplicateManifest = JSON.parse(
      duplicateManifestFiles.get("manifest.json")!,
    ) as {
      artifacts: Array<{
        path: string;
        schemaVersion: string;
        digest: string;
        bytes: number;
      }>;
    };
    duplicateManifest.artifacts[1] = duplicateManifest.artifacts[0];
    duplicateManifestFiles.set(
      "manifest.json",
      serializeCanonical(duplicateManifest),
    );
    expect(() => verifyArtifactFiles(duplicateManifestFiles)).toThrow(
      /paths disagree/,
    );

    const projectionFiles = createArtifactFiles(built());
    const omittedId = built().conflicts[0].id;
    const report = projectionFiles
      .get("report.md")!
      .split("\n")
      .filter((line) => !line.includes(omittedId))
      .join("\n");
    projectionFiles.set("report.md", report);
    const projectionManifest = JSON.parse(
      projectionFiles.get("manifest.json")!,
    ) as {
      artifacts: Array<{ path: string; digest: string; bytes: number }>;
    };
    const reportRecord = projectionManifest.artifacts.find(
      (row) => row.path === "report.md",
    )!;
    reportRecord.digest = `sha256:${createHash("sha256")
      .update(report)
      .digest("hex")}`;
    reportRecord.bytes = Buffer.byteLength(report);
    projectionFiles.set(
      "manifest.json",
      serializeCanonical(projectionManifest),
    );
    expect(() => verifyArtifactFiles(projectionFiles)).toThrow(
      /report.md disagrees with canonical audit data/,
    );
  });

  // Trace: FR-054-AC-6
  // TC-1182
  it("TC-1182", "isolates time from equal-input content identity", () => {
    const a = createArtifactFiles(built());
    const next = auditInput();
    next.timestamp = "2026-08-31T20:00:00.000Z";
    const b = createArtifactFiles(buildSemanticAudit(next));
    expect(JSON.parse(a.get("manifest.json")!).contentIdentity).toBe(
      JSON.parse(b.get("manifest.json")!).contentIdentity,
    );
  });

  // Trace: FR-055-AC-1
  // TC-1183
  it(
    "TC-1183",
    "reconciles findings to plane authority owner and decision",
    () => {
      for (const row of [...built().conflicts, ...built().missingTypes])
        expect(row.reconciliation).toMatchObject({
          plane: expect.any(String),
          authority: expect.any(String),
          owner: expect.any(String),
          decision: expect.any(String),
        });
    },
  );

  // Trace: FR-055-AC-2
  // TC-1184
  it("TC-1184", "classifies core data overlap without shadow contracts", () => {
    expect(
      built().reconciliation.coreData.every((row) =>
        ["reuse", "extension", "mapping", "conflict", "unrelated"].includes(
          row.disposition,
        ),
      ),
    ).toBe(true);
    expect(built().reconciliation.shadowContracts).toEqual([]);
  });

  // Trace: FR-055-AC-3
  // TC-1185
  it("TC-1185", "preserves the pinned Quire boundary", () => {
    expect(built().reconciliation.quire).toMatchObject({
      revision: SHA_A,
      preservesBoundary: true,
      responsibilities: expect.arrayContaining([
        "parse",
        "validate",
        "extract",
        "address",
        "byte-splice",
      ]),
    });
  });

  // Trace: FR-055-AC-4
  // TC-1186
  it("TC-1186", "gates every major interference boundary", () => {
    expect(
      built()
        .reconciliation.followUpBoundaries.filter(
          (row) => row.majorInterference,
        )
        .every((row) => row.gate),
    ).toBe(true);
  });

  // Trace: FR-055-AC-5
  // TC-1187
  it("TC-1187", "blocks signoff on fresh census drift", () => {
    expect(freshCensus(built().snapshot, built().snapshot).fresh).toBe(true);
    const drifted = structuredClone(built().snapshot);
    drifted.modules[0].resolvedSha = SHA_B;
    expect(freshCensus(built().snapshot, drifted)).toMatchObject({
      fresh: false,
      signoffBlocked: true,
    });
  });

  // Trace: NFR-015
  // TC-1188
  it("TC-1188", "proves complete module coverage", () => {
    expect(built().inventory.denominators.modules).toMatchObject({
      source: 2,
      inventoried: 2,
      reconciled: true,
    });
  });

  // Trace: NFR-015
  // TC-1189
  it("TC-1189", "proves complete axis coverage", () => {
    expect(
      built().inventory.declarations.length * built().typeFit.axes.length,
    ).toBe(
      built().typeFit.assessments.reduce(
        (sum: number, row) => sum + Object.keys(row.axes).length,
        0,
      ),
    );
  });

  // Trace: NFR-015
  // TC-1190
  it("TC-1190", "proves complete Markdown state coverage", () => {
    expect(built().inventory.denominators.documents).toMatchObject({
      reconciled: true,
    });
    expect(built().inventory.denominators.documents.source).toBe(
      built().inventory.documents.length,
    );
  });

  // Trace: NFR-015
  // TC-1191
  it("TC-1191", "proves byte identical equal input artifacts", () => {
    expect([...createArtifactFiles(built())]).toEqual([
      ...createArtifactFiles(buildSemanticAudit(auditInput())),
    ]);
  });

  // Trace: NFR-016
  // TC-1192
  it("TC-1192", "writes only below the configured output root", () => {
    const root = mkdtempSync(join(tmpdir(), "semantic-audit-"));
    const sentinel = join(root, "sentinel");
    writeFileSync(sentinel, "unchanged");
    chmodSync(sentinel, 0o444);
    const output = join(root, "output");
    writeArtifactFiles(output, createArtifactFiles(built()));
    expect(readFileSync(sentinel, "utf8")).toBe("unchanged");
    expect(statSync(join(output, "manifest.json")).isFile()).toBe(true);

    const redirected = join(root, "redirected");
    writeFileSync(redirected, "unchanged");
    const hostileOutput = join(root, "hostile-output");
    mkdirSync(hostileOutput);
    symlinkSync(redirected, join(hostileOutput, "snapshot.json"));
    expect(() =>
      writeArtifactFiles(hostileOutput, createArtifactFiles(built())),
    ).toThrow(/not a regular file/);
    expect(readFileSync(redirected, "utf8")).toBe("unchanged");
  });

  // Trace: NFR-016
  // TC-1193
  it("TC-1193", "enforces the changed path allowlist", () => {
    expect(isAllowedAuditPath("scripts/lib/semantic-module-type-fit.mjs")).toBe(
      true,
    );
    expect(
      isAllowedAuditPath("analysis/semantic-module-type-fit/report.md"),
    ).toBe(true);
    expect(isAllowedAuditPath(".prettierignore")).toBe(true);
    for (const path of [
      "src/index.ts",
      "default-modules.yaml",
      "schemas/x.json",
      "migrations/1.sql",
      "generated/python/model.py",
    ])
      expect(isAllowedAuditPath(path)).toBe(false);
  });

  it("classifies direct Markdown parser edge cases", () => {
    expect(parseMarkdownRecord("x.md", "# no frontmatter", {})).toMatchObject({
      state: "untyped",
    });
    expect(parseMarkdownRecord("x.md", "---\ntype: [\n---", {})).toMatchObject({
      state: "invalid",
    });
  });
});
