/**
 * Semantic catalog locks (FR-101).
 *
 * Records what a resolution resolved, so that an artifact produced today can
 * still be attributed to the contract that produced it.
 *
 * Two failures in this campaign are why this exists. `quoin#350`: a stale
 * `quire-rs` pin sat in front of the whole suite and removed every check at
 * once — by refusing, which is the good direction, and it still meant the
 * suite proved nothing until somebody noticed. `quire-rs#405`: a measurement
 * pinned ten modules and the engine answered from whichever copy it found
 * first, so the verdicts were sound and their attribution was not.
 *
 * Both are a resolution that cannot state what it resolved.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";

/** Every state a declared target can be in. Closed. */
export type TargetState =
  "present" | "missing" | "incompatible" | "unknown-major";

export interface TargetRecord {
  readonly target: string;
  readonly state: TargetState;
  /** Present only when the target has a generated package. */
  readonly coordinates?: string;
  readonly fingerprint?: string;
  /** Set when `missing` or `incompatible`: who owns the gap. */
  readonly owningIssue?: string;
  readonly detail?: string;
}

export interface LockRecord {
  readonly module: string;
  readonly root: string;
  /** The commit. A tag may accompany it and may never replace it. */
  readonly commit: string | null;
  readonly tag: string | null;
  readonly clean: boolean;
  readonly schemaVersion: string | null;
  readonly semanticCore: string | null;
  readonly compilerVersion: string | null;
  readonly artifactDigest: string | null;
  readonly resolution: "compiled" | "dynamic-only";
  readonly targets: readonly TargetRecord[];
}

export interface LockDiagnostic {
  readonly module: string;
  readonly code:
    "incompatible-lock" | "missing-generated-target" | "unknown-schema-major";
  readonly message: string;
}

