import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * The root of the installed `@agent-ix/quoin` package.
 *
 * **Deliberately a module of its own at the top of `src/`, and that is the
 * whole point rather than a stray file.** The derivation is `dirname` twice off
 * `import.meta.url`, which is correct from `src/<file>.ts` when the sources are
 * imported directly *and* from `dist/<chunk>.js` once the bundler has flattened
 * every chunk to one directory — the two layouts this package actually runs in.
 * It is therefore only correct in a module that sits one level down, so a file
 * under `src/core/` or `src/semantic/` cannot spell it inline: three `dirname`s
 * read correctly in the source tree and resolve above the package in a build,
 * and two read correctly in a build and land on `src/` in the source tree.
 * quoin#446 shipped that mistake for exactly as long as it took to run
 * `quoin catalog list` against a built tree, which is why it lives here now.
 *
 * `src/version.ts` spells the same two `dirname`s inline because it already sits
 * at this depth; anything deeper imports this.
 */
export function packageRoot(): string {
  return dirname(dirname(fileURLToPath(import.meta.url)));
}
