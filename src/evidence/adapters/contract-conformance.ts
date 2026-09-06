import type { RunEntry } from "../types.js";
import {
  AdapterError,
  type AdapterResult,
  type EvidenceAdapter,
} from "./types.js";

/**
 * The one protocol this adapter reads.
 *
 * Named rather than sniffed: a JSONL stream with different semantics under a
 * different protocol would parse just as cleanly and mean something else, and
 * a conformance run is exactly the place where a silently-misread row becomes
 * a passing fixture that was never checked.
 */
const PROTOCOL = "quire.contract.conformance-jsonl/v1";

/** Every status the protocol declares. An unknown one is refused, not skipped. */
const STATUS = new Map<string, RunEntry["outcome"]>([
  ["match", "pass"],
  ["mismatch", "fail"],
]);

interface ConformanceRow {
  protocol?: unknown;
  corpus_id?: unknown;
  fixture_id?: unknown;
  operation?: unknown;
  status?: unknown;
  mismatch_kinds?: unknown;
  trace_ids?: unknown;
}

/**
 * Contract conformance JSONL — one replayed corpus fixture per line.
 *
 * Produced by `quire-contract-conformance run --manifest <corpus>` in
 * `agent-ix/quire-contract-ir`. The runner replays a pinned corpus and reports
 * whether each fixture's canonicalization matched; it decides nothing about
 * sufficiency, and neither does this.
 *
 * No existing adapter reads it. `entries` expects one JSON object with an
 * `entries` array, `junit` expects XML, and the finding-shaped adapters would
 * write it into `findings/` — where the clean-versus-unrun distinction that
 * FR-034 exists to make would be lost for every fixture in the corpus.
 */
export const contractConformanceAdapter: EvidenceAdapter = {
  name: "contract-conformance",
  summary: `Contract conformance JSONL (${PROTOCOL}) — one replayed fixture per line.`,
  tools: [
    "quire-contract-conformance",
    "contract-conformance",
    "quire-contract-ir",
  ],
  parse(raw: string): AdapterResult {
    const lines = raw.split("\n").filter((line) => line.trim() !== "");
    if (lines.length === 0) {
      // An empty stream is not an empty corpus: a runner that produced no line
      // did not report that every fixture matched, it reported nothing.
      throw new AdapterError(
        "contract-conformance",
        "input contains no conformance rows; an empty run is not a clean run",
      );
    }

    const entries: RunEntry[] = [];
    lines.forEach((line, index) => {
      let row: ConformanceRow;
      try {
        row = JSON.parse(line) as ConformanceRow;
      } catch (error) {
        throw new AdapterError(
          "contract-conformance",
          `line ${index + 1} is not JSON: ${(error as Error).message}`,
        );
      }
      if (row.protocol !== PROTOCOL) {
        throw new AdapterError(
          "contract-conformance",
          `line ${index + 1} declares protocol ${JSON.stringify(row.protocol)}, expected ${PROTOCOL}`,
        );
      }
      for (const field of ["corpus_id", "fixture_id", "operation"] as const) {
        if (typeof row[field] !== "string" || row[field] === "") {
          throw new AdapterError(
            "contract-conformance",
            `line ${index + 1} has no ${field}`,
          );
        }
      }
      // `Map`, not an object literal — see the same guard in
      // differential-report.ts. An object literal resolves inherited property
      // names like `valueOf`, so `"status": "valueOf"` would have been read as
      // a declared status.
      const outcome = STATUS.get(row.status as string);
      if (outcome === undefined) {
        throw new AdapterError(
          "contract-conformance",
          `line ${index + 1} has unknown status ${JSON.stringify(row.status)}`,
        );
      }
      let traceIds: string[] | undefined;
      if (row.trace_ids !== undefined) {
        if (
          !Array.isArray(row.trace_ids) ||
          row.trace_ids.length === 0 ||
          row.trace_ids.some(
            (id: unknown) => typeof id !== "string" || id.trim() === "",
          ) ||
          new Set(row.trace_ids).size !== row.trace_ids.length
        ) {
          throw new AdapterError(
            "contract-conformance",
            `line ${index + 1} trace_ids must be a non-empty array of distinct non-blank strings`,
          );
        }
        // Bindings belong to recordRun: preserve producer values and ordering.
        // Legacy producers may omit trace_ids; that must not invent a binding.
        traceIds = row.trace_ids as string[];
      }
      entries.push({
        // Corpus and operation are part of the identity: the same fixture id
        // is replayed under several operations, and collapsing them would make
        // one result overwrite another.
        symbol: `${row.corpus_id as string}::${row.operation as string}::${row.fixture_id as string}`,
        outcome,
        ...(traceIds === undefined ? {} : { traceIds }),
      });
    });

    return { entries };
  },
};
