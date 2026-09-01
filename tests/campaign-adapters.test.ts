/**
 * Quoin #323 — campaign-native result adapters (FR-069).
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

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import fc from "fast-check";
import { beforeAll, describe, expect, it, vi } from "vitest";

import EvidenceRecord from "../src/commands/evidence/record.js";

import {
  ADAPTER_NAMES,
  AdapterError,
  contractConformanceAdapter,
  differentialReportAdapter,
  selectAdapter,
} from "../src/evidence/index.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const fixtures = join(repoRoot, "tests/fixtures/evidence");
const adapterRoot = join(repoRoot, "src/evidence/adapters");
const inventory = join(repoRoot, "docs/campaign-native-result-inventory.md");

const conformance = readFileSync(
  join(fixtures, "contract-conformance-real.jsonl"),
  "utf8",
);
const differential = readFileSync(
  join(fixtures, "differential-report-real.json"),
  "utf8",
);

let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

/** A minimal repository the record command can resolve obligations against. */
function repository(): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-campaign-adapters-"));
  mkdirSync(join(root, "spec", "functional"), { recursive: true });
  writeFileSync(
    join(root, "spec", "functional", "FR-001-a-requirement.md"),
    "---\nid: FR-001\ntype: FR\ntitle: A requirement\n---\n\n" +
      "## Description\n\nThe system shall do it.\n",
  );
  return root;
}

