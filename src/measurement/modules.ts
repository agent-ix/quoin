/**
 * Module-set resolution and the toolchain record (FR-085).
 *
 * Every file is read with `git show <rev>:<path>` against the module
 * repository, never from its working tree. Three of the 251 enumerated
 * repositories were dirty at census time; reading a working tree would let an
 * uncommitted edit change what a published rate is attributable to, and the
 * rate would still look clean.
 *
 * A missing or unresolvable member of the required set is a hard refusal before
 * any corpus document is read. A measurement that silently drops a module
 * reports a rate for a population nobody declared.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { join } from "node:path";

export interface ModuleFinding {
  readonly module: string;
  readonly kind:
    | "data-schema-digest-mismatch"
    | "module-load-diagnostic"
    | "ref-divergence"
    | "manifest-unreadable";
  readonly detail: string;
}

export interface ModuleRecord {
  readonly name: string;
  readonly repositoryPath: string;
  /** The ref asked for, and the commit it resolved to — a moved tag is visible. */
  readonly ref: string;
  readonly commit: string;
  readonly manifestPath: string;
  readonly manifestVersion: string | null;
  readonly manifestDigest: string;
  readonly semanticCore: string | null;
  readonly objectTypes: readonly string[];
  readonly artifactTypes: readonly string[];
  /** `null` when the module publishes none; its object types are still taken. */
  readonly mappingsDigest: string | null;
  readonly schemaDigests: Readonly<Record<string, string>>;
}

export interface ToolchainRecord {
  readonly engineVersion: string | null;
  readonly engineRevision: string | null;
  readonly cliVersion: string | null;
  readonly cliRevision: string | null;
  readonly semanticCoreByModule: Readonly<Record<string, string | null>>;
}

function git(cwd: string, ...args: string[]): string {
  return execFileSync("git", args, { cwd, encoding: "utf8", maxBuffer: 1 << 28 });
}

function tryGit(cwd: string, ...args: string[]): string | null {
  try {
    return git(cwd, ...args).trim();
  } catch {
    return null;
  }
}

export function sha256(text: string): string {
  return `sha256:${createHash("sha256").update(text, "utf8").digest("hex")}`;
}

/**
 * Reads a path out of the object store at a revision. Never the working tree,
 * and never trimmed: the trailing newline is part of the bytes a declared
 * digest was computed over, and stripping it reports every schema in the
 * ecosystem as a mismatch.
 */
export function showAt(repo: string, rev: string, path: string): string | null {
  try {
    return git(repo, "show", `${rev}:${path}`);
  } catch {
    return null;
  }
}

/** A scalar `key: value` read without a YAML dependency, for the few we need. */
function scalar(yaml: string, key: string): string | null {
  const m = new RegExp(`^${key}:\\s*["']?([^"'\\n]+)["']?\\s*$`, "m").exec(yaml);
  return m ? m[1].trim() : null;
}

