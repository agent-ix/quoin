/**
 * The assurance domain, asked of `quoin-core` (quoin#447, FR-040, FR-046,
 * FR-047, FR-096).
 *
 * What is left of `src/assurance/` on the TypeScript side: six calls and
 * nothing else. The case graph, the authored-argument view, the clause
 * discharge partition and all three renderers live in
 * `rust/crates/quoin-assurance/`.
 *
 * **Rendered markdown arrives as a JSON field, not as stdout.** The boundary's
 * rule is that stdout carries a canonical JSON payload and nothing else, so
 * `assurance.render_*` answers `{ rendered }` and the command writes the field.
 * That is the one shape difference from the retained functions, and it belongs
 * to the protocol rather than to this domain.
 *
 * # Why the case is `unknown` here and the authored view is not
 *
 * `AuthoredArgumentView`, `DischargeReport`, `BuildAuthoredArgumentRequest` and
 * `BuildDischargeRequest` are GENERATED (FR-097) and imported from
 * {@link ./types.js}; nothing below re-declares them.
 *
 * The assurance case is deliberately not, and that is a decision rather than an
 * omission. `render_case` reads `AssuranceCase<TrustAssessment,
 * IndependenceAssessment>`, and those two Rust types are, by their own crate
 * header, READERS: they declare 5 of 8 and 6 of 7 fields, only the ones the
 * renderer puts on the page, because a field nothing observes cannot be covered
 * by a byte comparison. Publishing them as the boundary's TypeScript would
 * state a lossy shape as the wire contract, and `src/evidence/types.ts` — which
 * owns the full shapes and still exists — would then have two answers to what a
 * `TrustAssessment` is.
 *
 * So the case crosses as an opaque value. `quoin assurance` never reads into
 * it: it receives it from `build_case`, hands it to `render_case` or to
 * `JSON.stringify`, and inspects no field. `unknown` states that exactly, and
 * unlike a hand-written interface it cannot drift, because there is nothing in
 * it to drift.
 */

import { CORE_EXIT, carriesPayload, runCoreAllowFailure } from "./exec.js";
import type {
  AuthoredArgumentView,
  BuildAuthoredArgumentRequest,
  BuildDischargeRequest,
  DischargeReport,
  RenderAuthoredArgumentPayload,
  RenderDischargePayload,
} from "./types.js";

/**
 * Run an operation whose refusals are the RETAINED CONTRACT'S, not the
 * transport's, and surface them as the retained function surfaced them.
 *
 * `buildAuthoredArgumentView` and `buildDischargeReport` threw an `Error`
 * carrying the contract's own sentence — "assurance argument has no id",
 * "duplicate discharge fact for clause X" — and `src/commands/*.ts` printed
 * that sentence through `this.error(detail, { exit: 2 })`. `runCore` would
 * wrap the same sentence in `quoin-core <op> exited 3 (CORE_BAD_REQUEST): …`,
 * which is a different line on an operator's terminal for an input that has
 * not changed. The cutover moves where the check runs; it does not get to
 * rewrite what a user reads when the check fails.
 *
 * Only `CORE_BAD_REQUEST` is unwrapped, and only its message. Every other
 * outcome — a refusal, a protocol skew, a boundary that failed — keeps
 * `runCore`'s framing, because those are facts about the boundary and the
 * operator needs to see that the boundary is what spoke.
 */
function askContract(op: string, request: unknown): unknown {
  const result = runCoreAllowFailure(op, request);
  if (carriesPayload(result.exitCode)) return result.payload;
  if (result.exitCode === CORE_EXIT.INVALID) {
    const contract = result.diagnostics.find(
      (d) => d.code === "CORE_BAD_REQUEST",
    );
    if (contract) throw new Error(contract.message);
  }
  // Every other outcome keeps the boundary's own framing. The subprocess is
  // NOT re-run to produce it: a second run of a refused request is a second
  // chance for the two runs to differ, which is how a diagnostic starts
  // describing an invocation nobody saw.
  const codes = result.diagnostics.map((d) => d.code).join(", ");
  const detail = result.diagnostics
    .map((d) => `${d.code}: ${d.message}`)
    .join("\n");
  throw new Error(
    `quoin-core ${op} exited ${result.exitCode}` +
      (codes ? ` (${codes}):\n${detail}` : " with no diagnostic on stderr."),
  );
}

/**
 * What {@link buildCase} is given, in the wire spelling.
 *
 * snake_case because `assurance.build_case` takes `CaseInput`'s own field
 * names, and `deny_unknown_fields` on the far side means a camelCase key is a
 * bad request rather than a silently ignored one. Every field is `unknown[]`
 * for the reason the module header gives: this layer forwards what the auditor,
 * the evidence store and `quire coverage` produced and reads none of it.
 */
export interface CaseRequest {
  /** Frontmatter of every document in the bundle. */
  documents: unknown[];
  /** Obligations, from `quire coverage --json`. */
  obligations: unknown[];
  /** The auditor's findings. */
  findings: unknown[];
  /** Artifact types that are top-level claims; defaults to `StR`. */
  claim_types?: string[];
  /** Documents whose frontmatter could not be read. */
  unreadable?: unknown[];
  /** Producer trust, carried through as context. */
  producer_trust?: unknown[];
  /** Profile-selected separation results, carried through as context. */
  evidence_independence?: unknown[];
}

/** Build the assurance case (FR-040). */
export function buildCase(request: CaseRequest): unknown {
  return askContract("assurance.build_case", request);
}

/** Render a built case as markdown (FR-040). */
export function renderCase(assurance: unknown): string {
  return (
    askContract("assurance.render_case", assurance) as { rendered: string }
  ).rendered;
}

/** Build the authored-argument view (FR-047). */
export function buildAuthoredArgumentView(
  request: BuildAuthoredArgumentRequest,
): AuthoredArgumentView {
  return askContract(
    "assurance.build_authored_argument",
    request,
  ) as AuthoredArgumentView;
}

/** Render an authored-argument view as markdown (FR-047). */
export function renderAuthoredArgument(view: AuthoredArgumentView): string {
  return (
    askContract(
      "assurance.render_authored_argument",
      view,
    ) as RenderAuthoredArgumentPayload
  ).rendered;
}

/** Partition binding clauses into evidence, dispositions and open work. */
export function buildDischargeReport(
  request: BuildDischargeRequest,
): DischargeReport {
  return askContract("assurance.build_discharge", request) as DischargeReport;
}

/** Render a discharge report as markdown (FR-046). */
export function renderDischargeReport(report: DischargeReport): string {
  return (
    askContract("assurance.render_discharge", report) as RenderDischargePayload
  ).rendered;
}
