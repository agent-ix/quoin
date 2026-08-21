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

describe("TC-249 characteristics are read from prose, not from link targets", () => {
  // TC-249
  it("ignores a link's destination and keeps its text", () => {
    // Measured across the ecosystem: of 13 statements matching `stakeholder`,
    // 12 matched the directory name inside `](../stakeholder/StR-005-…)` and
    // one was the word in prose. Every characteristic reading the raw string
    // inherited that, the twenty predating agent-ix/quoin#128 included.
    const cited =
      "The loader SHALL reject a malformed manifest " +
      "([StR-004-AC-2](../stakeholder/StR-004-thin-boundary.md)).";
    expect(characteristicsOf(cited)).not.toContain("stakeholder-facing");

    // The text of a link is prose the author wrote, so it still counts.
    const worded = "A [stakeholder](./StR-001.md) SHALL confirm the outcome.";
    expect(characteristicsOf(worded)).toContain("stakeholder-facing");
  });

  // TC-249
  it("does not treat `path-safety` as the safety characteristic", () => {
    // 10 of 11 `safety` matches in this ecosystem were `path-safety`. The
    // 25010 characteristic is freedom from harm, not memory/path/type safety.
    expect(
      characteristicsOf("A path-safety violation SHALL exit 1."),
    ).not.toContain("safety");
    expect(
      characteristicsOf("The declared hazard SHALL have a mitigation."),
    ).toContain("safety");
  });
});

describe("TC-250 high-criticality is read from the obligation, not inferred", () => {
  // TC-250
  it("mints only when the declared value says so, and never from the statement", () => {
    const statement = "The parser SHALL reject an invalid token.";
    // CR-008 deleted a hardcoded `["P0"]` because a built-in threshold is a
    // rule nobody chose. The value is read, not judged.
    expect(characteristicsOf(statement, "P0")).toContain("high-criticality");
    expect(characteristicsOf(statement, "high")).toContain("high-criticality");
    expect(characteristicsOf(statement, "P3")).not.toContain(
      "high-criticality",
    );
    expect(characteristicsOf(statement, null)).not.toContain(
      "high-criticality",
    );
    expect(characteristicsOf(statement)).not.toContain("high-criticality");
  });
});

describe("TC-252 the spec-side triggers for concolic execution", () => {
  // TC-252
  it("reads a magic-value comparison, and does not read a function signature", () => {
    // The classic wall a fuzzer cannot climb: random bytes have essentially no
    // chance of satisfying a checksum and a solver has a certainty of it.
    expect(
      characteristicsOf(
        "The parser SHALL reject a payload whose CRC-32 does not match.",
      ),
    ).toContain("magic-value-comparison");
    expect(
      characteristicsOf("The loader SHALL verify the HMAC before decoding."),
    ).toContain("magic-value-comparison");

    // `signature` is not a bare alternative. "Function signature" is everywhere
    // in this corpus — the same trap as `path-safety` matching `safety` (#128).
    expect(
      characteristicsOf(
        "The function signature SHALL remain stable across a minor release.",
      ),
    ).not.toContain("magic-value-comparison");
  });

  // TC-252
  it("reads constant-time phrasing", () => {
    expect(
      characteristicsOf("Key comparison SHALL be constant-time."),
    ).toContain("secret-dependent-branch");
  });

  // TC-252
  it("separates reference equivalence from approval testing", () => {
    // Two questions, two methods. Approval testing compares against a RECORDED
    // output; equivalence checking proves two IMPLEMENTATIONS agree for all
    // inputs. Collapsing the phrasings would make one method answer the other.
    expect(
      characteristicsOf(
        "The rewritten parser SHALL be semantically equivalent to the previous implementation.",
      ),
    ).toContain("reference-equivalence");
    expect(
      characteristicsOf(
        "The output SHALL be byte-identical to the recorded snapshot.",
      ),
    ).not.toContain("reference-equivalence");
  });
});

