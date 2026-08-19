/**
 * FR-036 — architecture conformance as a declared verification method
 * (TC-197..TC-201 the adapter, TC-204/TC-205 the advisor).
 *
 * The boundary checks this method exists to make possible live in
 * `arch-boundaries.test.ts` (TC-202, TC-203).
 */

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  parseAuditScript,
  selectFindingAdapter,
} from "../src/evidence/index.js";
import {
  advise,
  characteristicsOf,
  loadMethodCatalog,
  type MethodCatalog,
} from "../src/advisor/index.js";

const here = dirname(fileURLToPath(import.meta.url));
/** Real output: every `quire-rs/scripts/audits/*.sh`, captured 2026-08-18. */
const realClean = readFileSync(
  join(here, "fixtures", "evidence", "audit-static-real.txt"),
  "utf8",
);

// The FAIL line is verbatim from `quire-rs/scripts/audits/check_no_schemars.sh`,
// not invented for this test.
const FAILING = `check_dep_pins: OK
check_no_schemars: FAIL — 'schemars' present in Cargo.lock (FR-003-AC-4).
check_no_shellout: OK`;

describe("the audit-script adapter", () => {
  // Trace: FR-036-AC-1
  it("reads a real all-passing run as rules that ran and found nothing", () => {
    // Seven scripts reporting OK is a clean architecture-conformance run — the
    // case FR-034 exists to preserve. Zero findings with rules evaluated is a
    // RESULT; it must not read like a scan that never happened.
    const result = parseAuditScript(realClean);
    expect(result.findings).toEqual([]);
    expect(result.rulesEvaluated).toBe(7);
    expect(result.tool).toBe("audit-script");
  });

  // Trace: FR-036-AC-2
  it("turns a FAIL line into a finding carrying the criterion it audits", () => {
    // The join FR-034 called the reason this is worth building — and here it is
    // LITERAL. The script prints the criterion in its own failure line, so the
    // obligation is stated by the tool rather than mapped after the fact.
    const { findings, rulesEvaluated } = parseAuditScript(FAILING);
    expect(rulesEvaluated).toBe(3);
    expect(findings).toHaveLength(1);
    expect(findings[0].ruleId).toBe("check_no_schemars");
    expect(findings[0].severity).toBe("violation");
    expect(findings[0].traceIds).toEqual(["FR-003-AC-4"]);
    expect(findings[0].message).toMatch(/schemars/);
  });

  // Trace: FR-036-AC-3
  it("counts OK lines as rules evaluated, so a clean run is not vacuous", () => {
    // If only FAIL lines counted, a fully-passing suite would report zero rules
    // and read as vacuous — the check that found nothing because it looked for
    // nothing. That is precisely backwards for the healthiest possible result.
    expect(parseAuditScript(realClean).rulesEvaluated).toBeGreaterThan(0);
    expect(parseAuditScript("only_one: OK").rulesEvaluated).toBe(1);
  });

  // Trace: FR-036-AC-4
  it("rejects output carrying no audit line at all", () => {
    // No recognised line means no audit ran. Recording it would manufacture
    // conformance evidence from a file that proves nothing.
    expect(() => parseAuditScript("")).toThrow(/no .* line found/);
    expect(() =>
      parseAuditScript("Compiling quire-rs v0.33.0\nFinished"),
    ).toThrow(/audit-script output/);
  });

  // Trace: FR-036-AC-5
  it("is selected by --tool for the tools that emit this shape", () => {
    expect(selectFindingAdapter({ tool: "make ci" })?.name).toBe(
      "audit-script",
    );
    expect(selectFindingAdapter({ tool: "import-linter 2.0" })?.name).toBe(
      "audit-script",
    );
    expect(selectFindingAdapter({ adapter: "audit-script" })?.name).toBe(
      "audit-script",
    );
  });
});

/**
 * The catalog's real `architecture-conformance` entry, verbatim from
 * `spec-artifacts-process`. Copied rather than loaded from the installed module
 * so the test states the rule it depends on: if the module changes its
 * applicability keys, this fixture and the advisor disagree visibly instead of
 * the method quietly ceasing to be recommended.
 */
const ARCH_CATALOG = `manifest_version: 1
name: arch-fixture
version: 0.0.0
verification_catalog:
  architecture-conformance:
    name: Architecture conformance
    class: Analysis
    definition: >-
      Check the implemented dependency structure against the declared one, so a
      layering rule is enforced rather than documented.
    evidence_kind: Static
    applicability:
      characteristics: [layering, module-boundary]
    tooling: [import-linter, cargo-deny bans, custom audit]
`;

function archCatalog(): MethodCatalog {
  const root = mkdtempSync(join(tmpdir(), "quoin-arch-"));
  writeFileSync(join(root, "manifest.yaml"), ARCH_CATALOG, "utf8");
  return loadMethodCatalog([root]);
}

describe("advising architecture conformance", () => {
  // Trace: FR-036-AC-6
  it("mints both characteristics the catalog's rule is keyed on", () => {
    // `module-boundary` was declared by the catalog and minted by nothing, so
    // half of this method's applicability rule could never match. A census of
    // the shipped catalog found 41 of its 60 declared characteristics in that
    // state and 7 methods unreachable outright (agent-ix/quoin#128); this pins
    // the one FR-036 depends on.
    expect(
      characteristicsOf("The engine SHALL NOT depend on the CLI crate."),
    ).toContain("layering");
    expect(
      characteristicsOf(
        "Internals of the store SHALL NOT cross the module boundary.",
      ),
    ).toContain("module-boundary");
  });

  // Trace: FR-036-AC-6
  it("recommends the method for an architectural statement, end to end", () => {
    // Through the real loader and the real advisor — not `characteristicsOf`
    // alone. A characteristic nothing consumes recommends nothing.
    const advice = advise(archCatalog(), {
      id: "FR-036-AC-7",
      statement:
        "No library module SHALL depend on src/commands/; commands are leaves at the module boundary.",
    });
    expect(advice.inconclusive).toBe(false);
    expect(advice.recommended[0].method).toBe("architecture-conformance");
    expect(advice.recommended[0].evidenceKind).toBe("Static");
    // Both halves of the rule matched, which is the point of the fix.
    expect(advice.recommended[0].reasons.map((r) => r.value).sort()).toEqual([
      "layering",
      "module-boundary",
    ]);
  });
});
