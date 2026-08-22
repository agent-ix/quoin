/** FR-043 — assurance-aware specification authoring (TC-277..TC-278). */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { SCENARIOS } from "../evals/scenarios/index.mjs";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const skill = readFileSync(join(repoRoot, "skills/specify/SKILL.md"), "utf8");
const reference = readFileSync(
  join(repoRoot, "skills/specify/references/assurance-artifacts.md"),
  "utf8",
);
const normalizedSkill = skill.replace(/\s+/g, " ");

describe("assurance-aware specify contract", () => {
  // Trace: FR-043-AC-1 / TC-277
  it("authors requested installed assurance types through the live quoin contract", () => {
    for (const type of [
      "AssuranceProfile",
      "ArchitectureDescription",
      "MeasurementPlan",
    ]) {
      expect(skill).toContain(`\`${type}\``);
    }
    expect(skill).toContain("quoin write <repo_dir> --types");
    expect(reference).toContain(
      "exact requirements, decisions, or bounded system",
    );
    expect(reference).toContain(
      "Use only relationship shapes admitted by the fetched schema",
    );
  });

  // Trace: FR-043-AC-2 / TC-278
  it("keeps assurance artifacts opt-in and preserves the confirmation boundary", () => {
    expect(normalizedSkill).toMatch(/Assurance artifacts are opt-in/i);
    expect(normalizedSkill).toContain(
      "does not receive an unsolicited profile",
    );
    expect(normalizedSkill).toContain("Stop after the requested artifacts");
    expect(reference).toContain("do not launch reviews");
  });

  // Trace: FR-043-AC-2 / TC-278
  it("keeps the negative-control fixture independent of org discovery", () => {
    const scenario = SCENARIOS.find(({ id }) => id === "TC-EV-059");
    expect(scenario?.env?.({})).toEqual({ QUOIN_ORG: "agent-ix" });
  });
});
