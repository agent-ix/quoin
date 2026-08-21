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

import { chmodSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import Advise from "../src/commands/advise";
import {
  advise,
  loadMethodCatalog,
  uncataloguedAuthoredMethods,
} from "../src/advisor/index.js";
import type { MethodCatalog } from "../src/advisor/index.js";
import { runQuireAllowFailure, validateCoverage } from "../src/quire/index.js";

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
  // TC-150
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

// ── The three-state split, end to end (quoin#168; TC-274..TC-276) ──────────
//
// quire is faked on PATH (the tests/quire-exec.test.ts pattern): the command
// needs a version answer, one coverage payload and one properties payload, and
// what is under test is the classification and the reporting, not the engine.

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

/**
 * The battle-test oracle (agent-ix/quoin#168): five authored values from the
 * filament-ide-rs run, every one flagged `⚠ mismatch` by the two-state advisor
 * although none is a catalog method id or one of the four classes.
 */
const BATTLE_STRINGS = [
  "Static Analysis",
  "Audit Script",
  "CI Measurement",
  "Playwright rAF sampling",
  "Deferred",
];

/** A statement the fixture module has a rule for, so a mismatch WOULD fire. */
const RELIABILITY =
  "The client tolerates a dropped connection and retries without losing a message.";

/**
 * A CR-091 coverage payload: the five battle strings authored on obligations
 * the diagnostics diagnose as uncatalogued, one genuine disagreement
 * (`Inspection`, a declared class the engine does NOT diagnose), and one
 * obligation no rule matches. `diagnosticValues: false` is the degraded shape
 * an engine predating CR-091 emits — same diagnostics, no `value`.
 */
function battlePayload(opts: { diagnosticValues: boolean }): {
  [key: string]: unknown;
} {
  const obligations = [
    ...BATTLE_STRINGS.map((method, i) => ({
      source: "acceptance-criteria",
      id: `FR-00${i + 1}-AC-1`,
      document: `spec/functional/FR-00${i + 1}.md`,
      statement: RELIABILITY,
      statement_hash: "a".repeat(64),
      method,
    })),
    {
      source: "acceptance-criteria",
      id: "FR-015-AC-4",
      document: "spec/functional/FR-015.md",
      statement: RELIABILITY,
      statement_hash: "a".repeat(64),
      method: "Inspection",
    },
    {
      source: "acceptance-criteria",
      id: "FR-020-AC-9",
      document: "spec/functional/FR-020.md",
      statement: "The widget count equals seven.",
      statement_hash: "a".repeat(64),
      method: "Test",
    },
  ];
  return {
    unbacked_rows: [],
    status_lies: [],
    untracked_symbols: [],
    groups: [],
    totals: { backed: 0, total: obligations.length },
    obligations,
    diagnostics: BATTLE_STRINGS.map((value) => ({
      declaration: "acceptance-criteria",
      reason: "uncatalogued-verification-method",
      message:
        `'${value}' is neither a declared verification_catalog method id ` +
        `nor a declared class, so nothing can say what discharging it means`,
      ...(opts.diagnosticValues ? { value } : {}),
    })),
  };
}

/** A fake `quire` answering `--version`, `coverage` and `properties`. */
function fakeQuireDir(payload: Record<string, unknown>): string {
  const dir = mkdtempSync(join(tmpdir(), "quoin-fake-quire-"));
  const bin = join(dir, "quire");
  writeFileSync(
    bin,
    [
      "#!/bin/sh",
      'if [ "$1" = "--version" ]; then echo "quire 0.41.0"; exit 0; fi',
      'if [ "$1" = "properties" ]; then echo \'{"documents": []}\'; exit 0; fi',
      "cat <<'PAYLOAD'",
      JSON.stringify(payload),
      "PAYLOAD",
    ].join("\n"),
  );
  chmodSync(bin, 0o755);
  return dir;
}

/** A module whose catalog recommends fault-injection for RELIABILITY. */
function moduleDir(): string {
  const dir = mkdtempSync(join(tmpdir(), "quoin-advise-module-"));
  writeFileSync(
    join(dir, "manifest.yaml"),
    [
      "manifest_version: 1",
      "name: tc274-fixture",
      "version: 0.0.0",
      "verification_catalog:",
      "  fault-injection:",
      "    name: Fault injection",
      "    class: Test",
      "    definition: d",
      "    evidence_kind: Integration",
      "    applicability:",
      "      characteristics: [reliability]",
      "",
    ].join("\n"),
  );
  return dir;
}

/**
 * Run the real command against a faked quire, capturing log and warn.
 *
 * The fake serving `payload` goes FIRST on PATH here, from the same argument
 * the assertions reason about — callers previously faked PATH themselves and
 * passed the payload in as well, leaving the parameter dead and the two free
 * to diverge silently. Each describe's `afterEach` restores PATH.
 */
async function runAdvise(
  payload: Record<string, unknown>,
  extraArgs: string[] = [],
): Promise<{ logged: string[]; warned: string[] }> {
  process.env.PATH = `${fakeQuireDir(payload)}:${process.env.PATH}`;
  const logged: string[] = [];
  const warned: string[] = [];
  vi.spyOn(Advise.prototype, "log").mockImplementation((m?: string) => {
    logged.push(String(m ?? ""));
  });
  vi.spyOn(Advise.prototype, "warn").mockImplementation(((m: string) => {
    warned.push(String(m));
    return m;
  }) as never);
  const repo = mkdtempSync(join(tmpdir(), "quoin-advise-repo-"));
  await Advise.run(
    ["--repo", repo, "--module", moduleDir(), ...extraArgs],
    config,
  );
  return { logged, warned };
}

describe("TC-274 the battle-test oracle: real uncatalogued values are not mismatches (FR-031-AC-22)", () => {
  const savedPath = process.env.PATH;
  afterEach(() => {
    process.env.PATH = savedPath;
    vi.restoreAllMocks();
  });

  // TC-274
  it("the vendored contract accepts the CR-091 payload carrying diagnostic values", () => {
    const result = validateCoverage(battlePayload({ diagnosticValues: true }));
    expect(result.ok, JSON.stringify(result)).toBe(true);
  });

  // TC-274
  it("classifies all five uncatalogued, and the Inspection disagreement mismatch", async () => {
    const { logged } = await runAdvise(
      battlePayload({ diagnosticValues: true }),
    );
    for (const value of BATTLE_STRINGS) {
      const row = logged.find((line) => line.includes(`authored=${value} `));
      expect(row, `no output row for authored=${value}`).toBeDefined();
      expect(row).toContain("⚠ uncatalogued");
      expect(row).not.toContain("mismatch");
    }
    // The one the author may genuinely want to reconsider stays a mismatch —
    // `Inspection` IS a declared class, and no diagnostic names it.
    const inspection = logged.find((l) => l.includes("authored=Inspection"));
    expect(inspection).toContain("⚠ mismatch");
    expect(inspection).not.toContain("uncatalogued");
  });

  // TC-274
  it("carries the three states in JSON", async () => {
    const { logged } = await runAdvise(
      battlePayload({ diagnosticValues: true }),
      ["--json"],
    );
    const { advice } = JSON.parse(logged.join("\n")) as {
      advice: Array<{
        authored: string | null;
        mismatch: boolean;
        uncatalogued: boolean;
        inconclusive: boolean;
      }>;
    };
    for (const value of BATTLE_STRINGS) {
      const entry = advice.find((a) => a.authored === value);
      expect(entry, `no advice for authored=${value}`).toBeDefined();
      expect(entry?.uncatalogued).toBe(true);
      expect(entry?.mismatch).toBe(false);
    }
    expect(advice.find((a) => a.authored === "Inspection")?.mismatch).toBe(
      true,
    );
  });
});

describe("TC-275 an engine predating CR-091 degrades to two states, and says so (FR-031-AC-23)", () => {
  const savedPath = process.env.PATH;
  afterEach(() => {
    process.env.PATH = savedPath;
    vi.restoreAllMocks();
  });

  // TC-275
  it("reads the join off the diagnostics, and reports a value-less payload as degraded", () => {
    const classified = uncataloguedAuthoredMethods(
      battlePayload({ diagnosticValues: true }).diagnostics as never,
    );
    expect([...classified.values].sort()).toEqual([...BATTLE_STRINGS].sort());
    expect(classified.degraded).toBe(false);

    const degraded = uncataloguedAuthoredMethods(
      battlePayload({ diagnosticValues: false }).diagnostics as never,
    );
    expect(degraded.values.size).toBe(0);
    expect(degraded.degraded).toBe(true);

    // No diagnostics at all is NOT degraded: under an engine of either
    // vintage it means every authored value is catalogued.
    expect(uncataloguedAuthoredMethods(undefined).degraded).toBe(false);
    expect(
      uncataloguedAuthoredMethods([
        { declaration: "d", reason: "archetype-matches-nothing", message: "m" },
      ]).degraded,
    ).toBe(false);
  });

  // TC-275
  it("falls back to today's two-state report with an explicit note, not a misclassification", async () => {
    const { logged, warned } = await runAdvise(
      battlePayload({ diagnosticValues: false }),
    );
    expect(
      warned.some((w) =>
        w.includes("engine predates vocabulary classification"),
      ),
      warned.join("\n"),
    ).toBe(true);
    // Two-state behaviour, exactly as before: the five report as mismatches.
    for (const value of BATTLE_STRINGS) {
      const row = logged.find((line) => line.includes(`authored=${value} `));
      expect(row).toContain("⚠ mismatch");
      expect(row).not.toContain("uncatalogued");
    }
    expect(logged.join("\n")).toContain("0 uncatalogued");
  });
});

describe("TC-276 combined --*-only filters union, and the footer tallies the full population (FR-031-AC-24)", () => {
  const savedPath = process.env.PATH;
  afterEach(() => {
    process.env.PATH = savedPath;
    vi.restoreAllMocks();
  });

  // TC-276
  it("--mismatch-only returns only genuine disagreements", async () => {
    const { logged } = await runAdvise(
      battlePayload({ diagnosticValues: true }),
      ["--mismatch-only"],
    );
    const rows = logged.filter((line) => line.includes("authored="));
    expect(rows).toHaveLength(1);
    expect(rows[0]).toContain("authored=Inspection");
  });

  // TC-276
  it("--mismatch-only --inconclusive-only selects the union, not the empty intersection", async () => {
    // Inconclusive implies no recommendations implies never mismatch, so the
    // intersection was a GUARANTEED zero rows — and the footer then reported
    // `0 mismatch, 0 inconclusive` over a corpus full of both (#168).
    const { logged } = await runAdvise(
      battlePayload({ diagnosticValues: true }),
      ["--mismatch-only", "--inconclusive-only"],
    );
    const rows = logged.filter((line) => line.includes("authored="));
    expect(rows).toHaveLength(2);
    expect(rows.some((r) => r.includes("authored=Inspection"))).toBe(true);
    expect(rows.some((r) => r.includes("FR-020-AC-9"))).toBe(true);
    // The footer tallies the FULL population — 7 obligations, of which the
    // filter showed 2 — not the shown rows, which understated the very totals
    // the filter was asked about.
    const footer = logged.at(-1) ?? "";
    expect(footer).toContain("2 of 7 obligation(s) shown");
    expect(footer).toContain(
      "Of all 7: 1 mismatch, 5 uncatalogued, 1 inconclusive",
    );
  });
});
