/**
 * The evidence store's machine-written records (FR-030).
 *
 * ## Governing principle
 *
 * **Store only what cannot be recomputed from spec + code at HEAD.**
 *
 * Run outcomes, hash-at-binding, affirmations and the ratchet baseline are
 * facts about the past — nothing can re-derive them tomorrow. Obligations are
 * not: they are re-derived live from quire (quire-rs FR-053) on every read, so
 * they cannot drift out of agreement with the spec. A stored copy of an
 * obligation would be one more thing to keep in sync, and the thing it would
 * most likely disagree with is the requirement itself.
 *
 * ## Format rule
 *
 * Authored → markdown, validated as a corpus document (the `SuiteRegistry` and
 * `Inspections` archetypes, spec-artifacts-process FR-006).
 * Machine-transcribed → JSON, typeless and corpus-invisible, described here.
 *
 * No SQLite anywhere. quoin writes files; parsing, loading and indexing for an
 * application is `filament-ide-rs` daemon territory, and a derived index inside
 * the store would be a second source of truth for facts the files already hold.
 */

/** Where a suite's run records live, relative to the store root. */
export const RUNS_DIR = "runs";

/** Schema version carried by every machine-written file in the store. */
export const STORE_SCHEMA_VERSION = 1;

/**
 * One test symbol's outcome within a run.
 *
 * `symbol` is the FR-051 stable symbol identity — the join between what a tool
 * reports and what the matrix declares. A tool's own test name is not that
 * identity and must be mapped to it by the adapter (agent-ix/quoin#91) before a
 * record is written.
 */
export interface RunEntry {
  /** FR-051 stable symbol identity. */
  symbol: string;
  outcome: "pass" | "fail" | "skip" | "error";
  /**
   * A numeric result where the method produces one — a mutation score, a
   * coverage percentage, a measured latency. Absent for a plain pass/fail.
   */
  score?: number;
  /** Trace ids this symbol carries, as the adapter read them. */
  traceIds?: string[];
}

/**
 * One run of ONE suite at ONE commit.
 *
 * The suite is the atomic unit of evidence: aggregation is a view, and a
 * partial run must never be able to masquerade as a full one. The `make ci` /
 * `make ci-python` split already caused exactly that rot (quire-rs TC-715),
 * where a green "CI" meant one of two suites.
 *
 * Re-runs at the same commit are **last-write-wins, latest only**. Flake
 * history is analytics, not the record of record — keeping every attempt here
 * would make the store grow without bound to answer a question a different
 * tool should answer.
 */
export interface RunRecord {
  schemaVersion: number;
  suite: string;
  /** Full commit sha the run was performed at. */
  commit: string;
  /** Tool and version, as the adapter reported them. */
  tool: string;
  /**
   * The kind of evidence this run produced, from the declared `test_type`
   * vocabulary (`Unit`, `Property`, `Static`, `Manual`, …) — the same
   * vocabulary the suite registry's `Evidence Kind` column and the catalog's
   * `evidence_kind` use.
   *
   * Optional, and its absence is **not** an invitation to guess. Method
   * conformance used to infer "this was a test run" from `entries.length > 0`,
   * which is true of a transcribed inspection too — so every `Inspection` or
   * `Analysis` obligation recorded through `quoin evidence record` was flagged
   * (agent-ix/quoin#105). With no kind declared the question cannot be asked,
   * and the check says nothing rather than something wrong.
   */
  evidenceKind?: string;
  /** ISO-8601, supplied by the caller — never read from the clock here. */
  timestamp: string;
  entries: RunEntry[];
}

/** Someone re-affirming a binding after the statement it was made against changed. */
export interface Affirmation {
  who: string;
  commit: string;
  note?: string;
}

/**
 * One obligation bound to the evidence that discharges it.
 *
 * `statementHashAtBinding` is the entire mechanism: suspect detection is
 * `currentHash !== statementHashAtBinding`, computed by comparing this against
 * the obligation quire re-derives today. quire computes the hash; quoin only
 * ever compares.
 */
export interface Binding {
  /** Obligation id — the same id the coverage rollup and trace tags key on. */
  obligation: string;
  /** The statement hash as it stood when the binding was first made. */
  statementHashAtBinding: string;
  /** Suite that discharged it. */
  suite: string;
  /** Commit of the run that first discharged it. */
  commit: string;
  /** Symbols within that suite that carry the obligation's trace id. */
  symbols: string[];
  /** Re-affirmations recorded after a statement changed. */
  affirmations?: Affirmation[];
}

/**
 * The unified binding graph.
 *
 * Unified rather than per-suite because the graph IS cross-suite: one
 * obligation can be discharged by a unit test and a benchmark, and splitting
 * the file per suite would make that relationship something to reassemble.
 */
export interface BindingsFile {
  schemaVersion: number;
  bindings: Binding[];
}

/**
 * The accepted violation set a ratchet compares against.
 *
 * `accepted` holds `<kind>:<obligation>` keys — **every** finding kind, not two
 * named buckets. The original shape carried `undischarged` and `suspect` only,
 * so `stale-evidence`, `vacuous-evidence`, `method-conformance`,
 * `unknown-method` and `insufficient-multiplicity` could never appear in a
 * baseline and `--ratchet` reported the entire existing backlog for all five —
 * the outcome ratchet mode exists to prevent (agent-ix/quoin#105).
 */
export interface BaselineFile {
  schemaVersion: number;
  /** Commit the baseline was accepted at. */
  commit: string;
  /** Accepted findings as `<kind>:<obligation>`, sorted. */
  accepted: string[];
}