function git(cwd: string, ...args: string[]): string | null {
  try {
    // stderr silenced: `describe --exact-match` writes "no tag exactly
    // matches" for the ordinary case of an untagged commit, and a resolution
    // that prints an error for its normal path teaches readers to ignore its
    // errors.
    return execFileSync("git", args, {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return null;
  }
}

/** The recognised schema majors. A major outside this set is not guessed at. */
export const RECOGNISED_MAJORS = Object.freeze(["0", "1"]);

export function digestOf(value: unknown): string {
  const canonical = (node: unknown): unknown => {
    if (Array.isArray(node)) return node.map(canonical);
    if (node !== null && typeof node === "object") {
      const out: Record<string, unknown> = {};
      for (const key of Object.keys(node as object).sort()) {
        out[key] = canonical((node as Record<string, unknown>)[key]);
      }
      return out;
    }
    return node;
  };
  return `sha256:${createHash("sha256")
    .update(JSON.stringify(canonical(value)))
    .digest("hex")}`;
}

export interface ModuleInput {
  readonly name: string;
  readonly root: string;
  readonly semantic?: {
    readonly contract_version?: string;
    readonly semantic_core?: string;
    readonly targets?: readonly string[];
    readonly packages?: Readonly<
      Record<string, { coordinates: string; fingerprint: string }>
    >;
  };
}

export interface ResolveOptions {
  /** Declared target vocabulary; a module may name a subset. */
  readonly targets?: readonly string[];
  /** Fingerprints the lock expects, keyed `<module>/<target>`. */
  readonly expected?: Readonly<Record<string, string>>;
  /** Who owns a target that has no generated package. */
  readonly owners?: Readonly<Record<string, string>>;
  readonly compilerVersion?: string;
}

const DEFAULT_TARGETS = Object.freeze([
  "json-schema",
  "rust",
  "typescript",
  "python-pydantic-v2",
  "python-dataclass",
]);

/**
 * Resolves one module to a lock record.
 *
 * A dirty tree is recorded rather than rejected: it is a fact about the
 * resolution, and dropping the module would silently shrink the catalog — the
 * failure that makes a later rate describe a population nobody declared.
 */
export function lockModule(
  module: ModuleInput,
  options: ResolveOptions = {},
): { record: LockRecord; diagnostics: LockDiagnostic[] } {
  const diagnostics: LockDiagnostic[] = [];
  const semantic = module.semantic;
  const commit = git(module.root, "rev-parse", "HEAD");
  // `--exact-match` so a tag is reported only when it names *this* commit; a
  // nearest-tag answer would read as a pin and behave as a guess.
  const tag = git(module.root, "describe", "--tags", "--exact-match", "HEAD");
  const status = git(module.root, "status", "--porcelain");

  const schemaVersion = semantic?.contract_version ?? null;
  const major = schemaVersion ? schemaVersion.split(".")[0] : null;

  const declared = semantic?.targets ?? options.targets ?? DEFAULT_TARGETS;
  const packages = semantic?.packages ?? {};
  const targets: TargetRecord[] = [];

  for (const target of declared) {
    if (major !== null && !RECOGNISED_MAJORS.includes(major)) {
      // Not resolved against the nearest recognised major: a fallback here
      // would substitute a contract the module never declared.
      targets.push({
        target,
        state: "unknown-major",
        detail: `schema major ${major} is not recognised`,
      });
      diagnostics.push({
        module: module.name,
        code: "unknown-schema-major",
        message: `${module.name} declares schema version ${schemaVersion}; major ${major} is outside the recognised set`,
      });
      continue;
    }

    const entry = packages[target];
    if (!entry) {
      const owningIssue = options.owners?.[target];
      targets.push({
        target,
        state: "missing",
        ...(owningIssue ? { owningIssue } : {}),
        detail: "no generated package",
      });
      diagnostics.push({
        module: module.name,
        code: "missing-generated-target",
        message:
          `${module.name} declares target ${target} with no generated package` +
          (owningIssue ? `; owned by ${owningIssue}` : ""),
      });
      continue;
    }

    const expected = options.expected?.[`${module.name}/${target}`];
    if (expected !== undefined && expected !== entry.fingerprint) {
      // Reported, never substituted. A catalog that quietly accepts a
      // different package than the one it locked has stopped being a lock.
      targets.push({
        target,
        state: "incompatible",
        coordinates: entry.coordinates,
        fingerprint: entry.fingerprint,
        detail: `lock expects ${expected}`,
      });
      diagnostics.push({
        module: module.name,
        code: "incompatible-lock",
        message: `${module.name}/${target} fingerprints ${entry.fingerprint} and the lock expects ${expected}; not substituted`,
      });
      continue;
    }

    targets.push({
      target,
      state: "present",
      coordinates: entry.coordinates,
      fingerprint: entry.fingerprint,
    });
  }

  const record: LockRecord = {
    module: module.name,
    root: module.root,
    commit,
    tag,
    clean: status === "",
    schemaVersion,
    semanticCore: semantic?.semantic_core ?? null,
    compilerVersion: options.compilerVersion ?? null,
    artifactDigest: semantic ? digestOf(semantic) : null,
    resolution: semantic ? "compiled" : "dynamic-only",
    targets,
  };

  return { record, diagnostics };
}

/**
 * Resolves a whole catalog, in a stable order.
 *
 * Sorted by module name so two resolutions of an unchanged environment produce
 * byte-identical records: an order that follows discovery makes the lock
 * depend on the filesystem, and the lock is supposed to depend on the modules.
 */
export function lockCatalog(
  modules: readonly ModuleInput[],
  options: ResolveOptions = {},
): { records: LockRecord[]; diagnostics: LockDiagnostic[] } {
  const records: LockRecord[] = [];
  const diagnostics: LockDiagnostic[] = [];
  for (const module of [...modules].sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
  )) {
    const resolved = lockModule(module, options);
    records.push(resolved.record);
    diagnostics.push(...resolved.diagnostics);
  }
  return { records, diagnostics };
}
