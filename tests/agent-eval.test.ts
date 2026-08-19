/**
 * FR-042 — agent-eval reports as run evidence (TC-240..TC-244).
 *
 * `agent-eval-real.json` is a real `cli-agent-evals` report, checked in
 * unedited — the TC-EV-057 run of the `spec-fuzz` scenarios. The multi-scenario,
 * failing and empty cases are **constructed**, and labelled here as such: no
 * failing report survived to be captured, and fabricating one that claims to be
 * real would be worse than saying which is which.
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  parseAgentEval,
  selectAdapter,
  AdapterError,
} from "../src/evidence/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const realReport = readFileSync(
  join(here, "fixtures", "evidence", "agent-eval-real.json"),
  "utf8",
);

/** Constructed — see the module note. Shaped exactly like the real report. */
const constructed = (
  results: Array<{ id: string; ok: boolean; passRate?: string }>,
) => JSON.stringify({ ok: false, suite: "quoin", results });

describe("the agent-eval adapter", () => {
  // Trace: FR-042-AC-1
  it("reads a real report as one entry per scenario", () => {
    const result = parseAgentEval(realReport);
    expect(result.entries).toHaveLength(1);
    expect(result.entries[0]).toMatchObject({
      symbol: "TC-EV-057",
      outcome: "pass",
    });
  });

  // Trace: FR-042-AC-2
  it("uses the scenario id as its own trace id", () => {
    // `TC-EV-057` is what the matrix row names AND what the report keys on, so
    // the join FR-034 usually has to construct is already stated. Mapping it
    // would be inventing a second identity for the same thing.
    expect(parseAgentEval(realReport).entries[0].traceIds).toEqual([
      "TC-EV-057",
    ]);
  });

  // Trace: FR-042-AC-3
  it("takes the harness's verdict rather than recomputing it", () => {
    // A scenario passing 1 run of 3 is not a pass. `ok` is the suite's own
    // answer over `repeats`, and a second opinion here could disagree with the
    // report a human already read.
    const report = constructed([
      { id: "TC-EV-001", ok: true, passRate: "3/3" },
      { id: "TC-EV-002", ok: false, passRate: "1/3" },
    ]);
    const entries = parseAgentEval(report).entries;
    expect(entries.map((e) => e.outcome)).toEqual(["pass", "fail"]);
    // The rate is kept, because "flaky" and "failing" are different facts and
    // `outcome` alone cannot carry the difference.
    expect(entries[1].score).toBeCloseTo(1 / 3);
  });

  // Trace: FR-042-AC-4
  it("refuses a report with no scenario results", () => {
    // A suite that ran nothing. Recording it would manufacture evidence from a
    // file proving only that the harness started.
    expect(() => parseAgentEval(JSON.stringify({ results: [] }))).toThrow(
      /did the suite run/,
    );
    expect(() => parseAgentEval(JSON.stringify({ ok: true }))).toThrow(
      AdapterError,
    );
    expect(() => parseAgentEval("{{")).toThrow(/not JSON/);
  });

  // Trace: FR-042-AC-5
  it("is selected by --adapter and by the harness name", () => {
    expect(selectAdapter({ adapter: "agent-eval" }).name).toBe("agent-eval");
    expect(selectAdapter({ tool: "cli-agent-evals 0.4.0" }).name).toBe(
      "agent-eval",
    );
  });
});
