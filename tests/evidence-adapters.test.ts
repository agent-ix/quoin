/**
 * FR-033 — evidence format adapters (TC-151..TC-164).
 *
 * At least one criterion per adapter is stated over
 * `quoin evidence record --adapter <x> --results <file>`, not over the parse
 * function. The P1 review found three of four P0 gaps were a Test Matrix
 * reading ✅ over a capability nothing could reach, because the criteria were
 * written at the function boundary.
 */

import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { beforeAll, describe, expect, it } from "vitest";

import {
  ADAPTERS,
  ADAPTER_NAMES,
  AdapterError,
  cargoMutantsAdapter,
  junitAdapter,
  qualifiedName,
  selectAdapter,
} from "../src/evidence/index.js";
import EvidenceRecord from "../src/commands/evidence/record";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

/**
 * A repository the command will accept: `quoin evidence record` runs
 * `quire coverage` over `--repo`, which needs a real `spec/` tree. One FR is
 * enough — these criteria are about the adapter seam, not about coverage.
 */
function workspace(): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-adapters-"));
  mkdirSync(join(root, "spec", "functional"), { recursive: true });
  writeFileSync(
    join(root, "spec", "functional", "FR-001-a-requirement.md"),
    "---\nid: FR-001\ntype: FR\ntitle: A requirement\n---\n\n" +
      "## Description\n\nThe system shall do it.\n",
  );
  return root;
}

const JUNIT = `<?xml version="1.0"?>
<testsuites>
  <testsuite name="corpus">
    <testcase classname="tests.corpus" name="tc001_resolves">
      <properties><property name="trace" value="FR-001-AC-1, FR-001-AC-2"/></properties>
    </testcase>
    <testcase classname="tests.corpus" name="tc002_dangles"><failure message="boom"/></testcase>
    <testcase classname="tests.corpus" name="tc003_errors"><error message="panic"/></testcase>
    <testcase classname="tests.corpus" name="tc004_skipped"><skipped/></testcase>
  </testsuite>
</testsuites>`;

const MUTANTS = JSON.stringify({
  outcomes: [
    { scenario: "Baseline", summary: "Success" },
    {
      scenario: {
        Mutant: { file: "src/a.rs", function: { function_name: "resolve" } },
      },
      summary: "CaughtMutant",
    },
    {
      scenario: {
        Mutant: { file: "src/a.rs", function: { function_name: "resolve" } },
      },
      summary: "MissedMutant",
    },
    {
      scenario: {
        Mutant: { file: "src/a.rs", function: { function_name: "resolve" } },
      },
      summary: "Unviable",
    },
    {
      scenario: {
        Mutant: { file: "src/b.rs", function: { function_name: "harvest" } },
      },
      summary: "CaughtMutant",
    },
  ],
});

/** Record a run and return the entries the store persisted. */
async function record(
  root: string,
  args: string[],
  results: string,
  body: string,
): Promise<Array<Record<string, unknown>>> {
  writeFileSync(join(root, results), body);
  await EvidenceRecord.run(
    [
      "--repo",
      root,
      "--suite",
      "SUITE-001",
      "--commit",
      "0".repeat(40),
      "--results",
      join(root, results),
      ...args,
    ],
    config,
  );
  const runs = join(root, "spec", "evidence", "runs", "SUITE-001");
  const file = readdirSync(runs).find((f) => f.endsWith(".json"));
  const parsed = JSON.parse(
    readFileSync(join(runs, file as string), "utf8"),
  ) as {
    entries: Array<Record<string, unknown>>;
  };
  return parsed.entries;
}

