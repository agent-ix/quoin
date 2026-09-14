import { SEMANTIC_ROOT } from "./semantic/root.js";
import { carriesPayload, runCoreAllowFailure } from "./core/exec.js";
import type {
  Catalog as SpecCatalog,
  LoadRequest,
  SpecCatalogEntry,
  SpecModule,
} from "./core/types.js";

/**
 * Where modules live, re-exported from `src/module-roots.ts`.
 *
 * The filesystem-free catalog projection is now `quoin-catalog`; this module
 * remains only as the temporary Node boundary client while the oclif shell is
 * still being retired. Keeping roots below the client prevents a dependency
 * cycle and preserves the public module-location exports until Stage 9.
 */
export {
  defaultModuleRoots,
  filamentModulesDir,
  ixHome,
  locateModuleRoot,
} from "./module-roots.js";

/** The Rust-owned catalog types retained under their published names. */
export type { SpecCatalog, SpecCatalogEntry, SpecModule };

/** Publish the vendored semantic contract to the Rust host once per process. */
function publishSemanticRoot(): void {
  process.env.QUOIN_SEMANTIC_ROOT ??= SEMANTIC_ROOT;
}

/** Invoke one catalog operation, preserving the boundary's partial payload rule. */
function call<T>(op: string, request: unknown): T {
  publishSemanticRoot();
  const result = runCoreAllowFailure(op, request);
  if (carriesPayload(result.exitCode)) return result.payload as T;
  const detail = result.diagnostics
    .map((diagnostic) => `${diagnostic.code}: ${diagnostic.message}`)
    .join("\n");
  throw new Error(
    `quoin-core ${op} exited ${result.exitCode}` +
      (detail ? `:\n${detail}` : " with no diagnostic on stderr."),
  );
}

/**
 * Read the active artifact/object catalog across the Rust boundary.
 *
 * Omit `moduleRoots` for Rust-owned default discovery (`QUOIN_MODULE_PATHS`
 * followed by `$IX_HOME/filament/modules`), or pass a closed candidate list
 * for an isolated/reproducible view.
 */
export function loadCatalog(moduleRoots?: string[]): SpecCatalog {
  return call<SpecCatalog>(
    "catalog.load",
    moduleRoots === undefined
      ? {}
      : ({ roots: moduleRoots } satisfies LoadRequest),
  );
}

/**
 * Find the first catalog entry by ASCII case-insensitive type name.
 *
 * This compatibility helper disappears with the TypeScript authoring shell;
 * the deciding catalog projection and its canonical lookup data are Rust-owned.
 */
export function findCatalogEntry(
  catalog: SpecCatalog,
  name: string,
): SpecCatalogEntry | undefined {
  return catalog.entries.find(
    (entry) => entry.name.toLowerCase() === name.toLowerCase(),
  );
}