function listNames(yaml: string, block: string): string[] {
  const lines = yaml.split("\n");
  const start = lines.findIndex((l) => l === `${block}:`);
  if (start < 0) return [];
  const names: string[] = [];
  for (let i = start + 1; i < lines.length; i += 1) {
    const line = lines[i];
    // A new top-level key ends the block. List items sit at column zero in
    // these manifests, so indentation cannot be the terminator.
    if (/^[a-z_]+:/.test(line)) break;
    const m = /^-\s+name:\s*["']?([A-Za-z0-9_.-]+)/.exec(line);
    if (m) names.push(m[1]);
  }
  return names;
}

export function resolveModule(options: {
  name: string;
  repositoryPath: string;
  ref: string;
  findings: ModuleFinding[];
}): ModuleRecord | null {
  const { name, repositoryPath, ref, findings } = options;
  const commit = tryGit(repositoryPath, "rev-parse", `${ref}^{commit}`);
  if (!commit) {
    findings.push({
      module: name,
      kind: "manifest-unreadable",
      detail: `ref ${ref} does not resolve in ${repositoryPath}`,
    });
    return null;
  }

  const tree = tryGit(repositoryPath, "ls-tree", "-r", "--name-only", commit);
  const manifestPath = (tree ?? "")
    .split("\n")
    .find((p) => /^[a-z_]+\/manifest\.yaml$/.test(p));
  if (!manifestPath) {
    findings.push({
      module: name,
      kind: "manifest-unreadable",
      detail: `no <package>/manifest.yaml at ${commit}`,
    });
    return null;
  }

  const manifest = showAt(repositoryPath, commit, manifestPath);
  if (manifest === null) {
    findings.push({
      module: name,
      kind: "manifest-unreadable",
      detail: `${manifestPath} unreadable at ${commit}`,
    });
    return null;
  }

  const pkg = manifestPath.split("/")[0];
  const schemaDigests: Record<string, string> = {};
  for (const path of (tree ?? "").split("\n")) {
    if (!path.startsWith(`${pkg}/schemas/`) || !path.endsWith(".json")) continue;
    const body = showAt(repositoryPath, commit, path);
    if (body !== null) schemaDigests[path] = sha256(body);
  }

  // A declared digest that disagrees with the bytes actually read is a module
  // finding, never a document failure: no corpus document caused it.
  for (const m of manifest.matchAll(
    /schema:\s*["']?([^"'\s]+)["']?\s*\n\s*digest:\s*["']?(sha256:[0-9a-f]{64})/g,
  )) {
    const declaredPath = m[1].replace(/^\.\//, "");
    const full = declaredPath.startsWith(pkg)
      ? declaredPath
      : join(pkg, declaredPath);
    const actual = schemaDigests[full];
    if (actual && actual !== m[2]) {
      findings.push({
        module: name,
        kind: "data-schema-digest-mismatch",
        detail: `${full}: manifest declares ${m[2]}, file digests ${actual}`,
      });
    }
  }

  const mappingsPath = (tree ?? "")
    .split("\n")
    .find((p) => p === `${pkg}/mappings.yaml`);
  const mappings = mappingsPath
    ? showAt(repositoryPath, commit, mappingsPath)
    : null;

  return {
    name,
    repositoryPath,
    ref,
    commit,
    manifestPath,
    manifestVersion: scalar(manifest, "version"),
    manifestDigest: sha256(manifest),
    semanticCore: scalar(manifest, "  semantic_core") ?? scalar(manifest, "semantic_core"),
    objectTypes: listNames(manifest, "object_types"),
    artifactTypes: listNames(manifest, "artifact_types"),
    mappingsDigest: mappings === null ? null : sha256(mappings),
    schemaDigests,
  };
}

/** Resolves the whole required set. Throws if any member is unresolvable. */
export function resolveModuleSet(
  required: readonly { name: string; repositoryPath: string; ref: string }[],
): { modules: ModuleRecord[]; findings: ModuleFinding[] } {
  const findings: ModuleFinding[] = [];
  const modules: ModuleRecord[] = [];
  const missing: string[] = [];
  for (const entry of required) {
    const record = resolveModule({ ...entry, findings });
    if (record) modules.push(record);
    else missing.push(entry.name);
  }
  if (missing.length > 0) {
    throw new Error(
      `required modules did not resolve: ${missing.join(", ")}. ` +
        `A rate computed without them is a rate for a population nobody declared.`,
    );
  }
  return { modules, findings };
}

export function toolchainRecord(
  modules: readonly ModuleRecord[],
  options: { quireRepo?: string; quoinRepo?: string } = {},
): ToolchainRecord {
  const semanticCoreByModule: Record<string, string | null> = {};
  for (const m of modules) semanticCoreByModule[m.name] = m.semanticCore;
  return {
    engineVersion: tryGit(options.quireRepo ?? ".", "describe", "--tags", "--always"),
    engineRevision: options.quireRepo
      ? tryGit(options.quireRepo, "rev-parse", "HEAD")
      : null,
    cliVersion: tryGit(options.quoinRepo ?? ".", "describe", "--tags", "--always"),
    cliRevision: options.quoinRepo
      ? tryGit(options.quoinRepo, "rev-parse", "HEAD")
      : null,
    semanticCoreByModule,
  };
}