describe("the adapter registry", () => {
  // Trace: FR-033-AC-1
  it("selects by --adapter, then by --tool, then falls back to entries", () => {
    expect(selectAdapter({ adapter: "junit" }).name).toBe("junit");
    // The suite's declared tool picks one when no adapter is named.
    expect(selectAdapter({ tool: "cargo-mutants 27.1.0" }).name).toBe(
      "cargo-mutants",
    );
    expect(selectAdapter({ tool: "pytest 8.0" }).name).toBe("junit");
    // Nothing claims it → the normalized shape, which is always writable by hand.
    expect(selectAdapter({ tool: "some-bespoke-runner" }).name).toBe("entries");
    expect(selectAdapter({}).name).toBe("entries");
  });

  // Trace: FR-033-AC-2
  it("rejects an unknown --adapter instead of silently using the default", () => {
    // Falling back would parse a JUnit file as normalized JSON and complain
    // about JSON shape, sending the reader to their XML rather than their typo.
    expect(() => selectAdapter({ adapter: "junitt" })).toThrow(AdapterError);
    expect(() => selectAdapter({ adapter: "junitt" })).toThrow(/Available: /);
  });

  // Trace: FR-033-AC-3
  it("registers adapters as data, so an external tool can be added", () => {
    // Exact, not a length check: ADAPTER_NAMES is what `--adapter` accepts and
    // what `--help` lists, so a silently added or dropped name is a change to
    // the command's surface. Run-shaped first, then finding-shaped (FR-034).
    expect(ADAPTER_NAMES).toEqual([
      "entries",
      "junit",
      "cargo-mutants",
      "sarif",
      "audit-script",
      "cargo-audit",
    ]);
    for (const adapter of ADAPTERS) {
      expect(adapter.summary.length).toBeGreaterThan(0);
      expect(typeof adapter.parse).toBe("function");
    }
  });
});

describe("the junit adapter", () => {
  // Trace: FR-033-AC-4
  it("maps classname + name to the qualified name the extractor emits", () => {
    // The join, and the whole job: a tool's test name is NOT a symbol identity.
    expect(qualifiedName("tests.corpus", "tc001")).toBe("tests::corpus::tc001");
    expect(qualifiedName("", "bare")).toBe("bare");
    expect(qualifiedName("only", "")).toBe("only");
    // Runners that repeat the name must not double it.
    expect(qualifiedName("tests::tc001", "tc001")).toBe("tests::tc001");
    expect(qualifiedName("same", "same")).toBe("same");
  });

  // Trace: FR-033-AC-5
  it("reads every outcome class and the declared trace ids", () => {
    const { entries } = junitAdapter.parse(JUNIT);
    expect(entries.map((e) => e.outcome)).toEqual([
      "pass",
      "fail",
      "error",
      "skip",
    ]);
    expect(entries[0].traceIds).toEqual(["FR-001-AC-1", "FR-001-AC-2"]);
    expect(entries[1].traceIds).toBeUndefined();
  });

  // Trace: FR-033-AC-6
  it("names no evidence kind, because JUnit does not carry one", () => {
    // Unit, integration and e2e suites all emit JUnit. An adapter answering
    // "Unit" would assert something the format does not contain — and would
    // mint a fourth copy of a vocabulary that already exists in three places.
    expect(junitAdapter.parse(JUNIT).evidenceKind).toBeUndefined();
  });

  // Trace: FR-033-AC-7
  it("rejects input carrying no testcase rather than recording an empty run", () => {
    // An empty run is indistinguishable from a suite that passed nothing, which
    // is exactly the state a freshness check must not be fed.
    expect(() => junitAdapter.parse("<testsuites/>")).toThrow(AdapterError);
    expect(() => junitAdapter.parse("not xml at all")).toThrow(/no <testcase>/);
  });
});

