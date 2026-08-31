import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { isAllowedAuditPath } from "../scripts/lib/semantic-module-type-fit.mjs";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const architectureRoot = join(repoRoot, "docs", "semantic-module-architecture");
const architectureDelivery =
  "4a82644ad3cf75770cc53ef3812e3b13e80b516d";

const architectureFiles = [
  "index.md",
  "planes-and-authority.md",
  "ownership-and-boundaries.md",
  "dynamic-and-generated.md",
  "decision-ledger.md",
  "adr/index.md",
  "adr/0001-authority-by-concern.md",
  "adr/0002-preserve-quire-quoin-boundaries.md",
] as const;

function architecture(name: (typeof architectureFiles)[number]): string {
  return readFileSync(join(architectureRoot, name), "utf8");
}

function expectAll(text: string, fragments: readonly string[]): void {
  const normalizedText = text.replace(/\s+/g, " ");
  for (const fragment of fragments) {
    expect(
      normalizedText,
      `missing architecture contract fragment: ${fragment}`,
    ).toContain(fragment.replace(/\s+/g, " "));
  }
}

function gitLines(args: string[]): string[] {
  try {
    return execFileSync("git", args, { cwd: repoRoot, encoding: "utf8" })
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

function changedPaths(): string[] {
  // Once the architecture-only branch has shipped, preserve its scope proof by
  // checking the delivered merge itself. Comparing every future topic branch
  // with origin/main would incorrectly make this historical gate reject all
  // subsequent product work.
  const delivered = gitLines([
    "rev-parse",
    "--verify",
    `${architectureDelivery}^{commit}`,
  ])[0];
  if (delivered) {
    return gitLines([
      "diff",
      "--name-only",
      `${architectureDelivery}^1`,
      architectureDelivery,
    ]);
  }

  const base = gitLines(["merge-base", "origin/main", "HEAD"])[0];
  const committed = base
    ? gitLines(["diff", "--name-only", `${base}...HEAD`])
    : gitLines(["diff", "--name-only", "HEAD~1...HEAD"]);
  return [
    ...committed,
    ...gitLines(["diff", "--name-only"]),
    ...gitLines(["diff", "--name-only", "--cached"]),
    ...gitLines(["ls-files", "--others", "--exclude-standard"]),
  ].filter((path, index, all) => all.indexOf(path) === index);
}

function isAllowedArchitecturePath(path: string): boolean {
  return (
    isAllowedAuditPath(path) ||
    path.startsWith("docs/semantic-module-architecture/") ||
    path.startsWith("plan/PLAN-002-semantic-module-architecture/") ||
    path.startsWith("spec/reviews/") ||
    /^reviews\/2026-08-29-semantic-module-architecture-(code-review|gap-analysis)\.md$/.test(
      path,
    ) ||
    path === "tests/semantic-module-architecture.test.ts" ||
    [
      "spec/functional/FR-046-record-semantic-data-planes.md",
      "spec/functional/FR-047-allocate-semantic-module-ownership.md",
      "spec/functional/FR-048-declare-authority-by-concern.md",
      "spec/functional/FR-049-preserve-dynamic-and-generated-modules.md",
      "spec/functional/FR-050-reconcile-quire-decisions.md",
      "spec/non-functional/NFR-013-traceable-semantic-architecture.md",
      "spec/non-functional/NFR-014-non-disruptive-architecture-record.md",
      "spec/usecase/US-013-reason-about-semantic-module-boundaries.md",
      "spec/functional/index.md",
      "spec/non-functional/index.md",
      "spec/usecase/index.md",
      "spec/spec.md",
      "spec/matrix.md",
      "spec/log.md",
    ].includes(path)
  );
}

describe("semantic-module architecture contract", () => {
  // Trace: FR-046-AC-1
  // TC-1125
  it("defines all four semantic data planes", () => {
    expectAll(architecture("planes-and-authority.md"), [
      "Meta plane",
      "Definition plane",
      "Execution and observation plane",
      "Presentation plane",
    ]);
  });

  // Trace: FR-046-AC-2
  // TC-1126
  it("separates definitions, occurrences, and presentations", () => {
    expectAll(architecture("planes-and-authority.md"), [
      "`TestCase`",
      "`TestExecution`",
      "run report",
      "linked objects",
    ]);
  });

  // Trace: FR-046-AC-3
  // TC-1127
  it("keeps structural kinds independent from semantic roles", () => {
    expectAll(architecture("planes-and-authority.md"), [
      "Structural kind and semantic role are independent",
      "No universal `SemanticObject` runtime envelope",
    ]);
  });

  // Trace: FR-046-AC-4
  // TC-1128
  it("indexes every record and labels decision status", () => {
    const index = architecture("index.md");
    for (const file of architectureFiles.slice(1)) {
      expect(index).toContain(`](${file})`);
    }
    expectAll(index, ["normative", "provisional", "external gate"]);
  });

  // Trace: FR-047-AC-1
  // TC-1129
  it("preserves Quire ownership and exclusions", () => {
    expectAll(architecture("ownership-and-boundaries.md"), [
      "parse, validate, extract, address, and byte-splice",
      "template rendering",
      "cross-language generation",
      "schema-package publication",
      "registry sourcing",
    ]);
  });

  // Trace: FR-047-AC-2
  // TC-1130
  it("preserves Quoin ownership and exclusions", () => {
    expectAll(architecture("ownership-and-boundaries.md"), [
      "catalog discovery, locks, installation, update",
      "authoring-contract discovery, skills, workflows",
      "parser semantics",
      "compiler implementation",
      "runtime persistence",
      "consumer adapters",
    ]);
  });

  // Trace: FR-047-AC-3
  // TC-1131
  it("separates compiler and module-repository ownership", () => {
    expectAll(architecture("ownership-and-boundaries.md"), [
      "`filament-core-data`",
      "semantic kernel, IR, compatibility rules, compiler, and emitters",
      "Module repositories",
      "vocabulary, constraints, archetypes, skeletons, mappings, examples, and semantic versions",
    ]);
  });

  // Trace: FR-047-AC-4
  // TC-1132
  it("allocates consumer adapters and state without semantic forks", () => {
    expectAll(architecture("ownership-and-boundaries.md"), [
      "application adapters",
      "API and IPC projections",
      "persistence mappings and migrations",
      "runtime state",
      "UI presentation",
      "must not fork a shared semantic identity",
    ]);
  });

  // Trace: FR-047-AC-5
  // TC-1133
  it("retains accepted validation levels and capability roles", () => {
    expectAll(architecture("ownership-and-boundaries.md"), [
      "L0",
      "L1",
      "L2",
      "Validator",
      "Advisor",
      "Generator",
      "Auditor",
      "Consumer CI always executes L2 work",
    ]);
  });

  // Trace: FR-048-AC-1
  // TC-1134
  it("keeps typed Markdown authoritative for authored knowledge", () => {
    expectAll(architecture("planes-and-authority.md"), [
      "Reviewed typed Markdown",
      "authored durable knowledge",
      "extracted JSON, graph records, search indexes, embeddings, and rendered views",
    ]);
  });

  // Trace: FR-048-AC-2
  // TC-1135
  it("keeps package sources authoritative over generated language types", () => {
    expectAll(architecture("planes-and-authority.md"), [
      "Accepted schema/package source plus package metadata",
      "generated Rust, TypeScript, and Python",
      "never independent authorities",
    ]);
  });

  // Trace: FR-048-AC-3
  // TC-1136
  it("records the JSON Schema fallback without promoting TypeSpec", () => {
    expectAll(architecture("decision-ledger.md"), [
      "modular JSON Schema 2020-12",
      "TypeSpec",
      "unpromoted",
      "ADR-0004",
      "human resolution",
    ]);
  });

  // Trace: FR-048-AC-4
  // TC-1137
  it("keeps transactional and observation stores authoritative", () => {
    expectAll(architecture("planes-and-authority.md"), [
      "Owning PostgreSQL database or event store",
      "Owning run, evidence, or event store",
      "Markdown report",
      "presentation projection",
    ]);
  });

  // Trace: FR-048-AC-5
  // TC-1138
  it("classifies wire, analytical, and export projections", () => {
    expectAll(architecture("planes-and-authority.md"), [
      "Protobuf",
      "JSON",
      "Avro",
      "Arrow",
      "Parquet",
      "CSV",
      "TSV",
      "declared loss",
    ]);
  });

  // Trace: FR-048-AC-6
  // TC-1139
  it("stops on competing authorities", () => {
    expectAll(architecture("planes-and-authority.md"), [
      "promotion stops",
      "projection",
      "concerns are distinct",
      "reviewed ADR and migration",
      "Last-writer-wins is rejected",
    ]);
  });

  // Trace: FR-049-AC-1
  // TC-1140
  it("preserves unknown dynamic module data", () => {
    expectAll(architecture("dynamic-and-generated.md"), [
      "previously unknown module",
      "namespaced",
      "generic validated data",
    ]);
  });

  // Trace: FR-049-AC-2
  // TC-1141
  it("defines finite generated package exports", () => {
    expectAll(architecture("dynamic-and-generated.md"), [
      "finite generated export set",
      "native Rust, TypeScript, and Python types",
      "versioned package dependencies",
    ]);
  });

  // Trace: FR-049-AC-3
  // TC-1142
  it("requires an explicit unknown-extension policy", () => {
    expectAll(architecture("dynamic-and-generated.md"), [
      "preserve, reject, or surface",
      "named profile",
      "never misclassify",
    ]);
  });

  // Trace: FR-049-AC-4
  // TC-1143
  it("makes native regeneration elective", () => {
    expectAll(architecture("dynamic-and-generated.md"), [
      "does not require regeneration",
      "elects to adopt",
    ]);
  });

  // Trace: FR-049-AC-5
  // TC-1144
  it("separates distribution from generation and mapping declarations", () => {
    expectAll(architecture("dynamic-and-generated.md"), [
      "catalog and distribution",
      "exports",
      "targets",
      "mappings",
      "profiles",
      "distinct concerns",
    ]);
  });

  // Trace: FR-050-AC-1
  // TC-1145
  it("keeps the unified archetype decision structural", () => {
    expectAll(architecture("decision-ledger.md"), [
      "ADR-0003",
      "preserved",
      "structural parsing model",
      "not a universal semantic runtime base class",
    ]);
  });

  // Trace: FR-050-AC-2
  // TC-1146
  it("preserves direct and document-boundary canonical Markdown", () => {
    expectAll(architecture("decision-ledger.md"), [
      "ADR-0004",
      "direct typed Markdown",
      "canonical Markdown within the document boundary",
    ]);
  });

  // Trace: FR-050-AC-3
  // TC-1147
  it("retires draft rendering ownership while keeping byte-splicing", () => {
    expectAll(architecture("decision-ledger.md"), [
      "ADR-0002",
      "partially superseded",
      "rendering responsibility is historical",
      "byte-splicing remains preserved",
    ]);
  });

  // Trace: FR-050-AC-4
  // TC-1148
  it("keeps the accepted validation-level decision governing", () => {
    expectAll(architecture("decision-ledger.md"), [
      "ADR-0011",
      "Accepted",
      "governing",
    ]);
  });

  // Trace: FR-050-AC-5
  // TC-1149
  it("prevents Quire, Quoin, and compiler boundary regression", () => {
    expectAll(architecture("adr/0002-preserve-quire-quoin-boundaries.md"), [
      "Quire does not become a renderer or cross-language generator",
      "Quoin does not become the parser or semantic compiler",
    ]);
  });

  // Trace: FR-050-AC-6
  // TC-1150
  it("records a complete external-decision identity contract", () => {
    expectAll(architecture("decision-ledger.md"), [
      "Repository",
      "Path",
      "Status",
      "Reviewed revision or date",
      "Disposition",
    ]);
  });

  // Trace: NFR-013-M-1
  // Trace: NFR-013-M-2
  // TC-1151
  it("gives every external decision complete identity metadata", () => {
    const ledger = architecture("decision-ledger.md");
    const rows = ledger
      .split("\n")
      .filter((line) => /^\|\s+\x60[^\x60]+\x60\s+\|/.test(line));
    expect(rows.length).toBeGreaterThanOrEqual(6);
    for (const row of rows) {
      const cells = row
        .split("|")
        .slice(1, -1)
        .map((cell) => cell.trim());
      expect(cells).toHaveLength(6);
      for (const cell of cells) expect(cell).not.toBe("");
    }
  });

  // Trace: NFR-013-M-3
  // TC-1152
  it("resolves every local architecture link", () => {
    for (const file of architectureFiles) {
      const sourcePath = join(architectureRoot, file);
      const source = architecture(file);
      for (const match of source.matchAll(/\[[^\]]+\]\(([^)]+)\)/g)) {
        const target = match[1].split("#", 1)[0];
        if (!target || /^(?:https?:|ix:|#)/.test(target)) continue;
        const resolved = resolve(dirname(sourcePath), target);
        expect(
          existsSync(resolved),
          `${relative(repoRoot, sourcePath)} -> ${target}`,
        ).toBe(true);
      }
    }
  });

  // Trace: NFR-013-M-4
  // TC-1153
  it("does not present provisional decisions as normative", () => {
    const ledger = architecture("decision-ledger.md").replace(/\s+/g, " ");
    expect(ledger).toContain("TypeSpec remains unpromoted");
    expect(ledger).toContain("ADR-0004 remains provisional");
    expect(ledger).not.toContain("TypeSpec is normative");
  });

  // Trace: NFR-014-M-1
  // Trace: NFR-014-M-2
  // TC-1154
  it("keeps the branch inside the architecture-only path allowlist", () => {
    const disallowed = changedPaths().filter(
      (path) => !isAllowedArchitecturePath(path),
    );
    expect(disallowed).toEqual([]);
  });
});
