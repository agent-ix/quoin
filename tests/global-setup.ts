/**
 * Materialize the default module set before any test runs.
 *
 * **Why this exists.** Several tests exercise `quoin evidence record` and
 * `quoin advise` end to end, which shell out to `quire coverage`. That needs a
 * module declaring a `traceability:` model, and quire reads the installed set
 * from `~/.ix/filament/modules`.
 *
 * On a developer machine those modules are already there, so the tests passed
 * locally for months. In CI they are not, and every command-level test failed:
 *
 * ```
 * quire coverage --scope /tmp/quoin-adapters-… --json exited 1:
 * SearchPathMissing: /home/runner/.ix/filament/modules
 * no module in scope declares a `traceability:` model
 * ```
 *
 * That took `publish` down with it on v0.18.0 and v0.19.0 — the release gate had
 * been red since the command-level tests landed, and nothing noticed because the
 * suite runs on `workflow_dispatch` only.
 *
 * Installing here rather than in the workflow is deliberate: the release
 * workflow is a reusable one in another repository, so a fix there would not
 * travel with this repository's own tests. This makes the suite self-sufficient
 * wherever it runs.
 *
 * Idempotent — `ensureDefaultModules` reconciles in `lazy` mode rather than
 * reinstalling, so a developer's existing modules are left alone.
 */

import { ensureDefaultModules } from "../src/modules.js";

export default async function setup(): Promise<void> {
  try {
    ensureDefaultModules();
  } catch (cause) {
    // Reported, never swallowed. A failure here means the command-level tests
    // are about to fail for a reason that has nothing to do with the code under
    // test, and the operator needs to see which.
    const reason = cause instanceof Error ? cause.message : String(cause);
    throw new Error(
      `could not materialize the default module set, so the command-level ` +
        `tests cannot run: ${reason}`,
    );
  }
}