describe("FR-069 contract conformance", () => {
  it("TC-1328 transcribes a real conformance run, keyed by corpus, operation, and fixture", () => {
    const rows = conformance.split("\n").filter((line) => line.trim() !== "");
    const result = contractConformanceAdapter.parse(conformance);

    expect(result.entries).toHaveLength(rows.length);
    for (const [index, line] of rows.entries()) {
      const row = JSON.parse(line) as {
        corpus_id: string;
        operation: string;
        fixture_id: string;
        status: string;
      };
      expect(result.entries[index].symbol).toBe(
        `${row.corpus_id}::${row.operation}::${row.fixture_id}`,
      );
      expect(result.entries[index].outcome).toBe(
        row.status === "match" ? "pass" : "fail",
      );
    }

    // Distinct identities: the same fixture id under two operations must not
    // collapse into one entry that overwrites the other.
    expect(new Set(result.entries.map((entry) => entry.symbol)).size).toBe(
      rows.length,
    );

    // The real sample is real: it spans every operation the runner emits, and
    // it carries both a valid and an invalid fixture.
    const operations = new Set(
      rows.map((line) => (JSON.parse(line) as { operation: string }).operation),
    );
    expect([...operations].sort()).toEqual([
      "coverage",
      "expression",
      "migration",
      "package",
    ]);
    // Not every operation reports validity — coverage and migration rows
    // report other shapes — so this asserts over the rows that do.
    const validity = new Set(
      rows
        .map(
          (line) =>
            (JSON.parse(line) as { actual?: { valid?: boolean } }).actual
              ?.valid,
        )
        .filter((valid) => valid !== undefined),
    );
    expect(validity).toEqual(new Set([true, false]));
  });

  it("TC-1329 transcribes a real differential report, one entry per compared case", () => {
    const report = JSON.parse(differential) as {
      schemaVersion: string;
      cases: { id: string; status: string }[];
    };
    const result = differentialReportAdapter.parse(differential);

    expect(report.schemaVersion).toBe("tl-mltl.differential-summary/v1");
    const agreements = report.cases.filter((c) => c.status === "agreement");
    expect(result.entries).toHaveLength(agreements.length);
    expect(result.entries.map((entry) => entry.symbol)).toEqual(
      agreements.map((c) => c.id),
    );
    expect(new Set(result.entries.map((entry) => entry.outcome))).toEqual(
      new Set(["pass"]),
    );
  });

  it("TC-1330 names an unsupported case rather than transcribing it as another state", () => {
    const report = JSON.parse(differential) as {
      cases: { id: string; status: string }[];
    };
    const unsupported = report.cases.filter((c) => c.status === "unsupported");
    expect(unsupported.length).toBeGreaterThan(0);

    const result = differentialReportAdapter.parse(differential);
    expect(result.unrepresented).toHaveLength(unsupported.length);
    for (const [index, entry] of unsupported.entries()) {
      expect(result.unrepresented?.[index]).toMatchObject({
        symbol: entry.id,
        state: "unsupported",
      });
      expect(result.unrepresented?.[index].reason).toMatch(
        /neither a skip nor an error/,
      );
    }

    // It appears in no other guise: not as an entry, and not as any outcome.
    const symbols = result.entries.map((entry) => entry.symbol);
    for (const entry of unsupported) expect(symbols).not.toContain(entry.id);

    // A report with nothing unrepresentable omits the field rather than
    // asserting an empty list, so a present-but-empty list can never mean
    // "the adapter did not look".
    const clean = JSON.stringify({
      schemaVersion: "tl-mltl.differential-summary/v1",
      cases: [{ id: "a", status: "agreement" }],
    });
    expect(
      differentialReportAdapter.parse(clean).unrepresented,
    ).toBeUndefined();
  });

  it("TC-1331 refuses every malformed, unknown, and empty input by line or case", () => {
    const row = conformance.split("\n")[0];
    const parsed = JSON.parse(row) as Record<string, unknown>;

    const conformanceRefusals: [string, RegExp][] = [
      ["", /no conformance rows/],
      ["   \n\n", /no conformance rows/],
      ["{not json", /line 1 is not JSON/],
      [
        JSON.stringify({
          ...parsed,
          protocol: "quire.contract.conformance-jsonl/v2",
        }),
        /declares protocol .*expected/,
      ],
      [JSON.stringify({ ...parsed, status: "partial" }), /unknown status/],
      [JSON.stringify({ ...parsed, fixture_id: "" }), /has no fixture_id/],
      [
        `${row}\n${JSON.stringify({ ...parsed, operation: undefined })}`,
        /line 2 has no operation/,
      ],
    ];
    for (const [input, message] of conformanceRefusals) {
      expect(
        () => contractConformanceAdapter.parse(input),
        input.slice(0, 40),
      ).toThrow(AdapterError);
      expect(() => contractConformanceAdapter.parse(input)).toThrow(message);
    }

    const differentialRefusals: [string, RegExp][] = [
      ["{not json", /not JSON/],
      [JSON.stringify({ cases: [] }), /unknown schemaVersion/],
      [
        JSON.stringify({
          schemaVersion: "tl-mltl.differential-summary/v2",
          cases: [],
        }),
        /unknown schemaVersion/,
      ],
      [
        JSON.stringify({ schemaVersion: "tl-mltl.differential-summary/v1" }),
        /no cases array/,
      ],
      [
        JSON.stringify({
          schemaVersion: "tl-mltl.differential-summary/v1",
          cases: [],
        }),
        /examined nothing/,
      ],
      [
        JSON.stringify({
          schemaVersion: "tl-mltl.differential-summary/v1",
          cases: [{ id: "a", status: "partial" }],
        }),
        /unknown status/,
      ],
      [
        JSON.stringify({
          schemaVersion: "tl-mltl.differential-summary/v1",
          cases: [{ status: "agreement" }],
        }),
        /has no id/,
      ],
    ];
    for (const [input, message] of differentialRefusals) {
      expect(() => differentialReportAdapter.parse(input)).toThrow(
        AdapterError,
      );
      expect(() => differentialReportAdapter.parse(input)).toThrow(message);
    }

    // The inherited property names an object literal would have resolved.
    // CI found this with the counterexample "valueOf" after a local run of the
    // same property passed on a different seed — the reason the property is
    // here rather than a fixed list of statuses I thought of.
    for (const inherited of [
      "valueOf",
      "toString",
      "constructor",
      "hasOwnProperty",
      "__proto__",
    ]) {
      expect(
        () =>
          differentialReportAdapter.parse(
            JSON.stringify({
              schemaVersion: "tl-mltl.differential-summary/v1",
              cases: [{ id: "case", status: inherited }],
            }),
          ),
        inherited,
      ).toThrow(AdapterError);
      expect(
        () =>
          contractConformanceAdapter.parse(
            JSON.stringify({ ...parsed, status: inherited }),
          ),
        inherited,
      ).toThrow(AdapterError);
    }

    // No status outside the declared vocabularies is ever accepted, however
    // plausible it looks.
    fc.assert(
      fc.property(
        fc
          .string({ minLength: 1 })
          .filter(
            (status) =>
              !["agreement", "mismatch", "tool-error", "unsupported"].includes(
                status,
              ),
          ),
        (status) => {
          expect(() =>
            differentialReportAdapter.parse(
              JSON.stringify({
                schemaVersion: "tl-mltl.differential-summary/v1",
                cases: [{ id: "case", status }],
              }),
            ),
          ).toThrow(AdapterError);
        },
      ),
      { numRuns: 40 },
    );
  });

  it("TC-1332 registers both adapters by name and by declared tool", () => {
    expect(ADAPTER_NAMES).toContain("contract-conformance");
    expect(ADAPTER_NAMES).toContain("differential-report");

    expect(selectAdapter({ adapter: "contract-conformance" }).name).toBe(
      "contract-conformance",
    );
    expect(selectAdapter({ adapter: "differential-report" }).name).toBe(
      "differential-report",
    );
    expect(
      selectAdapter({ tool: "quire-contract-conformance 0.1.0" }).name,
    ).toBe("contract-conformance");
    expect(selectAdapter({ tool: "tl-mltl 0.1.0" }).name).toBe(
      "differential-report",
    );
  });
});

