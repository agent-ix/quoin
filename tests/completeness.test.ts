/**
 * FR-037 — declared-vocabulary completeness and its verdict policy
 * (TC-206..TC-217).
 *
 * Six of the twelve criteria are stated over `quoin completeness` rather than
 * over `assessVocabulary`, because the defect this program keeps finding is a
 * matrix reading ✅ over a capability no invocation reaches.
 */

import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { beforeEach, describe, expect, it, vi } from "vitest";

import Completeness from "../src/commands/completeness.js";
import {
  assessBundle,
  assessVocabulary,
  loadVocabularyCoverage,
  verdictFor,
  writtenReasonFor,
  type CompletenessFinding,
  type DocumentClaims,
  type VocabularyDeclaration,
} from "../src/completeness/index.js";

/** A module declaring one vocabulary with a three-value enum. */
function moduleWith(options: { justifiedAbsence?: boolean } = {}): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-vocab-"));
  mkdirSync(join(root, "schemas"), { recursive: true });
  writeFileSync(
    join(root, "schemas", "nfr.schema.json"),
    JSON.stringify({
      type: "object",
      properties: {
        quality_attribute: { enum: ["reliability", "security", "safety"] },
      },
    }),
  );
  writeFileSync(
    join(root, "manifest.yaml"),
    [
      "manifest_version: 1",
      "name: vocab-fixture",
      "version: 0.0.0",
      "artifact_types:",
      "  - name: NFR",
      "    frontmatter_schema_ref: schemas/nfr.schema.json",
      "traceability:",
      "  vocabulary_coverage:",
      "  - name: quality-characteristics",
      "    from: NFR",
      "    field: quality_attribute",
      "    check: unowned-quality-characteristic",
      ...(options.justifiedAbsence === false
        ? []
        : ["    justified_absence_field: quality_attributes_not_applicable"]),
      "",
    ].join("\n"),
  );
  return root;
}

/** A bundle whose documents carry the frontmatter given. */
function bundleWith(documents: Record<string, string>): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-bundle-"));
  const spec = join(root, "spec");
  mkdirSync(spec, { recursive: true });
  for (const [name, content] of Object.entries(documents)) {
    writeFileSync(join(spec, name), content);
  }
  return root;
}

const NFR = (attribute: string) =>
  `---\nid: NFR-001\ntype: NFR\nquality_attribute: ${attribute}\n---\n\n## Statement\n\nIt holds.\n`;

const declaration = (): VocabularyDeclaration => ({
  name: "quality-characteristics",
  from: "NFR",
  field: "quality_attribute",
  check: "unowned-quality-characteristic",
  justifiedAbsenceField: "quality_attributes_not_applicable",
  values: ["reliability", "security", "safety"],
  moduleName: "vocab-fixture",
});

const document = (over: Partial<DocumentClaims> = {}): DocumentClaims => ({
  path: "spec.md",
  claims: [],
  excuses: [],
  body: "",
  ...over,
});

let logged: string[];
let warned: string[];

beforeEach(() => {
  logged = [];
  warned = [];
  vi.spyOn(Completeness.prototype, "log").mockImplementation((m?: string) => {
    logged.push(String(m ?? ""));
  });
  vi.spyOn(Completeness.prototype, "warn").mockImplementation(((m: string) => {
    warned.push(String(m));
    return m;
  }) as never);
});

describe("reading the declared vocabulary", () => {
  // Trace: FR-037-AC-1
  it("reads the values from module data rather than minting a list", () => {
    // The ticket proposed walking a hardcoded 9-item ISO 25010 list; the module
    // declares 12. A second list is the drift this whole area exists to avoid,
    // so the count must come from the declaration.
    const { declarations, unresolved } = loadVocabularyCoverage([moduleWith()]);
    expect(unresolved).toEqual([]);
    expect(declarations).toHaveLength(1);
    expect(declarations[0].values).toEqual([
      "reliability",
      "security",
      "safety",
    ]);
    expect(declarations[0].justifiedAbsenceField).toBe(
      "quality_attributes_not_applicable",
    );
  });

  // Trace: FR-037-AC-2
  it("reports a vocabulary it cannot resolve instead of dropping it", () => {
    // The engine will still emit findings for it. A declaration quoin silently
    // ignores becomes findings quoin cannot explain, which is worse than none.
    const root = mkdtempSync(join(tmpdir(), "quoin-vocab-bad-"));
    writeFileSync(
      join(root, "manifest.yaml"),
      [
        "manifest_version: 1",
        "name: broken",
        "artifact_types:",
        "  - name: NFR",
        "traceability:",
        "  vocabulary_coverage:",
        "  - name: quality-characteristics",
        "    from: NFR",
        "    field: quality_attribute",
        "    check: unowned-quality-characteristic",
        "",
      ].join("\n"),
    );
    const { declarations, unresolved } = loadVocabularyCoverage([root]);
    expect(declarations).toEqual([]);
    expect(unresolved[0].name).toBe("quality-characteristics");
    expect(unresolved[0].reason).toMatch(/no frontmatter schema/);
  });
});

