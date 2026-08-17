/**
 * FR-031 — the catalog-driven test-plan advisor (TC-129..TC-136).
 *
 * The proto-advisor was a skill-local prose table, so `Verification` columns
 * defaulted to `Test` by habit and nothing ever advised DAST for an attack
 * surface, monitors for a temporal property, or fault injection for a
 * reliability NFR. These tests are about whether those recommendations are now
 * reachable — not merely whether a function returns a list.
 */

import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import {
  advise,
  characteristicsOf,
  loadMethodCatalog,
  methodClasses,
  type MethodCatalog,
} from "../src/advisor/index.js";

/** The real ecosystem catalog, read from the source tree. */
function ecosystemCatalog(): MethodCatalog {
  return loadMethodCatalog([
    "/home/peter/dev/spec-artifacts-process/spec_artifacts_process",
  ]);
}

/** A throwaway module declaring exactly the catalog a test needs. */
function moduleWith(yaml: string): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-advisor-"));
  mkdirSync(root, { recursive: true });
  writeFileSync(join(root, "manifest.yaml"), yaml, "utf8");
  return root;
}

describe("TC-129 the merged catalog is read from module data", () => {
  it("loads every declared method with its rules intact", () => {
    const catalog = ecosystemCatalog();
    expect(catalog.methods.length).toBeGreaterThanOrEqual(30);
    expect(methodClasses(catalog)).toEqual([
      "Analysis",
      "Demonstration",
      "Inspection",
      "Test",
    ]);
    const pbt = catalog.methods.find((m) => m.id === "property-based-testing");
    expect(pbt?.evidenceKind).toBe("Property");
    expect(pbt?.applicability.property_shapes).toContain("round-trip");
  });

  it("merges first-wins and reports a collision rather than absorbing it", () => {
    const entry = (cls: string) =>
      `name: m-${cls}\nverification_catalog:\n  shared:\n    name: Shared\n    class: ${cls}\n    definition: d\n`;
    const a = moduleWith(entry("Test"));
    const b = moduleWith(entry("Analysis"));
    const catalog = loadMethodCatalog([a, b]);
    expect(catalog.methods).toHaveLength(1);
    expect(catalog.methods[0].class).toBe("Test");
    expect(catalog.duplicates).toEqual([
      { id: "shared", modules: ["m-Test", "m-Analysis"] },
    ]);
  });

  it("treats an undeclared catalog as empty rather than an error", () => {
    expect(loadMethodCatalog([moduleWith("name: bare\n")]).methods).toEqual([]);
  });
});

describe("TC-130 an attack surface reaches DAST and SAST", () => {
  it("recommends the security methods rather than defaulting to Test", () => {
    const advice = advise(ecosystemCatalog(), {
      id: "FR-001-AC-1",
      statement: "The endpoint rejects a request carrying an expired token.",
      objectTypes: ["attack_surface"],
    });
    const ids = advice.recommended.map((r) => r.method);
    expect(ids).toContain("dast");
    expect(ids).toContain("sast");
    expect(ids).toContain("negative-abuse-testing");
    expect(advice.inconclusive).toBe(false);
  });
});

describe("TC-131 temporal phrasing reaches monitors and model checking", () => {
  it("recommends the methods a single execution cannot discharge", () => {
    const advice = advise(ecosystemCatalog(), {
      id: "NFR-006-AC-1",
      statement:
        "The scheduler always eventually drains the queue while the worker is running.",
    });
    const ids = advice.recommended.map((r) => r.method);
    expect(ids).toContain("runtime-monitoring");
    expect(ids).toContain("model-checking");
  });
});

describe("TC-132 a reliability NFR reaches fault injection", () => {
  it("recommends inducing the failure the requirement claims to tolerate", () => {
    const advice = advise(ecosystemCatalog(), {
      id: "NFR-009-AC-1",
      statement:
        "The client tolerates a dropped connection and retries without losing a message.",
    });
    expect(advice.recommended.map((r) => r.method)).toContain(
      "fault-injection",
    );
  });
});

