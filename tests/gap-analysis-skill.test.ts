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
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const skillDir = join(repoRoot, "skills", "gap-analysis");

const read = (...parts: string[]): string =>
  readFileSync(join(skillDir, ...parts), "utf8");

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

describe("gap-analysis: planless mode is the default", () => {
  it("declares the repository audit chain, not a plan, as the subject", () => {
    expect(skill).toContain("repository assurance audit");
    expect(skill).toMatch(
      /spec requirements\s+↔\s+Test Matrix\s+↔\s+tagged tests\s+↔\s+source code/,
    );
    expect(skill).toContain("**Planless (the default).**");
    // The frontmatter description is what a host shows when choosing a skill;
    // if it still promised a plan audit, callers would never reach the body.
    const description = /^---\n([\s\S]*?)\n---\n/.exec(skill)?.[1] ?? "";
    expect(description).toMatch(/planless by default/i);
  });

  it("requires the four repository checks without a plan", () => {
    // Matrix traceability, reverse code-to-spec gaps, stub / coverage
    // inflation, and the opt-in semantic review.
    expect(skill).toContain("quire coverage --scope <root> --json");
    expect(skill).toContain("no owning");
    expect(skill).toContain("coverage inflation");
    expect(skill).toContain("Semantic review");
    expect(skill).toMatch(
      /Steps 0–2 and 5 always run\. Step 3 is gated on user opt-in\./,
    );
  });

  it("never discovers, prompts for, or auto-selects a plan", () => {
    expect(targetSelection).toContain("**Planless is the default.**");
    expect(targetSelection).toMatch(
      /Do not glob `plan\/`, do not present a plan menu, and do not ask\s+which plan to audit/,
    );
    expect(skill).toContain("**Never auto-select a plan.**");
    // The old instruction: "If there is no plan bundle, stop and tell the
    // user — gap-analysis audits a plan". It must not come back.
    for (const text of all) {
      expect(text).not.toMatch(
        /stop and tell the user — gap-analysis audits a plan/,
      );
    }
    expect(targetSelection).toMatch(
      /If there is \*\*no\*\* plan bundle, that is the normal case/,
    );
  });

  it("keeps a missing Test Matrix a high finding rather than an excuse", () => {
    expect(skill).toContain(
      "**A missing Test Matrix stays a `high` finding.**",
    );
    expect(targetSelection).toMatch(
      /record a `high` finding\s+that the matrix is missing/,
    );
    expect(targetSelection).toContain("Do not create a matrix.");
  });
});

describe("gap-analysis: an explicit plan only adds bookkeeping", () => {
  it("gates the plan-completeness step on an explicit instruction", () => {
    expect(planCompletion).toContain(
      "# Plan Completeness (OPTIONAL — explicit plan only)",
    );
    expect(planCompletion).toMatch(
      /\*\*Skip this step entirely unless the caller named a plan\*\*/,
    );
    expect(planCompletion).toContain(
      "The\npresence of a `plan/` directory is not an instruction.",
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
    expect(planCompletion).toMatch(
      /narrow the matrix, reverse-gap, stub, or semantic steps to the plan's task set/,
    );
    expect(planCompletion).toMatch(
      /suppress, downgrade, or omit a repository finding because it falls outside the plan/,
    );
    // Each repository step restates its own scope, because a step is read in
    // isolation by a subagent that never saw SKILL.md.
    expect(matrix).toContain("**Scope: the whole repository, always.**");
    expect(reverseGap).toContain("**Scope: the whole source tree, always.**");
    expect(semantic).toMatch(/Opt-in works the same way in both modes/);
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
    expect(artifact).toMatch(
      /`Plan completion: not assessed` is the literal wording/,
    );
    expect(artifact).toMatch(
      /never write `Tasks done: 0 \/ 0` in a planless run/,
    );
  });

  it("emits no plan relationship in a planless run", () => {
    expect(artifact).toMatch(
      /Never emit a `reviews` edge to a plan in a\s+planless run/,
    );
  });

  it("keeps planless PASS from claiming planned work was completed", () => {
    expect(skill).toContain("**A planless PASS is a repository claim only.**");
    expect(artifact).toContain("### What a planless PASS means");
    expect(artifact).toMatch(
      /asserts nothing about whether the work was planned, tracked, or\s+completed against a plan/,
    );
  });

  it("makes incomplete tasks a FAIL only in plan-assisted mode", () => {
    for (const text of [skill, artifact]) {
      expect(text).toMatch(
        /\(plan-assisted only\)\s+any\s+incomplete\/blocked task|\(plan-assisted only\) any\s+incomplete\/blocked task/,
      );
    }
  });
});

describe("gap-analysis: authors nothing", () => {
  it("forbids creating a plan, Task, requirement, or acceptance criterion", () => {
    expect(skill).toContain("**Never author.**");
    expect(skill).toMatch(
      /does not create or edit a plan, a `Task`, a\s+requirement, or an acceptance criterion/,
    );
    expect(skill).toContain(
      "**Never manufacture a plan to satisfy the audit.**",
    );
    expect(artifact).toMatch(/This document is the run's \*\*only\*\* write\./);
    expect(planCompletion).toMatch(
      /do not create or edit a plan, a `Task`, a requirement, or an acceptance\s+criterion/,
    );
  });
});
