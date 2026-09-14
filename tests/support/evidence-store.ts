/**
 * Evidence-store fixtures, for tests that exercise something else.
 *
 * `src/evidence/` is gone (quoin#458): the store is `quoin-evidence`, reached
 * through `quoin-core`. The commands under test read it through that boundary,
 * and these writers put bytes on disk for them to read — a fixture, never a
 * second implementation. Nothing here is asserted against; the assertion is
 * always what the command did with what it found.
 *
 * The one shape rule they carry is the one the bytes themselves carry: records
 * are canonical JSON stamped with `STORE_SCHEMA_VERSION`, and bindings are
 * sorted by `(obligation, suite)` as the reader expects them. A fixture that
 * drifted from the layout would fail the test that uses it, which is the point.
 */

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import {
  STORE_SCHEMA_VERSION,
  bindingsPath,
  baselinePath,
  storeRoot,
  type BaselineFile,
  type Binding,
  type RunRecord,
} from "../../src/core/evidence.js";
import { canonicalJson } from "../../src/canonical-output.js";

export { STORE_SCHEMA_VERSION, bindingsPath, baselinePath, storeRoot };
export type { BaselineFile, Binding, RunRecord };

/**
 * Write one canonical document, creating its directory.
 *
 * It lives here rather than in `src/` because this is its only caller: the
 * retained `writeCanonical` was a store writer, and the store writer is
 * `quoin_store::store::write_canonical` now (quoin#504). What is left is a
 * fixture helper, and a fixture helper in `src/` is shipped code pretending to
 * be test support.
 */
function writeCanonical(path: string, value: unknown): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, canonicalJson(value), "utf8");
}

/** `runs/<suite>/<commit12>.json` — one file is one run of one suite. */
export function runPath(repo: string, suite: string, commit: string): string {
  return join(storeRoot(repo), "runs", suite, `${commit.slice(0, 12)}.json`);
}

/** Transcribe one suite run. */
export function writeRun(repo: string, record: RunRecord): string {
  const path = runPath(repo, record.suite, record.commit);
  writeCanonical(path, { ...record, schemaVersion: STORE_SCHEMA_VERSION });
  return path;
}

/** Write the whole binding graph, in the order the reader sees it. */
export function writeBindings(
  repo: string,
  file: { bindings: readonly Binding[] },
): void {
  const bindings = [...file.bindings].sort((a, b) => {
    if (a.obligation !== b.obligation)
      return a.obligation < b.obligation ? -1 : 1;
    if (a.suite === b.suite) return 0;
    return a.suite < b.suite ? -1 : 1;
  });
  writeCanonical(bindingsPath(repo), {
    schemaVersion: STORE_SCHEMA_VERSION,
    bindings,
  });
}

/** Write the ratchet baseline. */
export function writeBaseline(repo: string, file: BaselineFile): void {
  writeCanonical(baselinePath(repo), {
    schemaVersion: STORE_SCHEMA_VERSION,
    commit: file.commit,
    accepted: [...file.accepted].sort(),
  });
}
