import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Where the vendored semantic contract is, and nothing else (quoin#452).
 *
 * The contract itself — `schemas/` and `sweep-report.schema.json`, 35 files —
 * is DATA shipped inside this npm package, and after the cutover every rule it
 * expresses is read by `quoin-semantic` on the far side of the boundary. What
 * cannot move across is the ANSWER TO "where is it", because only this side
 * knows where its own package was installed: `quoin-core` is handed the path as
 * `QUOIN_SEMANTIC_ROOT` and refuses to judge anything without it.
 *
 * Its own module directory, and not `packageRoot()`: the bundler flattens every
 * chunk into `dist/`, so this file sits two directories down in the source tree
 * and one directory down when shipped, and `scripts/copy-quire-schemas.mjs`
 * copies the contract to `dist/` for exactly that reason. `import.meta.url` is
 * right in both trees; a `dirname` count is right in neither.
 */
export const SEMANTIC_ROOT = dirname(fileURLToPath(import.meta.url));
