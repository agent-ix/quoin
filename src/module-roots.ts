import { existsSync, readdirSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

/**
 * Where installed spec modules live, and which candidate paths are modules.
 *
 * Below `catalog.ts` rather than inside it, and that is a topology statement
 * rather than a tidy-up: `src/catalog.ts` now reads each module's semantic
 * block across the `quoin-core` boundary (quoin#452), so it imports `src/core/`
 * — while `src/core/completeness.ts` and `src/core/snapshot.ts` have always
 * needed to know where modules are. Those two edges together are a cycle, and
 * quoin#376 forbids one because Cargo forbids one: the crate topology of
 * quoin#373 cannot contain a loop the TypeScript still has.
 *
 * So the shared surface moves DOWN into a module neither side owns, as #376
 * requires, instead of an edge being cut arbitrarily. `catalog.ts` re-exports
 * every name below, so no caller and no published export changes.
 */

export function ixHome(): string {
  return process.env.IX_HOME && process.env.IX_HOME.length > 0
    ? process.env.IX_HOME
    : join(homedir(), ".ix");
}

/** The single directory that holds installed Filament modules; also read by quire-rs. */
export function filamentModulesDir(home = ixHome()): string {
  return join(home, "filament", "modules");
}

export function defaultModuleRoots(home = ixHome()): string[] {
  const roots: string[] = [];
  const env = process.env.QUOIN_MODULE_PATHS;
  if (env) roots.push(...env.split(":").filter(Boolean));

  const installed = filamentModulesDir(home);
  if (existsSync(installed)) {
    for (const name of readdirSync(installed))
      roots.push(join(installed, name));
  }
  return roots;
}

export function locateModuleRoot(candidate: string): string | undefined {
  const root = resolve(candidate);
  if (!existsSync(root)) return undefined;
  if (existsSync(join(root, "manifest.yaml"))) return root;
  if (!statSync(root).isDirectory()) return undefined;
  for (const child of readdirSync(root)) {
    const childRoot = join(root, child);
    if (existsSync(join(childRoot, "manifest.yaml"))) return childRoot;
  }
  return undefined;
}
