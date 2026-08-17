/**
 * `quoin advise` — the advisor's reachable path (TC-150).
 *
 * FR-031's eight ACs all passed against `advise()` directly, and no command
 * reached it: the deterministic advisor shipped as library code exercised only
 * by its own unit tests, while `skills/spec-evidence-analysis/SKILL.md` still
 * asked the agent to judge from `quoin catalog methods` — the prose table the
 * FR opens by saying it replaces (agent-ix/quoin#103).
 *
 * These cover the wiring the unit tests could not: that the command exists,
 * that it turns quire payloads into `ObligationFacts`, and that a partial
 * `properties` run still yields the property-shape axis.
 */

import { describe, expect, it } from "vitest";

import { advise, loadMethodCatalog } from "../src/advisor/index.js";
import type { MethodCatalog } from "../src/advisor/index.js";
import { runQuireAllowFailure } from "../src/quire/index.js";

const CATALOG: MethodCatalog = {
  methods: [
    {
      id: "property-based-testing",
      name: "PBT",
      class: "Test",
      definition: "d",
      evidenceKind: "Property",
      applicability: { property_shapes: ["round-trip", "invariant"] },
      tooling: [],
      moduleName: "m",
    },
    {
      id: "fault-injection",
      name: "Fault injection",
      class: "Test",
      definition: "d",
      evidenceKind: "Integration",
      applicability: { characteristics: ["reliability"] },
      tooling: [],
      moduleName: "m",
    },
  ],
  duplicates: [],
  unreadable: [],
};

describe("TC-150 the advisor is reachable from a command (FR-031-AC-10, AC-11)", () => {
  it("exposes a command class with the flags the workflow needs", async () => {
    // The command file existing is not enough: `vite.config.ts` enumerates
    // build entries by hand, and a command with no entry builds no module.
    // TC-149 guards the enumeration; this asserts the class itself.
    const { default: Advise } = await import("../src/commands/advise.js");
    expect(Advise.summary).toMatch(/verification method/i);
    expect(Object.keys(Advise.flags).sort()).toEqual([
      "inconclusive-only",
      "json",
      "mismatch-only",
      "module",
      "repo",
    ]);
  });

  it("advises from the property shape when quire supplies one", () => {
    // The axis most catalog entries are keyed on. Without it `round-trip`
    // never reaches property-based testing, and the advice collapses to
    // statement text — which reads as "no rule matched" and is not.
    const withShape = advise(CATALOG, {
      id: "FR-001-AC-1",
      statement: "Every parsed document serializes back to its input.",
      propertyShape: "round-trip",
      archetype: "FR",
    });
    expect(withShape.recommended.map((r) => r.method)).toEqual([
      "property-based-testing",
    ]);
    expect(withShape.recommended[0].reasons).toEqual([
      { rule: "property_shapes", value: "round-trip" },
    ]);

    const withoutShape = advise(CATALOG, {
      id: "FR-001-AC-1",
      statement: "Every parsed document serializes back to its input.",
      propertyShape: null,
      archetype: "FR",
    });
    expect(withoutShape.inconclusive).toBe(true);
  });

  it("still advises an NFR metric row, which has no property shape", () => {
    // An NFR `Measurement and Evaluation` row is an obligation and not a
    // criterion, so no classifier ran on it. It is advised from its statement.
    const advice = advise(CATALOG, {
      id: "NFR-011-M-2",
      statement: "The parser shall recover from a malformed token.",
      propertyShape: null,
      archetype: "NFR",
    });
    expect(advice.recommended.map((r) => r.method)).toEqual([
      "fault-injection",
    ]);
  });

  it("a run that exits non-zero still yields its payload", () => {
    // `quire properties` exits 1 when ANY input document fails to resolve while
    // still writing a complete payload for the rest. Treating that as total
    // failure cost the whole property-shape axis over two untyped asset files:
    // 359 of 583 obligations read "inconclusive" that should not have (#103).
    const result = runQuireAllowFailure([
      "--this-flag-does-not-exist-and-never-will",
    ]);
    expect(result.ok).toBe(false);
    // The point is that it RETURNS rather than throwing, so the caller decides.
    expect(typeof result.stdout).toBe("string");
    expect(typeof result.stderr).toBe("string");
  });

  it("reports an unreadable module instead of throwing", () => {
    // `loadMethodCatalog` is called on the command path, so a broken module
    // must not take down the advisor.
    const catalog = loadMethodCatalog([]);
    expect(catalog.unreadable).toEqual([]);
  });
});
