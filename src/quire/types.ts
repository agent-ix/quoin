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
}

/** A row whose status classes as complete while nothing backs it. */
export interface StatusLie {
  reference: string;
  document: string;
  row_id?: string | null;
  status: string;
  target_ids: string[];
}

/** An unbacked row exempted by a method that mints no source symbol (CR-041). */
export interface NoSymbolRow {
  reference: string;
  document: string;
  row_id?: string | null;
  test_type: string;
  target_ids: string[];
}

/** A source symbol whose trace tag resolves to nothing declared. */
export interface UntrackedSymbol {
  path: string;
  symbol: string;
  trace_id: string;
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
  untracked_symbols: UntrackedSymbol[];
  groups: GroupCounts[];
  criteria?: CriteriaCounts[];
  diagnostics?: CoverageDiagnostic[];
  obligations?: Obligation[];
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
