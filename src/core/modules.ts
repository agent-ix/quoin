import { readFileSync } from "node:fs";
import { join } from "node:path";

import { packageRoot } from "../package-root.js";
import { SEMANTIC_ROOT } from "../semantic/root.js";
import { carriesPayload, runCoreAllowFailure } from "./exec.js";
import type {
  EnsureDefaultsPayload,
  EnsureDefaultsRequest,
  InstallPayload,
  InstallRequest,
  InstalledModule,
  ListPayload,
  ListRequest,
  Mode,
  RemovePayload,
  RemoveRequest,
} from "./types.js";

/**
 * Installing, listing and removing spec modules across the `quoin-core`
 * boundary (quoin#446, Stage 7 of quoin#373).
 *
 * Replaces `src/plugins.ts` and `src/modules.ts`. Everything they decided —
 * how a CLI source argument parses, where a module materialises, what the
 * registry holds and in what byte order, when a semantic contract violation
 * rejects an install and what is rolled back when it does, and what a `lazy`
 * reconcile may skip — is now decided by `quoin-modules` and `quoin-semantic`.
 *
 * # Why these ops carry no documents
 *
 * `config.resolve_org` next door is pure: its inputs are three small files, so
 * they ride in the request and the library half opens nothing. These four
 * cannot be. An install resolves a **git remote over the network**, extracts a
 * subtree, and rewrites a registry; none of that fits on stdin, and a request
 * carrying the tree would be a request carrying the answer. So the capability
 * is granted instead of transported: `quoin-core`'s `main.rs` — the one file
 * outside `tc_library_containment.rs`'s audit — constructs the installer, the
 * `gix` resolver and the semantic gate and hands them to the operation, which
 * still names no filesystem of its own. See `quoin_core::capabilities`.
 *
 * The user-visible output of `quoin module list/install/remove/ensure-defaults`
 * is unchanged, and one part of that took work rather than following: the
 * registry record's JSON *values* are pinned byte-for-byte against the
 * TypeScript writer by `quoin-modules`' `tc_381_235` and `tc_381_239`, but the
 * boundary emits canonical JSON, whose keys are sorted at every depth, so the
 * record's printed **key order** would have changed on its own. See
 * `REGISTRY_KEY_ORDER` below, and `tests/core-modules.test.ts` for the pin.
 */

/** One installed module's registry record, as the registry file holds it. */
export type { InstalledModule } from "./types.js";

/**
 * The vendored semantic contract, published to the subprocess.
 *
 * The tree ships inside this npm package, so only this side knows where it is;
 * `quoin-core` refuses an install rather than accepting a module unjudged when
 * the variable is absent. `??=` rather than an assignment so a caller — a test
 * pointing at a fixture contract, an operator debugging one — keeps whatever it
 * set.
 */
function publishSemanticRoot(): void {
  process.env.QUOIN_SEMANTIC_ROOT ??= SEMANTIC_ROOT;
}

/** One boundary call, with the failure surfaced rather than swallowed. */
function call<T>(op: string, request: unknown): T {
  publishSemanticRoot();
  const result = runCoreAllowFailure(op, request);
  if (!carriesPayload(result.exitCode)) {
    const detail = result.diagnostics
      .map((d) => `${d.code}: ${d.message}`)
      .join("\n");
    throw new Error(
      `quoin-core ${op} exited ${result.exitCode}` +
        (detail ? `:\n${detail}` : " with no diagnostic on stderr."),
    );
  }
  return result.payload as T;
}

/**
 * The registry record's key order, restored after the boundary sorted it.
 *
 * **This is a user-visible property, not a cosmetic one.** `quoin module list`
 * and `quoin module install` print their records with `JSON.stringify`, so the
 * key order a caller sees is the key order of the object handed to it. Before
 * the cutover that object was parsed straight out of `registry.json`, so it
 * carried the file's own field order. The boundary emits **canonical JSON**,
 * whose keys are sorted at every depth (`installedAt` first, `source` seventh),
 * and a payload parsed from it therefore carries sorted keys instead — which
 * would have changed the output of both commands without changing a single
 * value.
 *
 * So the order is restored here, from the one place that has the authority to
 * state it: `quoin_modules::InstalledModule`'s declaration order, which is also
 * the order serde writes into `registry.json`. `tests/core-modules.test.ts`
 * pins it against a real registry file rather than against this list, so a
 * field added to the Rust struct is caught by a failing test rather than
 * silently dropped from the printed record.
 */
