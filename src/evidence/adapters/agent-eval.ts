import type { RunEntry } from "../types.js";
import {
  AdapterError,
  type AdapterResult,
  type EvidenceAdapter,
} from "./types.js";

/**
 * `cli-agent-evals` reports → run entries, one per scenario (FR-042).
 *
 * **Why this is worth an adapter at all.** An eval drives the real agent through
 * the real CLI, which makes it the most convincing verification this project
 * has — and until now the least recorded. `quire coverage` reconciles matrix
 * rows against test symbols in code; eval scenarios are data in
 * `evals/scenarios/index.mjs`, so they mint no symbol and can never back a row.
 *
 * Measured at the time of writing: **71 unbacked rows** across `spec/evals.md`,
 * `FR-028` and `FR-038`, every one of them a criterion whose ✅ rested on
 * somebody having run the evals (`SR-008` FND-001). For FR-038 they had been
 * run — four scenarios, 4/4, live — and the repository dropped the report.
 *
 * With a run record, an eval suite is subject to the same freshness, staleness
 * and vacuity checks as any other, and a matrix row is backed by a fact rather
 * than by trust.
 *
 * **The scenario id IS the symbol.** `TC-EV-054` is what the matrix row names
 * and what the report keys on, so no mapping is needed and none is invented —
 * the join FR-034 usually has to construct is already stated here.
 */
export function parseAgentEval(raw: string): AdapterResult {
  let report: unknown;
  try {
    report = JSON.parse(raw);
  } catch (cause) {
    throw new AdapterError(
      "agent-eval",
      `not JSON: ${cause instanceof Error ? cause.message : String(cause)}`,
    );
  }
  const root = report as { results?: unknown };
  if (!root || typeof root !== "object" || !Array.isArray(root.results)) {
    throw new AdapterError(
      "agent-eval",
      'expected a cli-agent-evals report with a "results" array',
    );
  }

  const entries: RunEntry[] = [];
  for (const raw of root.results as Array<Record<string, unknown>>) {
    const id = typeof raw.id === "string" ? raw.id : null;
    if (!id) continue;
    entries.push({
      symbol: id,
      // `ok` is the suite's own verdict over `repeats` runs — a scenario that
      // passed 1 of 3 is NOT a pass, and recomputing that here would be a
      // second opinion on a question the harness already answered.
      outcome: raw.ok === true ? "pass" : "fail",
      // The scenario id is also the trace id: `TC-EV-054` is what the matrix
      // row names. Stated rather than mapped.
      traceIds: [id],
      ...scoreOf(raw),
    });
  }

  if (entries.length === 0) {
    // A report with no results is a suite that ran nothing. Recording it would
    // manufacture evidence from a file proving only that the harness started.
    throw new AdapterError(
      "agent-eval",
      "no scenario results in the report — did the suite run?",
    );
  }
  return { entries };
}

/**
 * `passRate` as a ratio, where the report states one.
 *
 * `"2/3"` becomes `0.666…`. Recorded because a scenario that passes two runs in
 * three is flaky rather than passing, and `outcome` alone cannot say so — but
 * note this is NOT a mutation score: `agent-ix/quoin#138` is why `score` needs
 * a metric discriminator before any threshold reads it.
 */
function scoreOf(raw: Record<string, unknown>): { score?: number } {
  const rate = raw.passRate;
  if (typeof rate !== "string") return {};
  const [passed, total] = rate.split("/").map(Number);
  if (!Number.isFinite(passed) || !Number.isFinite(total) || total === 0) {
    return {};
  }
  return { score: passed / total };
}

export const agentEvalAdapter: EvidenceAdapter = {
  name: "agent-eval",
  summary:
    "cli-agent-evals report JSON — one entry per scenario, keyed on the scenario id.",
  tools: ["cli-agent-evals", "agent-evals", "agent-pty"],
  parse: parseAgentEval,
};
