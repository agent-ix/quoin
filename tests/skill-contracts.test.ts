/**
 * Behavioural contract tests for the shipped skills (quoin#202).
 *
 * 11 of the 18 skills had zero automated coverage of any kind, and the 7
 * "covered" ones relied on vocabulary-drift coupling or dispatch-only agent
 * evals — **nothing asserted what a skill produces**.
 *
 * These are deterministic contract tests over the skill definitions and the
 * module vocabulary they emit into. They do not drive an agent: an eval does
 * that, costs minutes, and cannot run on every commit. What is asserted here is
 * the part that silently drifts — the pairing between a skill and the declared
 * `analysis` value it writes.
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { parse as parseYaml } from "yaml";

const SKILLS_DIR = join(__dirname, "..", "skills");
const MODULE_SCHEMA = join(
  process.env.HOME ?? "",
  "dev/spec-artifacts-process/spec_artifacts_process/schemas/spec-review-frontmatter.schema.json",
);

function skillDirs(): string[] {
  return readdirSync(SKILLS_DIR).filter((d) =>
    statSync(join(SKILLS_DIR, d)).isDirectory(),
  );
}

function skillText(name: string): string {
  return readFileSync(join(SKILLS_DIR, name, "SKILL.md"), "utf8");
}

function frontmatter(text: string): string {
  return /^---\n([\s\S]*?)\n---\n/.exec(text)?.[1] ?? "";
}

function parsedFrontmatter(text: string): Record<string, unknown> {
  const parsed: unknown = parseYaml(frontmatter(text));
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("skill frontmatter must be a YAML mapping");
  }
  return parsed as Record<string, unknown>;
}

/** Every `analysis: <value>` the skill mentions. */
function declaredAnalyses(text: string): string[] {
  return [
    ...new Set(
      [...text.matchAll(/analysis:\s*`?([a-z-]+)`?/g)].map((m) => m[1]),
    ),
  ];
}

/** The module's declared vocabulary, or null when the module is not checked out. */
function analysisEnum(): string[] | null {
  try {
    return JSON.parse(readFileSync(MODULE_SCHEMA, "utf8")).properties.analysis
      .enum;
  } catch {
    return null;
  }
}

describe("skill definitions", () => {
  it("every skill ships valid frontmatter whose name matches its directory", () => {
    // Codex rejects the entire skill when its frontmatter is invalid YAML. A
    // regex-only check missed spec-fuzz's `evidence_kind: Fuzz` plain-scalar
    // continuation because it never exercised the parser used by consumers.
    const dirs = skillDirs();
    expect(dirs.length).toBeGreaterThanOrEqual(18);
    for (const dir of dirs) {
      const fm = parsedFrontmatter(skillText(dir));
      expect(fm.name).toBe(dir);
      expect(typeof fm.description).toBe("string");
      expect((fm.description as string).trim()).not.toBe("");
    }
  });

  it("no skill declares an analysis value outside the module vocabulary", () => {
    // A skill emitting `analysis: whatever` writes a document the contract
    // rejects — and the failure surfaces at authoring time, in a different
    // session, as a validation error nobody connects to the skill.
    const declared = analysisEnum();
    if (!declared) return; // module not checked out; the pairing test below still runs
    for (const dir of skillDirs()) {
      for (const value of declaredAnalyses(skillText(dir))) {
        expect(declared).toContain(value);
      }
    }
  });

  it("every skill that emits a SpecReview says which analysis value", () => {
    // The gap #202 is about. Five skills named by the module's own vocabulary
    // never stated they emitted under it, so a skill and its declared output
    // were unlinked and drift between them was invisible.
    const emitters = skillDirs().filter((d) => /SpecReview/.test(skillText(d)));
    expect(emitters.length).toBeGreaterThan(0);
    for (const dir of emitters) {
      expect(declaredAnalyses(skillText(dir)).length).toBeGreaterThan(0);
    }
  });

  it("every analysis value a skill is named for has an owning skill", () => {
    // The other direction. A declared value nobody emits is a vocabulary entry
    // that can never be produced — the `ac:unclassifiable` shape, one layer up.
    const declared = analysisEnum();
    if (!declared) return;
    const owned = new Set(
      skillDirs().flatMap((d) => declaredAnalyses(skillText(d))),
    );
    // `base` and `code-review` are deliberately excluded: `base` is the
    // generic default and `code-review` is emitted by a skill outside this
    // package. Both are named here rather than silently skipped.
    const expectOwned = declared.filter(
      (v) => v !== "base" && v !== "code-review",
    );
    for (const value of expectOwned) expect(owned).toContain(value);
  });
});

describe("vendored workflow registries", () => {
  const STALE = [
    "spec-blueprint",
    "spec-us-to-fr",
    "spec-write-fr",
    "spec-write-it",
    "spec-write-nfr",
    "spec-write-str",
    "spec-write-us",
  ];

  it("list no skill this package does not ship", () => {
    // The vendored `dist/` registries still listed seven skills whose source
    // repository is archived. A registry naming a skill that cannot be invoked
    // is a menu with dishes the kitchen does not make.
    const shipped = new Set(skillDirs());
    for (const dir of skillDirs()) {
      const registry = join(
        SKILLS_DIR,
        dir,
        "workflow-assets",
        "dist",
        "index.js",
      );
      let text: string;
      try {
        text = readFileSync(registry, "utf8");
      } catch {
        continue;
      }
      for (const stale of STALE) {
        expect(text).not.toContain(`id: "${stale}"`);
      }
      for (const [, id] of text.matchAll(/id:\s*"(spec-[a-z-]+)"/g)) {
        expect(shipped.has(id) || id === "spec-fuzz").toBe(true);
      }
    }
  });
});