describe("TC-253 the evidence-side facts", () => {
  const statement = "The cache SHALL evict the oldest entry.";

  // TC-253
  it("reports an exercised obligation nothing has measured", () => {
    expect(
      characteristicsOf(statement, null, {
        bound: true,
        faultDetectionScores: [],
      }),
    ).toContain("fault-detection-unmeasured");
  });

  // TC-253
  it("reports a seeded fault the tests missed, by the weakest bound symbol", () => {
    // The worst score, not the mean — FR-039's rule. Averaging lets a
    // well-tested symbol carry one whose mutants all survive, which is the case
    // the signal exists to find.
    const got = characteristicsOf(statement, null, {
      bound: true,
      faultDetectionScores: [1, 0.62],
    });
    expect(got).toContain("fault-detection-failed");
    expect(got).not.toContain("fault-detection-unmeasured");
  });

  // TC-253
  it("says nothing about an obligation nothing is bound to", () => {
    // That is `undischarged`, which the auditor reports separately. Conflating
    // them would recommend a solver for code nobody has tested yet.
    const got = characteristicsOf(statement, null, {
      bound: false,
      faultDetectionScores: [],
    });
    expect(got).not.toContain("fault-detection-unmeasured");
    expect(got).not.toContain("fault-detection-failed");

    // And an absent `evidence` means "not consulted", which is different again.
    expect(characteristicsOf(statement)).not.toContain(
      "fault-detection-unmeasured",
    );
  });

  // TC-253
  it("makes concolic-execution reachable from evidence alone", () => {
    const catalog = loadMethodCatalog();
    const advice = advise(catalog, {
      id: "FR-001-AC-1",
      statement,
      evidence: { bound: true, faultDetectionScores: [0.62] },
    });
    expect(advice.recommended.map((r) => r.method)).toContain(
      "concolic-execution",
    );
  });
});

describe("TC-266 structured parameters mint quantified-threshold (FR-031-AC-19)", () => {
  // The one STRUCTURED signal quire emits about an obligation was typed,
  // parsed and read nowhere (#166): the advisor guessed from prose while
  // `{"target": "< 4 min"}` sat unread in the record.
  const catalog: MethodCatalog = loadMethodCatalog([
    moduleWith(`name: fixture-benchmarking
verification_catalog:
  performance-benchmarking:
    name: Performance benchmarking
    class: Test
    definition: Measure against a stated budget.
    evidence_kind: Benchmark
    applicability:
      characteristics: [latency, throughput, quantified-threshold]
`),
  ]);

  // TC-266
  it("advises performance-benchmarking for the ticket's NFR-022-M-12 record", () => {
    const advice = advise(catalog, {
      id: "NFR-022-M-12",
      statement: "PR-tier CI wall clock (fmt/clippy/test/deny/unsafe-audit)",
      authoredMethod: "CI Measurement",
      parameters: { target: "< 4 min", threshold: "< 5 min" },
    });
    const methods = advice.recommended.map((r) => r.method);
    expect(methods).toContain("performance-benchmarking");
    expect(
      advice.recommended
        .find((r) => r.method === "performance-benchmarking")
        ?.reasons.map((x) => x.value),
    ).toContain("quantified-threshold");
  });

  // TC-266
  it("a parameters.target mints quantified-threshold whatever the statement says", () => {
    expect(
      characteristicsOf("Nothing numeric in this sentence.", null, undefined, {
        target: "under budget",
      }),
    ).toContain("quantified-threshold");
    expect(
      characteristicsOf("Nothing numeric in this sentence.", null, undefined, {
        threshold: "< 5 min",
      }),
    ).toContain("quantified-threshold");
  });

  // TC-266
  it("parameters carrying neither key mint nothing, and absent parameters change nothing", () => {
    expect(
      characteristicsOf("Nothing numeric in this sentence.", null, undefined, {
        unit: "minutes",
      }),
    ).not.toContain("quantified-threshold");
    expect(
      characteristicsOf("Nothing numeric in this sentence."),
    ).not.toContain("quantified-threshold");
  });
});

