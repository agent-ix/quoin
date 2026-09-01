import type { RunEntry } from "../types.js";
import {
  AdapterError,
  type AdapterResult,
  type EvidenceAdapter,
  type UnrepresentedResult,
} from "./types.js";

/**
 * The declared schema family this adapter reads.
 *
 * A domain declares its own prefix — `tl-mltl.differential-summary/v1` — so
 * two domains can use the same shape without either claiming the other's
 * identity. The version is pinned: a v2 with different semantics must not be
 * read by a v1 reader that happens to find the fields it knows.
 */
const SCHEMA = /^[a-z0-9][a-z0-9-]*\.differential-summary\/v1$/;

/**
 * Case states this adapter transcribes into run entries.
 *
 * `unsupported` is deliberately absent — see {@link UnrepresentedResult}.
 */
const STATUS: Record<string, RunEntry["outcome"]> = {
  agreement: "pass",
  mismatch: "fail",
  "tool-error": "error",
};

interface DifferentialCase {
  id?: unknown;
  status?: unknown;
}

interface DifferentialReport {
  schemaVersion?: unknown;
  externalTool?: unknown;
  cases?: unknown;
}

/**
 * Domain differential reports — a domain engine compared against an external
 * reference implementation, case by case.
 *
 * Produced by `agent-ix/tl-mltl` against R2U2/C2PO, and shaped for any domain
 * that runs the same comparison. The report states agreement, mismatch,
 * unsupported and tool-error counts; it reaches no verdict about whether the
 * comparison was sufficient, and neither does this.
 *
 * No existing adapter reads it. `junit` has no state for "the external tool
 * cannot express this case", `cargo-mutants` is mutation-shaped, `agent-eval`
 * is measurement-shaped, and the finding-shaped adapters would record a
 * disagreement as a rule violation — which is a different claim from "two
 * implementations disagree", and one the report does not make.
 */
export const differentialReportAdapter: EvidenceAdapter = {
  name: "differential-report",
  summary:
    "Domain differential summary (<domain>.differential-summary/v1) — engine versus external reference, case by case.",
  tools: ["differential-report", "tl-mltl", "r2u2", "c2po"],
  parse(raw: string): AdapterResult {
    let report: DifferentialReport;
    try {
      report = JSON.parse(raw) as DifferentialReport;
    } catch (error) {
      throw new AdapterError(
        "differential-report",
        `input is not JSON: ${(error as Error).message}`,
      );
    }
    if (
      typeof report.schemaVersion !== "string" ||
      !SCHEMA.test(report.schemaVersion)
    ) {
      throw new AdapterError(
        "differential-report",
        `unknown schemaVersion ${JSON.stringify(report.schemaVersion)}; expected <domain>.differential-summary/v1`,
      );
    }
    if (!Array.isArray(report.cases)) {
      throw new AdapterError(
        "differential-report",
        "report has no cases array",
      );
    }
    if (report.cases.length === 0) {
      // A comparison that examined nothing agrees with nothing. Recording it
      // as a clean run is how a vacuous differential becomes evidence.
      throw new AdapterError(
        "differential-report",
        "report contains no cases; a comparison that examined nothing is not an agreement",
      );
    }

    const entries: RunEntry[] = [];
    const unrepresented: UnrepresentedResult[] = [];
    (report.cases as DifferentialCase[]).forEach((entry, index) => {
      if (typeof entry?.id !== "string" || entry.id === "") {
        throw new AdapterError(
          "differential-report",
          `case ${index} has no id`,
        );
      }
      if (typeof entry.status !== "string" || entry.status === "") {
        throw new AdapterError(
          "differential-report",
          `case ${entry.id} has no status`,
        );
      }
      if (entry.status === "unsupported") {
        unrepresented.push({
          symbol: entry.id,
          state: "unsupported",
          reason:
            "the external reference does not support this case; that is " +
            "neither a skip nor an error, and no run-entry outcome carries it",
        });
        return;
      }
      const outcome = STATUS[entry.status];
      if (outcome === undefined) {
        throw new AdapterError(
          "differential-report",
          `case ${entry.id} has unknown status ${JSON.stringify(entry.status)}`,
        );
      }
      entries.push({ symbol: entry.id, outcome });
    });

    return unrepresented.length > 0 ? { entries, unrepresented } : { entries };
  },
};