describe("FR-069 recording", () => {
  it("TC-1333 prints every unrepresented result in human and JSON output", async () => {
    const root = repository();
    const results = join(root, "differential.json");
    writeFileSync(results, differential);

    const args = [
      "--repo",
      root,
      "--suite",
      "SUITE-001",
      "--commit",
      "0".repeat(40),
      "--tool",
      "tl-mltl 0.1.0",
      "--adapter",
      "differential-report",
      "--results",
      results,
    ];

    const human: string[] = [];
    vi.spyOn(console, "log").mockImplementation((line) =>
      human.push(String(line)),
    );
    await EvidenceRecord.run(args, config);
    // Named, not counted, and never dropped: the reader is told which result
    // the record is short by and why.
    expect(human.join("\n")).toContain("not transcribed");
    expect(human.join("\n")).toContain("closed-profile-not-mapped-v1");
    expect(human.join("\n")).toContain("unsupported");

    const machine: string[] = [];
    vi.spyOn(console, "log").mockImplementation((line) =>
      machine.push(String(line)),
    );
    await EvidenceRecord.run([...args, "--json"], config);
    const payload = JSON.parse(machine.join("\n")) as {
      unrepresented?: { symbol: string; state: string }[];
    };
    expect(payload.unrepresented).toEqual([
      {
        symbol: "closed-profile-not-mapped-v1",
        state: "unsupported",
        reason: expect.stringContaining("neither a skip nor an error"),
      },
    ]);
    vi.restoreAllMocks();

    // No new record family: the persisted run is the existing shape, and the
    // unrepresented results are reported rather than stored as a fifth state.
    const runs = join(root, "spec", "evidence", "runs", "SUITE-001");
    const file = readdirSync(runs).find((name) => name.endsWith(".json"));
    const stored = JSON.parse(
      readFileSync(join(runs, file as string), "utf8"),
    ) as { entries: { symbol: string; outcome: string }[] };
    expect(stored.entries.map((entry) => entry.outcome)).toEqual(
      Array(8).fill("pass"),
    );
    expect(stored.entries.map((entry) => entry.symbol)).not.toContain(
      "closed-profile-not-mapped-v1",
    );
  });
});

describe("FR-069 inventory and boundaries", () => {
  it("TC-1334 records a producer and a verdict for every scope item", () => {
    const text = readFileSync(inventory, "utf8");

    // Every format #323 names is dispositioned, by name.
    for (const item of [
      "JUnit",
      "SARIF",
      "cargo-mutants",
      "Audit-script",
      "Kani",
      "Solver analysis",
      "Contract conformance JSONL",
      "Corpus reports",
      "Measurements",
      "Counterexamples",
      "differential report",
    ]) {
      expect(text, `${item} is not dispositioned`).toMatch(
        new RegExp(item.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "i"),
      );
    }

    // Every added adapter names a real producer and a pinned sample.
    for (const sample of [
      "contract-conformance-real.jsonl",
      "differential-report-real.json",
    ]) {
      expect(text).toContain(sample);
      expect(readdirSync(fixtures)).toContain(sample);
    }
    expect(text).toContain("agent-ix/quire-contract-ir");
    expect(text).toContain("agent-ix/tl-mltl");

    // The out-of-scope rule is stated, not implied.
    expect(text).toMatch(/stdout or stderr scraping is out of\nscope/);
  });

  it("TC-1335 executes nothing and scrapes no console text for a verdict", () => {
    for (const name of ["contract-conformance.ts", "differential-report.ts"]) {
      const text = readFileSync(join(adapterRoot, name), "utf8");
      for (const forbidden of [
        "child_process",
        "execSync",
        "spawnSync",
        "spawn(",
        "fetch(",
        "readFileSync",
        "https://",
      ]) {
        expect(text, `${name} must not reach for ${forbidden}`).not.toContain(
          forbidden,
        );
      }
      // Both readers are pure over text and declare what they accept, so a
      // format that does not say what it is cannot be read by accident.
      expect(text).toMatch(/PROTOCOL|SCHEMA/);
    }
  });
});
