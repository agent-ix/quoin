/**
 * TypeScript models for the quire JSON payloads (FR-029).
 *
 * These mirror the vendored schemas rather than restating them: every value
 * that reaches consumer code has been validated against the published schema
 * first (`validateCoverage` / `validateProperties`), so a mismatch between
 * these declarations and the contract surfaces as a validation error naming the
 * offending path — not as an `undefined` three frames later.
 *
 * That ordering is the point. quoin's own `spec/review.md` Finding 8 records
 * "no contract test against quire": the shapes lived as prose in skill
 * markdown, and when one drifted an agent failed mid-skill with nothing to
 * diagnose. Types alone would not have fixed that — TypeScript erases at
 * runtime and JSON from a subprocess is `any`. The validator is what makes
 * these types true.
 */

/** One reference row answerable for trace ids nothing backs (FR-050-AC-3). */
export interface UnbackedRow {
  reference: string;
  document: string;
  row_id?: string | null;
  target_ids: string[];
  /**
   * 1-based document line of the matrix row (quire-rs #210, v0.42.0). Absent
   * on a payload from an engine predating the field.
   */
  line?: number;
}

/** A row whose status classes as complete while nothing backs it. */
export interface StatusLie {
  reference: string;
  document: string;
  row_id?: string | null;
  status: string;
  target_ids: string[];
  /** 1-based document line of the matrix row (quire-rs #210, v0.42.0). */
  line?: number;
}

/** An unbacked row exempted by a method that mints no source symbol (CR-041). */
export interface NoSymbolRow {
  reference: string;
  document: string;
  row_id?: string | null;
  test_type: string;
  target_ids: string[];
  /** 1-based document line of the matrix row (quire-rs #210, v0.42.0). */
  line?: number;
}

/**
 * A row whose authored status the module's vocabulary classes as nothing
 * (FR-050-AC-21, quire-rs CR-083).
 *
 * Carries no class, deliberately: having none is the entire finding, and two
 * undeclared glyphs in one corpus are told apart only by the authored value.
 */
export interface UndeclaredStatus {
  reference: string;
  document: string;
  row_id?: string | null;
  /** The authored status value, verbatim. */
  status: string;
  /**
   * 1-based document line of the matrix row (quire-rs #210, v0.42.0).
   * Byte-identical duplicate rows still collapse to one record (quire-rs
   * CR-086), which carries the first duplicate's line.
   */
  line?: number;
}

/**
 * One `implements` edge — requirement to production code (FR-062).
 *
 * **Carries no weight in `totals`.** Scope is not evidence: folding it into the
 * backed set would let code that merely cites a requirement move a coverage
 * number, which is the backdoor quire-rs CR-061 closed.
 */
export interface ImplementsRecord {
  path: string;
  symbol: string;
  trace_id: string;
  /** Name of the declared marker form that bound it. */
  form: string;
}

/** A source symbol whose trace tag resolves to nothing declared. */
export interface UntrackedSymbol {
  path: string;
  symbol: string;
  trace_id: string;
  /** 1-based declaration line of the tagged symbol (quire-rs #210, v0.42.0). */
  line?: number;
}

/** One of the distinct symbols binding a shared trace id (quire-rs CR-087). */
export interface SharedTraceSymbol {
  path: string;
  symbol: string;
}

/**
 * A trace id bound by more than one distinct source symbol (quire-rs
 * FR-050-AC-23, CR-087).
 *
 * One id names one symbol; a row backed by N symbols stays green while N-1 of
 * its tests rot. Carries no weight in `totals`, and `--strict` does not gate
 * on it in this contract revision.
 */
export interface SharedTraceId {
  /** The trace id, exactly as the binding forms yield it. */
  trace_id: string;
  /** The distinct binding symbols, ordered by (path, symbol); at least two. */
  symbols: SharedTraceSymbol[];
}

/** Backed/total counts for one minting document and target kind. */
export interface GroupCounts {
  document: string;
  target: string;
  backed: number;
  total: number;
}

/** Per-document property-shape counts (CR-028). */
export interface CriteriaCounts {
  document: string;
  archetype: string;
  criteria: number;
  property_shaped: number;
  by_property: Record<string, number>;
}

/** A declaration that selected nothing, and why (CR-054). */
export interface CoverageDiagnostic {
  declaration: string;
  /** Open machine vocabulary — deliberately not a union (FR-055). */
  reason: string;
  message: string;
  path?: string | null;
  /**
   * The vocabulary or catalog value the diagnostic is about, verbatim, when it
   * is about exactly one (quire-rs FR-054-AC-12, CR-091).
   *
   * `uncatalogued-verification-method` carries the authored method here —
   * byte-equal to the `Obligation` records' `method` — so the advisor's
   * mismatch / uncatalogued split is an equality join rather than a regex over
   * `message` prose (agent-ix/quoin#168). Absent on a payload from an engine
   * predating the classification, and absent when the diagnostic is not about
   * one value; a consumer must tolerate both.
   */
  value?: string;
}

/**
 * One declared coverage-vocabulary value, classified (quire-rs FR-059-AC-9,
 * CR-091).
 *
 * Carried ahead of the pinned `sourceTag` as an additive field (see
 * `QUIRE_CONTRACT`): a payload from quire 0.27.0 simply omits the whole
 * `vocabulary_coverage` array, and every consumer must treat absence as "the
 * engine predates the classification", never as "every value is owned".
 */
