/**
 * NFR-005-AC-1 — the spec-matrix skill's Status vocabulary is coupled to the
 * module manifest (TC-271).
 *
 * PR #163 retired the warning marker from `skills/spec-matrix/SKILL.md` and
 * both asset templates BY HAND, and nothing coupled the skill's declared
 * vocabulary to the manifest that actually enforces it
 * (`spec-artifacts-process`: `column_patterns.Status` admits the syntax,
 * `traceability.status` classes the semantics). That is the CR-083 drift
 * class recurring undetectably: the next vocabulary change would leave the
 * skills silently teaching agents the old one (agent-ix/quoin#177).
 *
 * **The seam, stated:** the INSTALLED manifest at
 * `~/.ix/filament/modules/spec-artifacts-process/manifest.yaml` — the same
 * seam TC-118 relies on, materialized by `tests/global-setup.ts` reconciling
 * `default-modules.yaml`'s pins before any test runs. So this gate follows
 * the pins: bumping them re-tests the skills against the new vocabulary in
 * the same run, with no second restatement to keep current.
 *
 * **The comparison, stated:** what the skill teaches must equal the CLASSED
 * set exactly (either direction of divergence fails), and every taught
 * marker must be admitted by the column pattern. Admitted-but-unclassed
 * markers are deliberately NOT required to be taught: under the currently
 * pinned process v0.21.1 that residue is exactly `⚠️` — admitted by the
 * contract and classed by the traceability model as nothing, so every row
 * carrying it was exempt from the status-lie check by construction. Teaching
 * it is what #163 removed; requiring it here would encode the module's own
 * retired defect (fixed upstream in process v0.22.0) into quoin.
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { parse as parseYaml } from "yaml";
import { describe, expect, it } from "vitest";

import { filamentModulesDir } from "../src/catalog.js";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const skillDir = join(repoRoot, "skills", "spec-matrix");
const skillMd = readFileSync(join(skillDir, "SKILL.md"), "utf8");
const templateMd = readFileSync(
  join(skillDir, "assets", "test-matrix-template.md"),
  "utf8",
);
const exampleMd = readFileSync(
  join(skillDir, "assets", "test-matrix-example.md"),
  "utf8",
);
const manifest = parseYaml(
  readFileSync(
    join(filamentModulesDir(), "spec-artifacts-process", "manifest.yaml"),
    "utf8",
  ),
) as Record<string, unknown>;

/** Status markers in `text`, normalized to their base character (no FE0F). */
function markersIn(text: string): Set<string> {
  const found = text.match(/\p{Extended_Pictographic}/gu) ?? [];
  return new Set(found.map((m) => m.replace(/\uFE0F/g, "")));
}

const sorted = (s: Set<string>) => [...s].sort();

/** Every `column_patterns.Status` regex anywhere in the manifest. */
function admittedSet(node: unknown, acc = new Set<string>()): Set<string> {
  if (node === null || typeof node !== "object") return acc;
  for (const [key, value] of Object.entries(node)) {
    if (key === "column_patterns" && typeof value === "object" && value) {
      const pattern = (value as Record<string, unknown>).Status;
      if (typeof pattern === "string") {
        for (const m of markersIn(pattern)) acc.add(m);
      }
    } else {
      admittedSet(value, acc);
    }
  }
  return acc;
}

/** The union of `traceability.status`'s class arrays — the semantic set. */
function classedSet(): Set<string> {
  const traceability = manifest.traceability as
    Record<string, unknown> | undefined;
  const status = traceability?.status as Record<string, unknown> | undefined;
  expect(status, "the manifest declares no traceability.status").toBeTruthy();
  const acc = new Set<string>();
  for (const value of Object.values(status ?? {})) {
    if (Array.isArray(value)) {
      for (const marker of value)
        acc.add(String(marker).replace(/\uFE0F/g, ""));
    }
  }
  return acc;
}

/** The `Status` column cells of every markdown table in `text`. */
function statusCells(text: string): string[] {
  const cells: string[] = [];
  let statusIdx = -1;
  for (const line of text.split("\n")) {
    if (!line.trimStart().startsWith("|")) {
      statusIdx = -1;
      continue;
    }
    const row = line.split("|").map((c) => c.trim());
    if (statusIdx === -1) {
      statusIdx = row.findIndex((c) => c === "Status");
      continue; // header (or a table without a Status column: stays -1)
    }
    if (/^[-\s:|]+$/.test(line)) continue; // separator
    if (statusIdx < row.length) cells.push(row[statusIdx]);
  }
  return cells;
}

/** The vocabulary the SKILL.md `### Status` section declares. */
function skillDeclared(): Set<string> {
  const section = /###\s+`Status`\n+([^\n]+)/.exec(skillMd);
  expect(section, "SKILL.md declares no `### Status` section").toBeTruthy();
  return markersIn(section?.[1] ?? "");
}

/** The vocabulary the SKILL.md `## Markers` list declares. */
function skillMarkersList(): Set<string> {
  const section = /## Markers\n([\s\S]*?)(?:\n## |$)/.exec(skillMd);
  expect(section, "SKILL.md declares no `## Markers` list").toBeTruthy();
  return markersIn(section?.[1] ?? "");
}

describe("TC-271 the spec-matrix vocabulary cannot drift from the module manifest", () => {
  // TC-271
  it("SKILL.md's declared Status vocabulary equals the manifest's classed set", () => {
    // Exact equality: a marker taught but classed by nothing is exempt from
    // the status-lie check by construction (the ⚠️ defect); a marker classed
    // but not taught is vocabulary the skill silently withholds.
    expect(sorted(skillDeclared())).toEqual(sorted(classedSet()));
  });

  // TC-271
  it("every taught marker is admitted by the manifest's Status column pattern", () => {
    const admitted = admittedSet(manifest);
    expect(admitted.size, "no column_patterns.Status found").toBeGreaterThan(0);
    for (const marker of skillDeclared()) {
      expect(admitted, `\`${marker}\` is taught but not admitted`).toContain(
        marker,
      );
    }
  });

  // TC-271
  it("the `## Markers` list restates exactly the declared vocabulary", () => {
    expect(sorted(skillMarkersList())).toEqual(sorted(skillDeclared()));
  });

  // TC-271
  it("the template's Status cells use exactly the declared vocabulary", () => {
    const used = markersIn(statusCells(templateMd).join(" "));
    expect(sorted(used)).toEqual(sorted(skillDeclared()));
  });

  // TC-271
  it("the example's Status cells stay within the declared vocabulary", () => {
    // Subset, not equality: an example need not exhibit a retired-row (`⛔`)
    // to be a good example, but it must not exhibit a marker the vocabulary
    // does not admit.
    const cells = statusCells(exampleMd);
    const used = markersIn(cells.join(" "));
    expect(used.size).toBeGreaterThan(0);
    for (const marker of used) {
      expect(
        skillDeclared(),
        `example uses undeclared \`${marker}\``,
      ).toContain(marker);
    }
    // #177's second finding: `🚧 Partial` reintroduced the retired concept as
    // a note word — the marker plus the word are two forms for one meaning.
    for (const cell of cells) {
      expect(cell, "the retired concept returned as a note word").not.toMatch(
        /partial/i,
      );
    }
  });
});
