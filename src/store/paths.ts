/**
 * Where retained bytes live (FR-030).
 *
 * The root is owned by neither the evidence store nor change assurance: both
 * write families beneath it, and while it lived in `evidence/store.ts` the
 * change-assurance store had to import upward into evidence to place its own
 * directory — one half of the import cycle Cargo forbids (agent-ix/quoin#376).
 */

import { join } from "node:path";

/**
 * The store root for a repository.
 *
 * Under `spec/`, not at the repository root. quire-rs CR-045 bounds the
 * document walk to `<scope>/spec`, so the authored half of the store —
 * `suites.md` and `inspections.md` — is only a validated corpus document if it
 * lives there. The machine-written half sits beside it so the whole store is
 * one directory rather than two halves in different places.
 *
 * agent-ix/quoin#79's original layout put `evidence/` at the repository root on
 * the premise that quire "validates them wherever they live". Measured: it does
 * not — a typed registry at the root minted nothing and was reported nowhere.
 */
export function storeRoot(repo: string): string {
  return join(repo, "spec", "evidence");
}