describe("TC-133 a property shape reaches the property methods", () => {
  it("routes a round-trip criterion to property-based and metamorphic testing", () => {
    const advice = advise(ecosystemCatalog(), {
      id: "FR-005-AC-1",
      statement:
        "Parsing then serializing a document yields the original bytes.",
      propertyShape: "round-trip",
    });
    const ids = advice.recommended.map((r) => r.method);
    expect(ids).toContain("property-based-testing");
    expect(ids).toContain("metamorphic-testing");
  });

  it("ranks a method two rules agree on above one a single rule suggested", () => {
    const advice = advise(ecosystemCatalog(), {
      id: "FR-005-AC-2",
      statement:
        "Every concurrent write eventually converges, invariant across orderings.",
      propertyShape: "concurrency",
    });
    // Deterministic ordering: more matching rules first, then id.
    const counts = advice.recommended.map((r) => r.reasons.length);
    expect(counts).toEqual([...counts].sort((a, b) => b - a));
  });
});

describe("TC-134 a mismatch with the authored cell is flagged, advisory only", () => {
  it("flags an authored method none of the recommendations cover", () => {
    const advice = advise(ecosystemCatalog(), {
      id: "NFR-009-AC-1",
      statement:
        "The client tolerates a dropped connection and retries without losing a message.",
      authoredMethod: "Inspection",
    });
    expect(advice.authored).toBe("Inspection");
    expect(advice.mismatch).toBe(true);
  });

  it("accepts an authored method matching a recommended method's class", () => {
    const advice = advise(ecosystemCatalog(), {
      id: "NFR-009-AC-1",
      statement:
        "The client tolerates a dropped connection and retries without losing a message.",
      authoredMethod: "Test (TC-001)",
    });
    // The annotation is stripped, and `Test` is the class of fault-injection.
    expect(advice.authored).toBe("Test");
    expect(advice.mismatch).toBe(false);
  });
});

describe("TC-135 silence is reported as silence, not as a recommendation", () => {
  it("is inconclusive when no rule matched, and flags no mismatch", () => {
    const advice = advise(ecosystemCatalog(), {
      id: "FR-001-AC-9",
      statement: "The widget count equals seven.",
      authoredMethod: "Test",
    });
    expect(advice.inconclusive).toBe(true);
    expect(advice.recommended).toEqual([]);
    // An advisor that recommends `Test` because it found nothing is the habit
    // this replaces; and reporting silence as a mismatch would bury the real ones.
    expect(advice.mismatch).toBe(false);
  });
});

describe("TC-136 an unobservable axis is skipped, not failed", () => {
  it("does not reject a method whose rule names an axis the advisor cannot see", () => {
    // The engine leaves the axis set open (quire-rs FR-054-CON-2), so a module
    // may declare rules this advisor has no facts for. That is a gap in what can
    // be observed, not grounds to drop the method.
    const root = moduleWith(
      "name: exotic\n" +
        "verification_catalog:\n" +
        "  exotic-method:\n" +
        "    name: Exotic\n" +
        "    class: Test\n" +
        "    definition: d\n" +
        "    applicability:\n" +
        "      phase_of_moon: [waxing]\n" +
        "      characteristics: [security]\n",
    );
    const advice = advise(loadMethodCatalog([root]), {
      id: "X",
      statement: "The endpoint requires an authenticated principal.",
    });
    expect(advice.recommended).toHaveLength(1);
    // Matched on the axis it *could* observe; the unknown one contributed
    // nothing rather than vetoing.
    expect(advice.recommended[0].reasons).toEqual([
      { rule: "characteristics", value: "security" },
    ]);
  });
});

describe("characteristic detection is lexical and deterministic", () => {
  it("reads facts about the text, never intent", () => {
    expect(characteristicsOf("responds within 5ms")).toContain("latency");
    expect(characteristicsOf("responds within 5ms")).toContain(
      "quantified-threshold",
    );
    expect(characteristicsOf("parses malformed input")).toContain("parser");
    expect(characteristicsOf("the widget is blue")).toEqual([]);
  });

  it("is stable across calls", () => {
    const s = "The parser rejects malformed untrusted input within 5ms.";
    expect(characteristicsOf(s)).toEqual(characteristicsOf(s));
  });
});
