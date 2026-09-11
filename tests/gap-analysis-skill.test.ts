/**
 * Gap analysis is planless by default (agent-ix/quoin#365).
 *
 * The skill used to target a plan bundle: target selection globbed `plan/*`,
 * asked which one, and stopped when there was none — so a repository whose work
 * predated planning could not be audited at all without first manufacturing a
 * retrospective plan. That reverses the assurance order the skill exists to
 * enforce, and turns a repository check into bookkeeping.
 *
 * What is asserted here is the part that silently drifts: that the default path
 * is repository-driven, that a plan may only ADD completion bookkeeping, and
 * that the emitted SpecReview says which of the two it did. These are prose
 * contracts because the skill is prose — an agent eval drives the behaviour,
 * costs minutes, and cannot run on every commit.
 *
 * **Every assertion runs against whitespace-flattened text.** Prettier owns the
 * line wrapping in these files, so a match anchored on a newline asserts the
 * wrap rather than the claim: re-flowing a paragraph, or changing `proseWrap`,
 * would fail a test whose subject did not change. Flattening keeps the coupling
 * on the sentence.
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const skillDir = join(repoRoot, "skills", "gap-analysis");

/** File contents with every whitespace run collapsed to a single space. */
const read = (...parts: string[]): string =>
  readFileSync(join(skillDir, ...parts), "utf8").replace(/\s+/g, " ");

const skill = read("SKILL.md");
const targetSelection = read("references", "step-1-target-selection.md");
const planCompletion = read("references", "step-2-plan-completion.md");
const matrix = read("references", "step-3-matrix-verification.md");
const reverseGap = read("references", "step-4-underspecified-code.md");
const semantic = read("references", "step-5-semantic-review.md");
const artifact = read("references", "step-6-specreview-artifact.md");

const all = [
  skill,
  targetSelection,
  planCompletion,
  matrix,
  reverseGap,
  semantic,
  artifact,
];

/** Raw frontmatter of the skill, which is not whitespace-flattened above. */
const skillFrontmatter =
  /^---\n([\s\S]*?)\n---\n/.exec(
    readFileSync(join(skillDir, "SKILL.md"), "utf8"),
  )?.[1] ?? "";

describe("gap-analysis: planless mode is the default", () => {
  it("declares the repository audit chain, not a plan, as the subject", () => {
    expect(skill).toContain("repository assurance audit");
    expect(skill).toContain(
      "spec requirements ↔ Test Matrix ↔ tagged tests ↔ source code",
    );
    expect(skill).toContain("**Planless (the default).**");
    // The frontmatter description is what a host shows when choosing a skill;
    // if it still promised a plan audit, callers would never reach the body.
    expect(skillFrontmatter).not.toBe("");
    expect(skillFrontmatter).toMatch(/planless by default/i);
  });

  it("runs all four repository checks with no plan", () => {
    // Matrix traceability, reverse code-to-spec gaps, stub / coverage
    // inflation, and the opt-in semantic review. The first three are
    // unconditional, so each is asserted with its "Always" label attached —
    // the claim is not that the step is mentioned but that nothing gates it.
    expect(skill).toContain(
      "**Always — [Matrix verification](references/step-3-matrix-verification.md).**",
    );
    expect(skill).toContain("quire coverage --scope <root> --json");
    expect(skill).toContain(
      "**Always — [Underspecified code](references/step-4-underspecified-code.md).**",
    );
    expect(skill).toContain("code/behavior with no owning requirement");
    expect(skill).toContain("coverage inflation");
    expect(skill).toContain(
      "**Only when the user opts in — [Semantic review](references/step-5-semantic-review.md).**",
    );
    expect(skill).toContain(
      "**Always — [SpecReview artifact](references/step-6-specreview-artifact.md).**",
    );
  });

  it("never discovers, prompts for, or auto-selects a plan", () => {
    expect(targetSelection).toContain("**Planless is the default.**");
    expect(targetSelection).toContain(
      "Do not glob `plan/`, do not present a plan menu, and do not ask which plan to audit",
    );
    expect(skill).toContain("**Never auto-select a plan.**");
    // The old instruction: "If there is no plan bundle, stop and tell the
    // user — gap-analysis audits a plan". It must not come back.
    for (const text of all) {
      expect(text).not.toContain(
        "stop and tell the user — gap-analysis audits a plan",
      );
    }
    expect(targetSelection).toContain(
      "If there is **no** plan bundle, that is the normal case",
    );
  });

  it("keeps a missing Test Matrix a high finding rather than an excuse", () => {
    expect(skill).toContain(
      "**A missing Test Matrix stays a `high` finding.**",
    );
    expect(targetSelection).toContain(
      "record a `high` finding that the matrix is missing",
    );
    expect(targetSelection).toContain("Do not create a matrix.");
  });
});

