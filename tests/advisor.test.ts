/**
 * FR-031 — the catalog-driven test-plan advisor (TC-129..TC-136, TC-133).
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

/**
 * A fixture catalog declaring the rules these tests exercise.
 *
 * Deliberately NOT the ecosystem's own catalog read off disk: an earlier
 * version pointed at an absolute path under `~/dev`, which passed on the
 * author's machine and produced an empty catalog — and six silent failures —
 * anywhere else. These tests are about the **advisor's behaviour** given rules;
 * the ecosystem's catalog *content* is tested where it lives
 * (`spec-artifacts-process` TC-048..TC-055).
 */
const FIXTURE_CATALOG = `name: fixture-methods
verification_catalog:
  property-based-testing:
    name: Property-based testing
    class: Test
    definition: Execute a property over generated inputs.
    evidence_kind: Property
    applicability:
      property_shapes: [universal, invariant, round-trip, idempotence, ordering]
      characteristics: [universally-quantified]
  metamorphic-testing:
    name: Metamorphic testing
    class: Test
    definition: Assert a relation between two related executions.
    evidence_kind: Property
    applicability:
      property_shapes: [round-trip, idempotence, ordering]
  deterministic-simulation:
    name: Deterministic simulation
    class: Test
    definition: Run under a seeded scheduler.
    evidence_kind: Integration
    applicability:
      property_shapes: [concurrency]
      characteristics: [concurrent]
  runtime-monitoring:
    name: Runtime monitoring
    class: Test
    definition: Check a temporal property against a running system.
    evidence_kind: Integration
    applicability:
      characteristics: [temporal, liveness, invariance]
  model-checking:
    name: Temporal model checking
    class: Analysis
    definition: Exhaustively check a temporal property against a model.
    evidence_kind: Static
    applicability:
      characteristics: [temporal, liveness, safety]
  fault-injection:
    name: Fault injection
    class: Test
    definition: Induce a failure the system claims to tolerate.
    evidence_kind: Integration
    applicability:
      characteristics: [reliability, fault-tolerance]
  dast:
    name: Dynamic application security testing
    class: Test
    definition: Probe the running system's exposed surface.
    evidence_kind: Integration
    applicability:
      object_types: [attack_surface]
      characteristics: [network-exposed, security]
  sast:
    name: Static application security testing
    class: Analysis
    definition: Match declared rules against source.
    evidence_kind: Static
    applicability:
      object_types: [attack_surface]
      characteristics: [security, injection-risk]
  negative-abuse-testing:
    name: Negative / abuse-case testing
    class: Test
    definition: Exercise the paths an adversary takes.
    evidence_kind: Integration
    applicability:
      object_types: [attack_surface]
      characteristics: [security, input-validation]
  inspection:
    name: Inspection
    class: Inspection
    definition: A person reads the artifact against the requirement.
    evidence_kind: Manual
    applicability:
      characteristics: [no-executable-oracle]
  demonstration:
    name: Demonstration
    class: Demonstration
    definition: The system is operated in front of a witness.
    evidence_kind: Manual
    applicability:
      characteristics: [stakeholder-acceptance, user-visible]
`;

/** A throwaway module declaring exactly the catalog a test needs. */
function moduleWith(yaml: string): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-advisor-"));
  mkdirSync(root, { recursive: true });
  writeFileSync(join(root, "manifest.yaml"), yaml, "utf8");
  return root;
}

/** The fixture catalog, loaded through the real loader. */
function fixtureCatalog(): MethodCatalog {
  return loadMethodCatalog([moduleWith(FIXTURE_CATALOG)]);
}

describe("TC-129 the merged catalog is read from module data", () => {
  // TC-129
  it("loads every declared method with its rules intact", () => {
    const catalog = fixtureCatalog();
    expect(catalog.methods.length).toBe(11);
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
  // TC-130
  it("recommends the security methods rather than defaulting to Test", () => {
    const advice = advise(fixtureCatalog(), {
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
  // TC-131
  it("recommends the methods a single execution cannot discharge", () => {
    const advice = advise(fixtureCatalog(), {
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
  // TC-132
  it("recommends inducing the failure the requirement claims to tolerate", () => {
    const advice = advise(fixtureCatalog(), {
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
  // TC-133
  it("routes a round-trip criterion to property-based and metamorphic testing", () => {
    const advice = advise(fixtureCatalog(), {
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
    const advice = advise(fixtureCatalog(), {
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
  // TC-134
  it("flags an authored method none of the recommendations cover", () => {
    const advice = advise(fixtureCatalog(), {
      id: "NFR-009-AC-1",
      statement:
        "The client tolerates a dropped connection and retries without losing a message.",
      authoredMethod: "Inspection",
    });
    expect(advice.authored).toBe("Inspection");
    expect(advice.mismatch).toBe(true);
  });

  it("accepts an authored method matching a recommended method's class", () => {
    const advice = advise(fixtureCatalog(), {
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
  // TC-135
  it("is inconclusive when no rule matched, and flags no mismatch", () => {
    const advice = advise(fixtureCatalog(), {
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
  // TC-136
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

describe("TC-133 an unreadable module manifest is reported, not thrown (FR-031-AC-9)", () => {
  // `loadMethodCatalog` read `manifest.yaml` with no guard, so a manifest whose
  // YAML is malformed took down `quoin catalog methods` — the command an
  // operator runs *to diagnose* module problems (agent-ix/quoin#106).
  //
  // A root with no `manifest.yaml` at all never reaches the read:
  // `locateModuleRoot` already returns `undefined` for it, which is why the
  // gap was only reachable through a manifest that exists and does not parse.
  // TC-133
  it("skips a root with no manifest without reporting it as unreadable", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-nomanifest-"));
    const catalog = loadMethodCatalog([root]);
    expect(catalog.methods).toEqual([]);
    expect(catalog.unreadable).toEqual([]);
  });

  it("skips malformed YAML and still merges the modules that parsed", () => {
    const bad = mkdtempSync(join(tmpdir(), "quoin-badyaml-"));
    writeFileSync(join(bad, "manifest.yaml"), "name: m\n  bad: [indent\n");
    const good = mkdtempSync(join(tmpdir(), "quoin-goodyaml-"));
    writeFileSync(
      join(good, "manifest.yaml"),
      "name: good\nverification_catalog:\n  unit-testing:\n" +
        "    name: Unit testing\n    class: Test\n    definition: d\n",
    );

    const catalog = loadMethodCatalog([bad, good]);
    expect(catalog.methods.map((m) => m.id)).toEqual(["unit-testing"]);
    expect(catalog.unreadable.map((u) => u.moduleRoot)).toEqual([bad]);
    expect(catalog.unreadable[0].reason).toBeTruthy();
  });
});
