/**
 * The `auditor` domain at the boundary (quoin#501, FR-096).
 *
 * Replaces `src/auditor/` and `src/advisor/` with four calls into
 * `quoin-core`. There are four and not six because the unit of IPC is a
 * command-shaped operation, never a function: `ratchet`, `findingKey` and
 * `scoresFor` were helpers *inside* a command's work, so they ride inside the
 * operation that needed them rather than each becoming a round trip.
 *
 * `loadMethodCatalog` and `methodClasses` are deliberately absent. They read
 * installed module manifests, which is the host's job and not the engine's, so
 * they stay in `src/method-catalog.ts` and callers import them from there.
 *
 * The domain is granted **no capability**: `quoin-auditor` runs no process,
 * opens no path and reads no environment, so every input it needs arrives on
 * stdin. That is what makes one request per command affordable — the caller
 * has already read the store for its own reasons.
 */

import { carriesPayload, runCoreAllowFailure } from "./exec.js";
import type {
  AdvisePayload,
  AdviseRequest,
  AuditInput,
  AuditPayload,
  BaselinePayload,
  VocabularyPayload,
} from "./types.js";

export type {
  Advice,
  AdvisePayload,
  AdviseRequest,
  AuditFinding,
  AuditInput,
  AuditPayload,
  AuditReport,
  CoverageDiagnostic,
  PropertyShape,
  Recommendation,
  UnevaluatedCheck,
} from "./types.js";

/** One boundary call, with the failure surfaced rather than swallowed. */
function call<T>(op: string, request: unknown): T {
  const result = runCoreAllowFailure(op, request);
  if (!carriesPayload(result.exitCode)) {
    const detail = result.diagnostics
      .map((d) => `${d.code}: ${d.message}`)
      .join("\n");
    throw new Error(
      detail ||
        `quoin-core ${op} exited ${result.exitCode} with no diagnostic.`,
    );
  }
  return result.payload as T;
}

/**
 * Audit the store, optionally against an accepted baseline.
 *
 * `accepted` absent is not `accepted: []`. Absent means no baseline was read
 * and {@link AuditPayload.reported} comes back null, so the caller reports the
 * whole backlog and says so; an empty array means a baseline was read and
 * accepted nothing, so every finding is genuinely new (agent-ix/quoin#169).
 */
export function audit(
  input: AuditInput,
  accepted?: readonly string[],
): AuditPayload {
  return call<AuditPayload>("auditor.audit", {
    input,
    ...(accepted === undefined ? {} : { accepted: [...accepted] }),
  });
}

/**
 * The finding keys `quoin evidence baseline` accepts, sorted.
 *
 * The same audit as {@link audit}, asked a different question: the keying and
 * the sort are the engine's, so the baseline a run writes and the baseline a
 * later run ratchets against cannot disagree about spelling.
 */
export function baselineKeys(input: AuditInput): string[] {
  return call<BaselinePayload>("auditor.baseline", { input }).accepted;
}

/**
 * Advise every obligation, in the order they arrived.
 *
 * One request for the whole population rather than one per obligation: the
 * advisor is pure over what the caller already read, so per-obligation calls
 * would have bought nothing but N spawns.
 */
export function advise(request: AdviseRequest): AdvisePayload {
  return call<AdvisePayload>("auditor.advise", request);
}

/**
 * Every characteristic value the advisor's fact set can ever mint, sorted.
 *
 * Not a function over a request — the boundary stating a constant of itself,
 * the same shape as `evidence.store_facts`. The catalog declares the values
 * that trigger each method; this is the set that can ever match them, and
 * nothing compared the two until 7 of 33 methods turned out to be unreachable
 * by any statement ever written (agent-ix/quoin#128).
 */
export function vocabulary(): string[] {
  return call<VocabularyPayload>("auditor.vocabulary", {})
    .mintableCharacteristics;
}