describe("the verdict policy", () => {
  // Trace: FR-037-AC-3
  it("reports a value no document claims as an unowned gap, at medium", () => {
    const { findings, rollup } = assessVocabulary(declaration(), [
      document({ claims: ["reliability"] }),
    ]);
    expect(rollup).toEqual({
      vocabulary: "quality-characteristics",
      declared: 3,
      owned: 1,
      excused: 0,
      unowned: 2,
    });
    expect(findings.map((f) => f.value).sort()).toEqual(["safety", "security"]);
    expect(findings.every((f) => f.kind === "unowned")).toBe(true);
    expect(findings.every((f) => f.severity === "medium")).toBe(true);
  });

  // Trace: FR-037-AC-4
  it("rates an exclusion with no written reason above an admitted gap", () => {
    // The asymmetry IS the policy. Saying nothing about safety is a gap a reader
    // can see; excusing it with no reason asserts completeness with nothing
    // behind it AND removes the finding that would have prompted the work.
    const { findings } = assessVocabulary(declaration(), [
      document({ excuses: ["safety"] }),
    ]);
    const excluded = findings.find((f) => f.value === "safety");
    expect(excluded?.kind).toBe("unjustified-exclusion");
    expect(excluded?.severity).toBe("high");
    expect(excluded?.document).toBe("spec.md");
    expect(findings.find((f) => f.value === "security")?.severity).toBe(
      "medium",
    );
  });

  // Trace: FR-037-AC-5
  it("names an exclusion whose value is not in the vocabulary", () => {
    // A typo excuses nothing — the real value keeps reporting — while reading,
    // to whoever wrote it, as handled.
    const { findings, rollup } = assessVocabulary(declaration(), [
      document({ excuses: ["saftey"] }),
    ]);
    const typo = findings.find((f) => f.value === "saftey");
    expect(typo?.kind).toBe("undeclared-exclusion");
    expect(typo?.severity).toBe("high");
    expect(typo?.message).toMatch(/excuses nothing/);
    expect(rollup.excused).toBe(0);
    expect(
      findings.some((f) => f.value === "safety" && f.kind === "unowned"),
    ).toBe(true);
  });

  // Trace: FR-037-AC-6
  it("accepts a table row naming the value with a real reason", () => {
    const { findings } = assessVocabulary(declaration(), [
      document({
        excuses: ["safety"],
        body:
          "| Characteristic | Justification |\n|---|---|\n" +
          "| safety | quoin controls no physical process. |\n",
      }),
    ]);
    expect(findings.some((f) => f.value === "safety")).toBe(false);
  });

  // Trace: FR-037-AC-7
  it("does not accept a non-answer as a reason", () => {
    // `-` and `TBD` are an author acknowledging the question and declining to
    // answer it, which is precisely what an unjustified exclusion is.
    for (const cell of ["-", "TBD", "n/a", "none", "no"]) {
      expect(
        writtenReasonFor("safety", `| safety | ${cell} |\n`),
        `"${cell}" must not read as a reason`,
      ).toBeNull();
    }
    expect(
      writtenReasonFor("safety", "| safety | controls no hardware |"),
    ).toBe("controls no hardware");
    // A mention in prose is not a justification: "safety" occurs in any document
    // that discusses safety, so only a row naming it counts.
    expect(
      writtenReasonFor("safety", "Safety is out of scope for this component."),
    ).toBeNull();
  });

  // Trace: FR-037-AC-8
  it("fails on a high finding and only escalates a gap under --strict", () => {
    const gap: CompletenessFinding[] = [
      {
        vocabulary: "v",
        value: "safety",
        kind: "unowned",
        severity: "medium",
        message: "m",
      },
    ];
    const claim: CompletenessFinding[] = [
      {
        vocabulary: "v",
        value: "safety",
        kind: "unjustified-exclusion",
        severity: "high",
        message: "m",
      },
    ];
    expect(verdictFor([], false)).toBe("PASS");
    expect(verdictFor([], true)).toBe("PASS");
    expect(verdictFor(gap, false)).toBe("CONDITIONAL");
    expect(verdictFor(gap, true)).toBe("FAIL");
    expect(verdictFor(claim, false)).toBe("FAIL");
  });
});