describe("gap-analysis: an explicit plan only adds bookkeeping", () => {
  it("gates the plan-completeness step on an explicit instruction", () => {
    expect(planCompletion).toContain(
      "# Plan Completeness (OPTIONAL — explicit plan only)",
    );
    expect(planCompletion).toContain(
      "**Skip this step entirely unless the caller named a plan**",
    );
    expect(planCompletion).toContain(
      "The presence of a `plan/` directory is not an instruction.",
    );
    expect(skill).toContain(
      "**Only with an explicit plan — [Plan completeness](references/step-2-plan-completion.md).**",
    );
    expect(skill).toContain("`gap-analysis --plan Plan-001`");
  });

  it("carries exactly the three plan-completeness checks", () => {
    expect(planCompletion).toContain("**All tasks done.**");
    expect(planCompletion).toContain("**Done-task dependency sanity.**");
    expect(planCompletion).toContain(
      "**Task status vs plan checkbox consistency.**",
    );
  });

  it("forbids a plan narrowing, driving, or suppressing the audit", () => {
    expect(skill).toContain("A plan is an *addition*, never a lens.");
    expect(planCompletion).toContain(
      "narrow the matrix, reverse-gap, stub, or semantic steps to the plan's task set",
    );
    expect(planCompletion).toContain(
      "suppress, downgrade, or omit a repository finding because it falls outside the plan",
    );
    // Each repository step restates its own scope, because a step is read in
    // isolation by a subagent that never saw SKILL.md.
    expect(matrix).toContain("**Scope: the whole repository, always.**");
    expect(reverseGap).toContain("**Scope: the whole source tree, always.**");
    expect(semantic).toContain("Opt-in works the same way in both modes");
  });
});

describe("gap-analysis: the SpecReview states what it assessed", () => {
  it("mandates the literal planless plan-completion line", () => {
    expect(skill).toContain("Plan completion: not assessed");
    expect(artifact).toContain(
      "| Planless | `Plan completion: not assessed` |",
    );
    expect(artifact).toContain(
      "| Plan-assisted | `Plan completion: assessed (<Plan-id>) — tasks done X / Y` |",
    );
    expect(artifact).toContain(
      "`Plan completion: not assessed` is the literal wording",
    );
    // A zero denominator reads as a completed plan, which is the exact
    // overclaim #365 exists to prevent.
    expect(artifact).toContain(
      "never write `Tasks done: 0 / 0` in a planless run",
    );
  });

  it("emits no plan relationship in a planless run", () => {
    expect(artifact).toContain(
      "Never emit a `reviews` edge to a plan in a planless run",
    );
  });

  it("keeps planless PASS from claiming planned work was completed", () => {
    expect(skill).toContain("**A planless PASS is a repository claim only.**");
    expect(artifact).toContain("### What a planless PASS means");
    expect(artifact).toContain(
      "asserts nothing about whether the work was planned, tracked, or completed against a plan",
    );
  });

  it("makes incomplete tasks a FAIL only in plan-assisted mode", () => {
    // Both verdict rules — SKILL.md's and the artifact step's — must carry the
    // qualifier. An unqualified copy is how a planless run starts failing on a
    // plan it was never given.
    for (const text of [skill, artifact]) {
      expect(text).toContain(
        "(plan-assisted only) any incomplete/blocked task",
      );
    }
  });
});

describe("gap-analysis: authors nothing", () => {
  it("forbids creating a plan, Task, requirement, or acceptance criterion", () => {
    expect(skill).toContain("**Never author.**");
    expect(skill).toContain(
      "does not create or edit a plan, a `Task`, a requirement, or an acceptance criterion",
    );
    expect(skill).toContain(
      "**Never manufacture a plan to satisfy the audit.**",
    );
    expect(artifact).toContain("This document is the run's **only** write.");
    expect(planCompletion).toContain(
      "do not create or edit a plan, a `Task`, a requirement, or an acceptance criterion",
    );
  });
});