describe("the cargo-mutants adapter", () => {
  // Trace: FR-033-AC-8
  it("carries a native score, which is why this format is in scope", () => {
    const { entries } = cargoMutantsAdapter.parse(MUTANTS);
    const resolve = entries.find((e) => e.symbol === "src/a.rs::resolve");
    // 1 caught, 1 missed, 1 unviable → 1/2. The unviable mutant is in NEITHER
    // side: it does not compile, so it exercised no test and belongs in no
    // denominator. Counting it would report 1/3 and say nothing true.
    expect(resolve?.score).toBe(0.5);
    expect(resolve?.outcome).toBe("fail");
    const harvest = entries.find((e) => e.symbol === "src/b.rs::harvest");
    expect(harvest?.score).toBe(1);
    expect(harvest?.outcome).toBe("pass");
  });

  // Trace: FR-033-AC-9
  it("reports the tool's own classification, not a threshold", () => {
    // A surviving mutant is a demonstrated gap that cargo-mutants itself
    // classified. Whether a score is ACCEPTABLE is the auditor's and the
    // consumer gate's decision, never the adapter's.
    const { entries } = cargoMutantsAdapter.parse(MUTANTS);
    expect(entries.every((e) => e.score !== undefined)).toBe(true);
    expect(new Set(entries.map((e) => e.outcome))).toEqual(
      new Set(["pass", "fail"]),
    );
  });

  // Trace: FR-033-AC-10
  it("rejects malformed and empty reports", () => {
    expect(() => cargoMutantsAdapter.parse("{")).toThrow(/not JSON/);
    expect(() => cargoMutantsAdapter.parse("{}")).toThrow(
      /no `outcomes` array/,
    );
    expect(() =>
      cargoMutantsAdapter.parse(
        JSON.stringify({ outcomes: [{ scenario: "Baseline" }] }),
      ),
    ).toThrow(/no viable mutants/);
    // A mutant naming no function cannot be attributed and is skipped, which
    // must not be mistaken for a viable one.
    expect(() =>
      cargoMutantsAdapter.parse(
        JSON.stringify({
          outcomes: [{ scenario: { Mutant: {} }, summary: "CaughtMutant" }],
        }),
      ),
    ).toThrow(/no viable mutants/);
  });
});

describe("quoin evidence record --adapter", () => {
  // Trace: FR-033-AC-11
  it("records a JUnit file end to end, through the command", async () => {
    const root = workspace();
    const entries = await record(
      root,
      ["--tool", "pytest 8.0", "--adapter", "junit"],
      "results.xml",
      JUNIT,
    );
    expect(entries).toHaveLength(4);
    expect(entries[0].symbol).toBe("tests::corpus::tc001_resolves");
    expect(entries[0].traceIds).toEqual(["FR-001-AC-1", "FR-001-AC-2"]);
  });

  // Trace: FR-033-AC-12
  it("records a cargo-mutants report end to end, preserving score", async () => {
    const root = workspace();
    const entries = await record(
      root,
      ["--tool", "cargo-mutants 27.1.0", "--adapter", "cargo-mutants"],
      "outcomes.json",
      MUTANTS,
    );
    expect(entries.map((e) => e.score)).toEqual([0.5, 1]);
  });

  // Trace: FR-033-AC-13
  it("selects the adapter from --tool when none is named", async () => {
    const root = workspace();
    const entries = await record(
      root,
      ["--tool", "pytest 8.0"],
      "results.xml",
      JUNIT,
    );
    expect(entries).toHaveLength(4);
  });

  // Trace: FR-033-AC-14
  it("still accepts the normalized shape with no adapter at all", async () => {
    // The escape hatch: a consumer whose tool no adapter reads writes entries
    // by hand, so the registry is never a gate on recording evidence.
    const root = workspace();
    const entries = await record(
      root,
      ["--tool", "bespoke 1.0"],
      "run.json",
      JSON.stringify({
        entries: [{ symbol: "tests::tc001", outcome: "pass" }],
      }),
    );
    expect(entries).toEqual([{ symbol: "tests::tc001", outcome: "pass" }]);
  });
});