describe("TC-267 compound tokens do not leak fragment characteristics (FR-031-AC-20)", () => {
  // The regression corpus #167 asks for: hyphenated compounds are single
  // tokens. A characteristic keyword INSIDE one is a fragment and mints
  // nothing; a regex naming the WHOLE compound still matches it.

  // TC-267
  it("`unsafe-audit` does not mint memory-safety — the battle test's clearest false positive", () => {
    // A wall-clock budget for a CI lane, advised as a memory-safety check
    // because the lane's step list contains the substring "unsafe".
    expect(
      characteristicsOf(
        "PR-tier CI wall clock (fmt/clippy/test/deny/unsafe-audit)",
      ),
    ).not.toContain("memory-safety");
  });

  // TC-267
  it("bare `unsafe` still mints memory-safety", () => {
    expect(
      characteristicsOf("The crate SHALL contain no unsafe code."),
    ).toContain("memory-safety");
  });

  // TC-267
  it("a compound NAMED by the regex still matches: memory-safe, memory safety, use-after-free", () => {
    expect(characteristicsOf("The parser SHALL be memory-safe.")).toContain(
      "memory-safety",
    );
    expect(
      characteristicsOf("Memory safety SHALL be preserved across the FFI."),
    ).toContain("memory-safety");
    // Hyphens INSIDE the match are untouched by the guard.
    expect(
      characteristicsOf("The allocator SHALL exhibit no use-after-free."),
    ).toContain("memory-safety");
  });

  // TC-267
  it("thread-safety is named whole and mints concurrent; thread-pool is a fragment and does not", () => {
    expect(
      characteristicsOf("The registry SHALL guarantee thread-safety."),
    ).toContain("concurrent");
    expect(
      characteristicsOf("The thread-pool size SHALL default to 4."),
    ).not.toContain("concurrent");
  });

  // TC-267
  it("fail-safe still mints safety", () => {
    expect(
      characteristicsOf(
        "The valve SHALL enter a fail-safe state on loss of power.",
      ),
    ).toContain("safety");
  });

  // TC-267
  it("the NFR-022-M-12 record end to end: no memory-safety methods, performance-benchmarking instead", () => {
    const catalog: MethodCatalog = loadMethodCatalog([
      moduleWith(`name: fixture-167
verification_catalog:
  compile-time-check:
    name: Compile-time check
    class: Analysis
    definition: The compiler proves the property.
    evidence_kind: Static
    applicability:
      characteristics: [memory-safety]
  dynamic-analysis-sanitizer:
    name: Sanitizer
    class: Test
    definition: Run under a sanitizer.
    evidence_kind: Dynamic
    applicability:
      characteristics: [memory-safety]
  performance-benchmarking:
    name: Performance benchmarking
    class: Test
    definition: Measure against a stated budget.
    evidence_kind: Benchmark
    applicability:
      characteristics: [latency, throughput, quantified-threshold]
`),
    ]);
    const advice = advise(catalog, {
      id: "NFR-022-M-12",
      statement: "PR-tier CI wall clock (fmt/clippy/test/deny/unsafe-audit)",
      authoredMethod: "CI Measurement",
      parameters: { target: "< 4 min", threshold: "< 5 min" },
    });
    const methods = advice.recommended.map((r) => r.method);
    expect(methods).not.toContain("compile-time-check");
    expect(methods).not.toContain("dynamic-analysis-sanitizer");
    expect(methods).toContain("performance-benchmarking");
  });
});

describe("TC-268 architectural statements reach architecture-conformance (FR-031-AC-21)", () => {
  // The converse failure #167 names: `architecture-conformance` was keyed on
  // [layering, module-boundary] and was one of four catalog methods never
  // recommended across 1,107 obligations, because no regex minted those
  // characteristics from the corpus's actual architectural wording. Each
  // statement below is verbatim from that corpus (filament-ide-rs FR-015 /
  // NFR-009).

  // TC-268
  it("an architecture-absence claim mints module-boundary (FR-015-AC-4)", () => {
    expect(
      characteristicsOf(
        "Domain behavior remains absent from Core and is owned by top-level modules",
      ),
    ).toContain("module-boundary");
  });

  // TC-268
  it("a zero-dependencies declaration mints layering (FR-015-AC-6)", () => {
    expect(
      characteristicsOf(
        "Core declares zero dependencies on first-party top-level module crates",
      ),
    ).toContain("layering");
  });

  // TC-268
  it("a dependency-check rejection mints layering (FR-015-AC-3)", () => {
    expect(
      characteristicsOf(
        "Core dependency checks reject Tauri, Postgres, MCP, CLI, network, plugin, and packaging crates",
      ),
    ).toContain("layering");
  });

  // TC-268
  it("an acyclicity NFR mints layering (NFR-009)", () => {
    expect(
      characteristicsOf(
        "Core SHALL remain dependency-light and acyclic so every top-level module can depend on Core without Core depending back on any top-level module.",
      ),
    ).toContain("layering");
  });

  // TC-268
  it("`top-level claim` — the assurance-case term of art — mints nothing", () => {
    expect(
      characteristicsOf("No document declares itself a top-level claim."),
    ).not.toContain("module-boundary");
  });
});
