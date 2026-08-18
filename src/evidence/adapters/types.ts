import type { RunEntry } from "../types.js";

/**
 * One tool output format, transcribed into run entries.
 *
 * The contract is deliberately narrow, and the narrowness IS the contract
 * (ADR-0011 invariant 1 — quoin transcribes; the consumer's CI executes):
 *
 * - **Pure.** `parse` receives the raw text and returns entries. It reads no
 *   file beyond that text, spawns no process, and reaches no network. An
 *   adapter that could run the tool would make `quoin evidence record` a test
 *   runner, and a transcript nobody can trust.
 * - **No verdict.** An adapter reports what the tool said. Deciding whether a
 *   result is acceptable belongs to the auditor and the consumer's gate
 *   policy, never here — see {@link AdapterResult.evidenceKind} and the note on
 *   coverage in `AdapterRegistry`.
 */
export interface EvidenceAdapter {
  /** Registry key, and the value `--adapter` takes. */
  readonly name: string;
  /** One line for `--help`; what format this reads. */
  readonly summary: string;
  /**
   * Tool identifiers this adapter claims, matched case-insensitively against
   * the suite's declared `tool` as a substring. Empty means "never selected
   * automatically" — an adapter that only runs when named explicitly.
   */
  readonly tools: readonly string[];
  parse(raw: string): AdapterResult;
}

/** What an adapter produces. */
export interface AdapterResult {
  entries: RunEntry[];
  /**
   * The evidence kind this format proves, when the FORMAT ITSELF determines
   * it — and usually it does not.
   *
   * A JUnit XML file is emitted by unit, integration and end-to-end suites
   * alike, so an adapter answering "Unit" would be asserting something the
   * format does not contain. The kind is declared by the suite registry
   * (`Evidence Kind`) and passed with `--kind`; that is where the vocabulary
   * lives, and where it stays.
   *
   * The field exists because an external adapter for a single-purpose tool may
   * legitimately know. The adapters shipped here leave it unset rather than
   * mint a fourth copy of a vocabulary that already exists in three places
   * (agent-ix/quoin#114).
   */
  evidenceKind?: string;
}

/** Raised when an adapter cannot read its input. */
export class AdapterError extends Error {
  constructor(
    readonly adapter: string,
    message: string,
  ) {
    super(`${adapter}: ${message}`);
    this.name = "AdapterError";
  }
}