export interface VocabularyValueRecord {
  /** The `vocabulary_coverage` declaration's name. */
  vocabulary: string;
  /** The projected archetype (the declaration's `from`). */
  archetype: string;
  /** The frontmatter field whose schema `enum` is the vocabulary. */
  field: string;
  /** The declared `<check>` severity token. */
  check: string;
  /** The vocabulary value, verbatim from the schema enum. */
  value: string;
  /** Closed deliberately, unlike `CoverageDiagnostic.reason`. */
  state: "owned" | "excused" | "unowned";
  /** Scope-relative documents that decide the state; empty for `unowned`. */
  documents: string[];
}

/**
 * One derived obligation (quire-rs FR-053) — the quire↔quoin contract itself.
 *
 * `statement_hash` is the whole reason this record exists downstream: suspect
 * detection is `current_hash !== hash_at_binding` and nothing more.
 */
export interface Obligation {
  source: string;
  id: string;
  document: string;
  statement: string;
  statement_hash: string;
  method?: string | null;
  parameters?: Record<string, string>;
  criticality?: string | null;
  /**
   * Test-case ids the criterion's method cell names — `Test (TC-707)` yields
   * `["TC-707"]` (quire-rs FR-053-AC-11, v0.35.0).
   *
   * Optional because an engine before v0.35.0 emits none, and because a cell
   * naming no test case carries none. Treat absent and empty alike: both mean
   * "this obligation names no test case", never "the engine is old" — a
   * consumer that branched on the difference would report a version as a
   * finding.
   */
  target_ids?: string[];
}

/** Bundle-wide totals. The criteria pair is all-or-nothing. */
export interface CoverageTotals {
  backed: number;
  total: number;
  criteria?: number | null;
  property_shaped?: number | null;
}

/** The `quire coverage --json` payload (v1). */
export interface CoverageReport {
  unbacked_rows: UnbackedRow[];
  status_lies: StatusLie[];
  /** Absent — not empty — when the module declares no exemption vocabulary. */
  no_symbol_rows?: NoSymbolRow[];
  /** Absent — not empty — when every status value in the corpus is declared. */
  undeclared_statuses?: UndeclaredStatus[];
  untracked_symbols: UntrackedSymbol[];
  /**
   * Trace ids bound by more than one distinct symbol (quire-rs FR-050-AC-23,
   * CR-087). ABSENT — not empty — for a corpus whose every id is uniquely
   * bound.
   */
  shared_trace_ids?: SharedTraceId[];
  groups: GroupCounts[];
  criteria?: CriteriaCounts[];
  diagnostics?: CoverageDiagnostic[];
  obligations?: Obligation[];
  /**
   * Absent when the module declares no `implements` marker forms. Present in
   * the published schema since CR-080 and missing from this interface until
   * CR-083 — the vendored contract and its types had drifted in the one
   * direction the contract test does not check.
   */
  implements?: ImplementsRecord[];
  /**
   * One record per declared coverage-vocabulary value, classified
   * owned / excused / unowned (quire-rs FR-059-AC-9, CR-091). ABSENT — not
   * empty — for a module declaring no `vocabulary_coverage`, and absent on any
   * payload from an engine predating the classification.
   */
  vocabulary_coverage?: VocabularyValueRecord[];
  /**
   * Source files a declared `source_exclude` glob removed from the symbol walk
   * (quire-rs FR-050-AC-24, #215). ABSENT — never 0 — for a model declaring no
   * `source_exclude` or one whose globs match nothing. Without it an
   * over-broad glob silently drops legitimate backing and the report reads as
   * a coverage regression.
   */
  excluded_source_files?: number;
  totals: CoverageTotals;
}

/** A byte-offset span carrying its own text (FR-052). */
export interface PropertySpan {
  start: number;
  end: number;
  text: string;
}

/** The FR-047 shape axis. Closed in the engine, so closed here. */
export type AcShape =
  "assertion" | "obligation" | "given-when-then" | "unstructured";

/** The FR-052 property-shape axis. Closed by FR-052-CON-3. */
export type PropertyShape =
  | "round-trip"
  | "idempotence"
  | "ordering"
  | "invariant"
  | "error-case"
  | "lifecycle"
  | "concurrency"
  | "universal"
  | "example"
  | "unclassified";

/** What a generator may do with a criterion (CR-033). */
export type Extraction = "extractable" | "candidate" | "not-extractable";

/**
 * The obligation as nested on a criterion record.
 *
 * Carries no `id`, `statement` or `document`: the criterion and its enclosing
 * document object already have all three.
 */
export interface CriterionObligation {
  source: string;
  statement_hash: string;
  method?: string | null;
  parameters?: Record<string, string>;
  criticality?: string | null;
}

/** One classified acceptance criterion. */
export interface Criterion {
  row_id: string | null;
  statement: string;
  line: number | null;
  shape: AcShape;
  property: PropertyShape;
  extractable: boolean;
  extraction: Extraction;
  domain: PropertySpan | null;
  precondition: PropertySpan | null;
  oracle: PropertySpan | null;
  signals: string[];
  obligation: CriterionObligation | null;
}

/** One document's classified criteria. */
export interface PropertiesDocument {
  document: string;
  archetype: string;
  criteria: Criterion[];
}

/** The `quire properties --json` payload (v1). */
export interface PropertiesReport {
  documents: PropertiesDocument[];
}