describe("quoin completeness", () => {
  // Trace: FR-037-AC-9
  it("reports the gaps over a real bundle and exits 0 while advisory", async () => {
    const repo = bundleWith({ "NFR-001-a.md": NFR("reliability") });
    await Completeness.run([
      "--repo",
      repo,
      "--module",
      moduleWith(),
      "--json",
    ]);
    const report = JSON.parse(logged.join("\n"));
    expect(report.verdict).toBe("CONDITIONAL");
    expect(report.rollups[0]).toMatchObject({ owned: 1, unowned: 2 });
    expect(report.findings.map((f: CompletenessFinding) => f.value)).toEqual([
      "safety",
      "security",
    ]);
  });

  // Trace: FR-037-AC-10
  it("exits non-zero on an unjustified exclusion, without --strict", async () => {
    const repo = bundleWith({
      "spec.md":
        "---\ntype: Spec\nquality_attributes_not_applicable: [safety]\n---\n\n# Spec\n",
      "NFR-001-a.md": NFR("reliability"),
    });
    await expect(
      Completeness.run(["--repo", repo, "--module", moduleWith()]),
    ).rejects.toMatchObject({ oclif: { exit: 1 } });
    expect(logged.join("\n")).toMatch(/FAIL — 1 high/);
  });

  // Trace: FR-037-AC-11
  it("exits non-zero under --strict on gaps alone", async () => {
    const repo = bundleWith({ "NFR-001-a.md": NFR("reliability") });
    const module = moduleWith();
    // Same bundle, both ways round: advisory passes, strict does not. Asserting
    // the pair is what makes --strict a policy rather than a second code path.
    await Completeness.run(["--repo", repo, "--module", module]);
    expect(logged.join("\n")).toMatch(/CONDITIONAL/);
    await expect(
      Completeness.run(["--repo", repo, "--module", module, "--strict"]),
    ).rejects.toMatchObject({ oclif: { exit: 1 } });
  });

  // Trace: FR-037-AC-12
  it("says nothing was checked when no module declares a vocabulary", async () => {
    // Not a pass. A repository whose modules declare no coverage has not been
    // checked, and PASS over it is the green-matrix-over-dead-links result.
    const repo = bundleWith({ "NFR-001-a.md": NFR("reliability") });
    const empty = mkdtempSync(join(tmpdir(), "quoin-vocab-none-"));
    writeFileSync(
      join(empty, "manifest.yaml"),
      "manifest_version: 1\nname: empty\n",
    );
    await Completeness.run(["--repo", repo, "--module", empty]);
    expect(warned.join("\n")).toMatch(/nothing was checked/);
    expect(logged.join("\n")).toMatch(/UNCHECKED/);
    expect(logged.join("\n")).not.toMatch(/PASS/);
    // A repository that has not adopted the vocabulary is not broken by
    // installing quoin; one that asked for strict completeness is told it
    // cannot be known.
    await expect(
      Completeness.run(["--repo", repo, "--module", empty, "--strict"]),
    ).rejects.toMatchObject({ oclif: { exit: 1 } });
  });
});

describe("agreement with the engine", () => {
  // Trace: FR-037-AC-13
  it("counts the same unowned values the bundle read reports", () => {
    // quoin and quire-rs answer the same question from the same declaration by
    // different routes. If they ever disagree, one of them is describing a
    // vocabulary the other is not — the failure TC-183 pins for FR-035.
    const repo = bundleWith({
      "NFR-001-a.md": NFR("reliability"),
      "NFR-002-b.md": NFR("security").replace("NFR-001", "NFR-002"),
    });
    const report = assessBundle({
      bundleRoot: join(repo, "spec"),
      moduleRoots: [moduleWith()],
    });
    expect(report.vocabularies).toEqual(["quality-characteristics"]);
    expect(report.rollups[0].owned).toBe(2);
    expect(report.findings.map((f) => f.value)).toEqual(["safety"]);
  });
});
