/**
 * FR-036 — architecture conformance as a declared verification method.
 *
 * The boundary checks this method exists to make possible live in
 * `arch-boundaries.test.ts` (TC-202, TC-203); the audit-script adapter that
 * reads a conformance run is `quoin-evidence`'s
 * (`src/adapters/audit_script.rs`, quoin#458).
 */

import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  advise,
  characteristicsOf,
  loadMethodCatalog,
  type MethodCatalog,
} from "../src/advisor/index.js";

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
  // Trace: FR-036-AC-6, FR-036-CON-3
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
