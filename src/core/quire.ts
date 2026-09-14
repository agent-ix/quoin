/**
 * The quire domain, asked of `quoin-core` (quoin#502, FR-052, FR-099, FR-096).
 *
 * This file is what is left of `src/quire/` on the TypeScript side. The engine
 * is no longer a subprocess quoin spawns and version-checks: `quoin-quire` is
 * LINKED into `quoin-core`, so the vendored schemas, the contract hashes and
 * the version premise that guarded a separately released `quire` binary all
 * dissolve into a Cargo edge. One process boundary replaces two.
 *
 * That is why there is no `checkVersionPremise` here and no `quireVersion`.
 * The premise existed because the caller could not know which quire it got;
 * the answer now ships in the same binary as the question.
 *
 * Both operations take the repository root and an optional CLOSED module set.
 * An empty list is not an empty set — it is ambient discovery, the resolution
 * `quire coverage` performed with no `--module` (quire-rs#405).
 */

import { runCore } from "./exec.js";
import type { CoveragePayload, PropertiesPayload } from "./types.js";

/**
 * The obligations a repository states, and the diagnostics deriving them
 * raised.
 *
 * Diagnostics ride WITH the obligations rather than being read from a second
 * call: `quoin advise` joins them by obligation id, and two calls could answer
 * about two different walks of the tree.
 */
export function coverage(scope: string, modules?: string[]): CoveragePayload {
  return runCore("quire.coverage", {
    scope,
    ...(modules?.length ? { modules } : {}),
  }) as CoveragePayload;
}

/**
 * The FR-052 property shape and archetype of each criterion, keyed by row id.
 *
 * **Deliberately scoped, never module-selected.** A module selection names the
 * one module supplying the traceability model, while classification needs the
 * archetype the document's own `type:` declares — usually a different module.
 * Selecting here resolved no `FR` archetype at all and yielded zero criteria,
 * silently.
 *
 * `unresolved` is a partial answer stated out loud. The retained path learned
 * the same thing through an exit status it had to swallow: a single asset with
 * no `type:` must not cost the whole shape axis. Here the classified documents
 * and the skipped ones are one payload.
 */
export function propertyShapes(scope: string): PropertiesPayload {
  return runCore("quire.properties", { scope }) as PropertiesPayload;
}