describe("adapter edge cases the 100% gate requires", () => {
  // Trace: FR-033-AC-7
  it("decodes XML entities and ignores properties it does not recognise", () => {
    const xml = `<testsuites><testcase classname="a&amp;b" name="t&lt;1&gt;">
      <properties>
        <property name="owner" value="someone"/>
        <property name="trace_ids" value="FR-002-AC-1"/>
        <property value="no name"/>
        <property name="no value"/>
      </properties>
    </testcase></testsuites>`;
    const { entries } = junitAdapter.parse(xml);
    // The five predefined entities are decoded; nothing else is.
    expect(entries[0].symbol).toBe("a&b::t<1>");
    // `trace_ids` is accepted alongside `trace`/`traceIds`; `owner` is not a
    // trace property and a property missing either attribute is skipped rather
    // than throwing — a runner's extra metadata must not break a transcript.
    expect(entries[0].traceIds).toEqual(["FR-002-AC-1"]);
  });

  // Trace: FR-033-AC-5
  it("reads a self-closing testcase as a pass and skips a nameless one", () => {
    const { entries } = junitAdapter.parse(
      `<testsuites><testcase classname="a" name="t"/><testcase classname="" name=""/></testsuites>`,
    );
    expect(entries).toEqual([{ symbol: "a::t", outcome: "pass" }]);
  });

  // Trace: FR-033-AC-4
  it("does not double a classname that already ends with the member", () => {
    expect(qualifiedName("mod.tests.tc001", "tc001")).toBe("mod::tests::tc001");
  });

  // Trace: FR-033-AC-10
  it("counts a timed-out mutant as survived, not as absent", () => {
    // A mutant that hung was not caught. Dropping it would quietly raise the
    // score by removing the evidence that the suite did not kill it.
    const { entries } = cargoMutantsAdapter.parse(
      JSON.stringify({
        outcomes: [
          {
            scenario: {
              Mutant: { file: "src/a.rs", function: { function_name: "f" } },
            },
            summary: "Timeout",
          },
        ],
      }),
    );
    expect(entries).toEqual([
      { symbol: "src/a.rs::f", outcome: "fail", score: 0 },
    ]);
  });

  // Trace: FR-033-AC-10
  it("skips an outcome whose scenario is neither Baseline nor a Mutant", () => {
    expect(() =>
      cargoMutantsAdapter.parse(
        JSON.stringify({
          outcomes: [{ scenario: 7 }, { scenario: { Other: {} } }],
        }),
      ),
    ).toThrow(/no viable mutants/);
  });

  // Trace: FR-033-AC-2
  it("reports a malformed normalized payload against the entries adapter", () => {
    const { entriesAdapter } = { entriesAdapter: selectAdapter({}) };
    expect(() => entriesAdapter.parse("{")).toThrow(/not JSON/);
    expect(() => entriesAdapter.parse("{}")).toThrow(/entries/);
  });
});

describe("junit attribute and separator edges", () => {
  // Trace: FR-033-AC-4
  it("handles a testcase with no classname attribute at all", () => {
    const { entries } = junitAdapter.parse(
      `<testsuites><testcase name="bare_test"/></testsuites>`,
    );
    expect(entries[0].symbol).toBe("bare_test");
  });

  // Trace: FR-033-AC-5
  it("drops empty ids produced by trailing or repeated separators", () => {
    const { entries } = junitAdapter.parse(
      `<testsuites><testcase classname="a" name="t">
        <properties><property name="trace" value=" FR-1 ,, FR-2 , "/></properties>
      </testcase></testsuites>`,
    );
    expect(entries[0].traceIds).toEqual(["FR-1", "FR-2"]);
  });
});

// Trace: FR-033-AC-7
it("skips a testcase carrying neither classname nor name", () => {
  // Nothing to key an entry on. Recording it under an empty symbol would put a
  // row in the store that can never match a declared symbol, which reads as
  // evidence while binding to nothing.
  const { entries } = junitAdapter.parse(
    `<testsuites><testcase time="0.1"/><testcase classname="a" name="t"/></testsuites>`,
  );
  expect(entries).toEqual([{ symbol: "a::t", outcome: "pass" }]);
});
