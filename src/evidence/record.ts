/**
 * Transcribing a suite run into the store (FR-030).
 *
 * quoin **transcribes**; the consumer's CI executes (ADR-0011 invariant 1).
 * Nothing here runs a test, and nothing here decides whether a result is
 * acceptable — recording a failing run is as legitimate as recording a passing
 * one, and more useful, because a suite that stopped passing is exactly what a
 * freshness check needs to see.
 */

import type { CoverageReport, Obligation } from "../quire/index.js";
import { bind, readBindings, writeBindings, writeRun } from "./store.js";
import { STORE_SCHEMA_VERSION, type Binding, type RunEntry } from "./types.js";

/** What a caller supplies to record one run. */
export interface RecordRequest {
  repo: string;
  suite: string;
  commit: string;
  tool: string;
  /**
   * The kind of evidence this run produced, from the declared `test_type`
   * vocabulary. Optional; method conformance compares kind to kind and stays
   * silent when a run declares none, rather than inferring one.
   */
  evidenceKind?: string;
  /** ISO-8601. Passed in rather than read from the clock, so a record is reproducible. */
  timestamp: string;
  entries: RunEntry[];
  /**
   * The obligations as quire derives them **today**.
   *
   * Read live rather than stored: an obligation is always re-derivable from
   * spec + code at HEAD, so a stored copy could only ever disagree with the
   * requirement it describes.
   */
  obligations: Obligation[];
}

/** What recording a run changed. */
export interface RecordOutcome {
  runPath: string;
  /** Obligations bound for the first time by this run. */
  bound: string[];
  /**
   * Obligations already bound whose statement has changed since — the suspect
   * set. Reported, never auto-cleared: re-running a test does not re-affirm a
   * reworded requirement.
   */
  suspect: string[];
  /** Trace ids in the run that match no derived obligation. */
  unmatched: string[];
}

/**
 * Record one suite run and update the binding graph.
 *
 * The join is: run entry → trace ids it carries → obligation of the same id.
 * A trace id matching no obligation is reported rather than dropped, because
 * that is the quire-rs#72 class arriving from the other direction — a test
 * claiming to verify something the spec does not state.
 */
export function recordRun(request: RecordRequest): RecordOutcome {
  const runPath = writeRun(request.repo, {
    schemaVersion: STORE_SCHEMA_VERSION,
    suite: request.suite,
    commit: request.commit,
    tool: request.tool,
    evidenceKind: request.evidenceKind,
    timestamp: request.timestamp,
    entries: request.entries,
  });

  const byId = new Map(request.obligations.map((o) => [o.id, o]));
  /** Obligation id → the symbols in this run that carry it. */
  const discharged = new Map<string, string[]>();
  const unmatched = new Set<string>();

  for (const entry of request.entries) {
    // Only a passing symbol discharges. A failing or skipped test is evidence
    // that the suite ran, which the run record already holds — it is not
    // evidence that the obligation holds, and binding on it would make the
    // store agree with a red build.
    for (const id of entry.traceIds ?? []) {
      if (!byId.has(id)) {
        unmatched.add(id);
        continue;
      }
      if (entry.outcome !== "pass") continue;
      const symbols = discharged.get(id) ?? [];
      symbols.push(entry.symbol);
      discharged.set(id, symbols);
    }
  }

  let bindings: Binding[] = readBindings(request.repo).bindings;
  const bound: string[] = [];
  const suspect: string[] = [];

  // Plain comparison, not `localeCompare`: the outcome lists are reported and
  // the store is written from this order, and `localeCompare` without an
  // explicit locale depends on the runtime's ICU data (agent-ix/quoin#106).
  for (const [id, symbols] of [...discharged].sort(([a], [b]) =>
    a === b ? 0 : a < b ? -1 : 1,
  )) {
    const obligation = byId.get(id);
    if (!obligation) continue;
    const result = bind(bindings, {
      obligation: id,
      statementHashAtBinding: obligation.statement_hash,
      suite: request.suite,
      commit: request.commit,
      symbols: [...symbols].sort(),
    });
    bindings = result.bindings;
    if (result.created) bound.push(id);
    if (result.suspect) suspect.push(id);
  }

  writeBindings(request.repo, {
    schemaVersion: STORE_SCHEMA_VERSION,
    bindings,
  });

  return {
    runPath,
    bound: bound.sort(),
    suspect: suspect.sort(),
    unmatched: [...unmatched].sort(),
  };
}

/**
 * The obligations carried by a validated coverage payload.
 *
 * A payload from a model declaring no obligation sources carries none, which is
 * an empty list rather than an error: a repository that has not adopted
 * obligations can still record runs, it just binds nothing.
 */
export function obligationsFrom(report: CoverageReport): Obligation[] {
  return report.obligations ?? [];
}