const REGISTRY_KEY_ORDER: Record<string, readonly string[]> = {
  record: [
    "name",
    "source",
    "ref",
    "sha",
    "resolvedPath",
    "targetPath",
    "installedAt",
    "semantic",
  ],
  // `Source` is internally tagged, so `type` leads; the rest is the union of
  // the six variants' fields, each variant's own order preserved within it.
  source: [
    "type",
    "repo",
    "url",
    "path",
    "package",
    "ref",
    "sha",
    "version",
    "registry",
  ],
  // `exports` is a `BTreeMap`, so its own keys are sorted in both spellings.
  semantic: ["package", "semanticCore", "exports"],
};

/** One object with `order` applied; keys it does not name keep their place. */
function ordered(
  value: unknown,
  order: readonly string[],
): Record<string, unknown> {
  const record = value as Record<string, unknown>;
  const out: Record<string, unknown> = {};
  for (const key of order) {
    if (key in record) out[key] = record[key];
  }
  // Anything the boundary sent that the list does not name is kept rather than
  // dropped: losing a field is a worse failure than ordering it last, and the
  // pin in `tests/core-modules.test.ts` is what reports the omission.
  for (const [key, item] of Object.entries(record)) {
    if (!(key in out)) out[key] = item;
  }
  return out;
}

/** One record with the registry's key order restored, at every depth. */
function inRegistryOrder(module: InstalledModule): InstalledModule {
  const record = ordered(module, REGISTRY_KEY_ORDER.record);
  if (record.source) {
    record.source = ordered(record.source, REGISTRY_KEY_ORDER.source);
  }
  if (record.semantic) {
    record.semantic = ordered(record.semantic, REGISTRY_KEY_ORDER.semantic);
  }
  return record as unknown as InstalledModule;
}

/** The `home` field a request carries, omitted when the host resolves one. */
function homeField(home?: string): { home?: string } {
  return home === undefined ? {} : { home };
}

/**
 * Every installed module, in registry order.
 *
 * Registry order and not sorted here: `src/plugins.ts` returned what the
 * registry held, in the order it held it, and a sort introduced during a port
 * would be a user-visible change smuggled in under one.
 */
export function listModules(home?: string): InstalledModule[] {
  return call<ListPayload>("modules.list", {
    ...homeField(home),
  } satisfies ListRequest).modules.map(inRegistryOrder);
}

/**
 * Install or update one module from a CLI source argument.
 *
 * The `path:` / `github:` / `package:` spellings are unchanged; they are parsed
 * on the far side by `quoin_modules::Source::parse_arg`, pinned against the
 * TypeScript oracle by `tc_381_230`. A semantic-contract rejection throws, and
 * the previous version is restored before it does — `tc_381_281` and
 * `tc_381_282` pin both the restore and the clean-up of a rejected first
 * install.
 */
export function installModule(source: string, home?: string): InstalledModule {
  return inRegistryOrder(
    call<InstallPayload>("modules.install", {
      source,
      ...homeField(home),
    } satisfies InstallRequest).module,
  );
}

/** Remove an installed module and its registry record. */
export function removeModule(name: string, home?: string): void {
  call<RemovePayload>("modules.remove", {
    name,
    ...homeField(home),
  } satisfies RemoveRequest);
}

/**
 * The committed default module set shipped with quoin, as its YAML text.
 *
 * The text and not a parsed object: it is what the request carries, and
 * parsing it here only to re-serialise it would put the manifest schema in two
 * places. `quoin-modules` validates it, and `tc_381_233` pins that the
 * committed file parses to the same entries the TypeScript validator produced.
 */
export function defaultModulesManifest(): string {
  // Not derived here: the bundler flattens every chunk to `dist/`, so a file
  // two directories down in the source tree is one directory down at runtime
  // and no `dirname` count is right in both. See `src/package-root.ts`.
  return readFileSync(join(packageRoot(), "default-modules.yaml"), "utf8");
}

/**
 * Lazily install the default module set into `~/.ix/filament/modules`.
 *
 * Idempotent, and once installed and pinned it performs no network read
 * (`tc_381_283`), which is what makes it safe to call before every catalog
 * read. Pass a manifest — as YAML text — to override the committed set.
 */
export function ensureDefaultModules(
  home?: string,
  manifest: string = defaultModulesManifest(),
  mode: Mode = "lazy",
): EnsureDefaultsPayload {
  return call<EnsureDefaultsPayload>("modules.ensure_defaults", {
    manifest,
    mode,
    ...homeField(home),
  } satisfies EnsureDefaultsRequest);
}
