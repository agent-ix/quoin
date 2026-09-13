import { readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

import {
  ConfigService,
  getRuntimeContext,
  type RuntimeContext,
} from "@agent-ix/ix-cli-core";

import {
  QUOIN_ENV_BINDINGS,
  QUOIN_PLUGIN_ID,
  QuoinConfigSchema,
} from "../config-schema.js";
import { carriesPayload, runCoreAllowFailure } from "./exec.js";
import type {
  ResolveOrgPayload,
  ResolveOrgRequest,
  UnresolvedOrgMessagePayload,
} from "./types.js";

/**
 * Resolving the authoring organization across the `quoin-core` boundary
 * (quoin#446, Stage 7 of quoin#373).
 *
 * `src/org.ts` decided four things and this file decides none of them. The
 * precedence of flag over environment over stored config over git remote, the
 * layering and strict-schema validation of the two config documents, which
 * `[remote "origin"]` url shapes name an org, and the sentence shown when
 * nothing did — all four now live in `quoin_config` and are reached through
 * `config.resolve_org` and `config.unresolved_org_message`.
 *
 * What is left here is the host state the boundary is forbidden to acquire:
 * locating the git directory a repository's config actually lives in, and
 * reading three small files. `quoin-core`'s library half may not name a
 * filesystem (`rust/crates/quoin-core/tests/tc_library_containment.rs`), so the
 * shell does the reads and the library does the deciding. The equivalence
 * between deciding over those bytes and deciding over the tree they came from
 * is not assumed: `quoin-config`'s `tc_446_026` builds a real tree and asserts
 * both paths answer identically.
 */

/**
 * Where a resolved organization came from, or `none` when nothing yielded one.
 *
 * Reported alongside the organization itself so an author can tell an
 * explicitly stated org from one inferred off the repository's remote. The
 * five spellings are `quoin_config::OrgSource::as_str`'s; `src/write.ts` keys
 * its labels on them.
 */
export type OrgSource = "flag" | "env" | "config" | "git" | "none";

/**
 * A resolved organization: the boundary's own payload, with `source` narrowed.
 *
 * **Derived rather than re-declared, and that is FR-097 rather than taste.**
 * `src/org.ts` spelled this interface out by hand, and the fields happened to
 * match `ResolveOrgPayload` in the generated `src/core/types.ts` exactly — two
 * declarations of one contract, which is the drift `tc_1614` exists to catch
 * and did. So the shape now comes from the generated type and only the one
 * thing the JSON Schema cannot express is added: `source` is an enumeration
 * (`quoin_config::OrgSource::as_str`) that crosses the wire as a plain string,
 * and `src/write.ts` keys its labels on the five spellings.
 *
 * Consequently `degraded` is always present and `org` may be `null` rather than
 * absent — the payload's own shape. `degraded` says a config layer fell back to
 * schema defaults instead of contributing its content: `src/org.ts` discarded
 * that fact silently, so a malformed config resolved to "no stored org" with
 * nothing to show for it. The run is still a success — a broken config must not
 * stop an author writing specs (FR-027-AC-5) — but the caller can now say so.
 */
export type ResolvedOrg = Omit<ResolveOrgPayload, "source"> & {
  source: OrgSource;
};

/** Options accepted by {@link resolveOrg}. */
export interface OrgOptions {
  /** The explicit `--org` value. */
  flag?: string;
  /**
   * The environment the resolution reads. Defaults to `process.env`.
   *
   * It is a plain map layered over the config document by the declared
   * `QUOIN_ORG` binding, on the far side — one precedence rule in one place,
   * which is the property `src/org.ts` went out of its way to keep.
   */
  env?: NodeJS.ProcessEnv;
  /**
   * Project-local `.ix` layering overrides. Omit them in normal use:
   * `BaseCommand.init` publishes the runtime context (honouring
   * `--no-project-config`) and this reads it back. Supplied by tests that need
   * a hermetic root.
   */
  projectConfigRoot?: string;
  projectConfigEnabled?: boolean;
}

/**
 * Resolve the authoring organization for a repository (FR-025).
 *
 * Precedence, first non-empty wins: the explicit `--org` value, `QUOIN_ORG`,
 * the stored config, then the `origin` remote in the repository's git config.
 * Deliberately returns `source: "none"` rather than substituting a default; an
 * org qualifies a repository, so inventing one defeats the point of carrying
 * it. The rule is `quoin_config::resolve_org_from_documents`'s — this function
 * only supplies the documents it decides over.
 */
export function resolveOrg(
  repoRoot: string,
  options: OrgOptions = {},
): ResolvedOrg {
  const payload = call<ResolveOrgPayload>("config.resolve_org", {
    ...(options.flag === undefined ? {} : { flag: options.flag }),
    env: stringValuedEnv(options.env ?? process.env),
    ...documents(repoRoot, options),
  } satisfies ResolveOrgRequest);
  // Passed through rather than rebuilt: a field added to the payload reaches
  // the caller instead of being silently dropped by a literal that predates it.
  return { ...payload, source: payload.source as OrgSource };
}

/**
 * Parse the org out of the `[remote "origin"]` url in a git config.
 *
 * A `resolve_org` over that one document and nothing else: with no flag, no
 * environment and no config layer, the git remote is the only source left, so
 * this is the same rule rather than a second parser reachable another way.
 */
export function originOrg(config: string): string | undefined {
  const payload = call<ResolveOrgPayload>("config.resolve_org", {
    env: {},
    git_config: config,
  } satisfies ResolveOrgRequest);
  return payload.org ?? undefined;
}

/** Memoised: the sentence cannot change under a running binary. */
let cachedMessage: string | undefined;

/**
 * Message shown when no source yielded an organization (FR-025, NFR-003).
 *
 * Served over the boundary rather than restated here: it was a `const` in
 * `src/org.ts` and a `const` in `quoin_config::org`, and two copies of a
 * user-facing sentence is exactly the drift FR-101 retires.
 *
 * A function where `src/org.ts` exported a `const` string, and that shape
 * change is the point rather than an accident: reading the sentence is now a
 * subprocess call, and a module-level `const` would have made every importer of
 * the package spawn `quoin-core` at import time — including importers that
 * never resolve an org. The result is memoised, so the call happens at most
 * once per process and only if something asks.
 */
export function unresolvedOrgMessage(): string {
  cachedMessage ??= call<UnresolvedOrgMessagePayload>(
    "config.unresolved_org_message",
    {},
  ).message;
  return cachedMessage;
}

/** One boundary call, with the failure surfaced rather than swallowed. */
function call<T>(op: string, request: unknown): T {
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
 * `process.env` as the string→string map the request declares.
 *
 * Node types every entry as possibly `undefined` (an unset variable reads as
 * absent, not empty), and an `undefined` would fail `deny_unknown_fields`'s
 * sibling — the typed `BTreeMap<String, String>` — rather than being ignored.
 */
function stringValuedEnv(env: NodeJS.ProcessEnv): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [name, value] of Object.entries(env)) {
    if (typeof value === "string") out[name] = value;
  }
  return out;
}

/** The three documents the resolution decides over, as far as they exist. */
function documents(
  repoRoot: string,
  options: OrgOptions,
): Pick<ResolveOrgRequest, "user_config" | "project_config" | "git_config"> {
  const user = readIfPresent(userConfigPath(), MAX_CONFIG_LAYER_BYTES);
  const projectPath = projectConfigPath(options);
  const project = projectPath
    ? readIfPresent(projectPath, MAX_CONFIG_LAYER_BYTES)
    : undefined;
  const gitDir = resolveGitDir(repoRoot);
  const git = gitDir
    ? readIfPresent(join(gitDir, "config"), MAX_GIT_CONFIG_BYTES)
    : undefined;
  return {
    ...(user === undefined ? {} : { user_config: user }),
    ...(project === undefined ? {} : { project_config: project }),
    ...(git === undefined ? {} : { git_config: git }),
  };
}

/**
 * The largest config layer this side will send, in bytes.
 *
 * `quoin_config::service::MAX_CONFIG_FILE_BYTES`, and the same number the
 * boundary refuses past (`ops::config::MAX_CONFIG_LAYER_BYTES`). Restated here
 * because it is a ceiling on a *read*, and `tests/core-org.test.ts` pins the
 * behaviour it produces rather than the constant.
 */
const MAX_CONFIG_LAYER_BYTES = 1 << 20;

/**
 * The largest `.git/config` this side will send, in bytes.
 *
 * `quoin_config::org::MAX_GIT_CONFIG_BYTES`, matching the in-process resolver:
 * `org_from_git_config` stats the file and answers "no org here" past this
 * number rather than reading it.
 */
const MAX_GIT_CONFIG_BYTES = 4 << 20;

/**
 * An absent, unreadable or oversized document is absent, never an error.
 *
 * "No config here" is a resolution outcome, not a failure — the same tolerance
 * `ConfigService` applies, and the reason a malformed or unreadable file
 * resolves to "no stored org" and carries on to the remote.
 *
 * **The size check is here, before the read, and that is the point.** The
 * retained in-process rule *degrades* past its ceiling — `ConfigService.get`
 * logs an incident and falls back, `org_from_git_config` returns `None`, and a
 * broken config must not stop an author writing specs (FR-027-AC-5). The
 * boundary, correctly, *refuses* an oversized field (`CORE_REFUSED`, exit 2),
 * and {@link call} throws on a refusal. Reading a 2 MiB config whole and
 * shipping it across would therefore have turned a degradation into a dead
 * `quoin write`. Over the ceiling the layer is simply absent, which is the
 * answer both in-process paths already give. The boundary's refusal stays where
 * it is, as the defence against a caller that is not this one.
 */
function readIfPresent(path: string, limit: number): string | undefined {
  try {
    // `statSync` and not `readFileSync(...).length`: a ceiling applied after
    // the whole file is in memory is a remark about an allocation that already
    // happened (rust-style §"Untrusted input").
    const stats = statSync(path);
    if (stats.isFile() && stats.size > limit) return undefined;
    return readFileSync(path, "utf8");
  } catch {
    return undefined;
  }
}

/** The user-level config file, as ix-cli-core itself spells it. */
function userConfigPath(): string {
  return ConfigService.forPlugin(QUOIN_PLUGIN_ID, QuoinConfigSchema, {
    envBindings: QUOIN_ENV_BINDINGS,
  }).filePath();
}

/**
 * The project-level config file, when a project layer applies.
 *
 * ix-cli-core declares `configPathForRoot` but does not export it from its
 * package entry, so the last segment of the layout — `<root>/config.d/<id>.yaml`
 * for every id but the reserved `core` — is spelled here. That is the one rule
 * this file restates, and `tests/core-org.test.ts` pins it by writing a file at
 * the derived path and asserting ix-cli-core's own `ConfigService` reads it, so
 * the copy cannot drift from the library that owns it without a test failing.
 */
function projectConfigPath(options: OrgOptions): string | undefined {
  const context: RuntimeContext = getRuntimeContext();
  const enabled = options.projectConfigEnabled ?? context.projectConfigEnabled;
  if (!enabled) return undefined;
  const root = options.projectConfigRoot ?? context.projectConfigRoot;
  return root === undefined
    ? undefined
    : join(root, "config.d", `${QUOIN_PLUGIN_ID}.yaml`);
}

/**
 * Locate the git directory holding the config for `repoRoot`.
 *
 * In an ordinary checkout that is `<repoRoot>/.git`. In a worktree (and in a
 * submodule) `.git` is a *file* holding `gitdir: <path>`, and the config lives
 * in the shared common directory that `<gitdir>/commondir` points at — so a
 * worktree still resolves the org its main checkout would. Kept on this side
 * because a pointer chain is host state: `quoin_config::org::resolve_git_dir`
 * is the same walk, and `tc_446_028` pins the two to the same answer.
 */
function resolveGitDir(repoRoot: string): string | undefined {
  const dotGit = join(repoRoot, ".git");
  let stats;
  try {
    stats = statSync(dotGit);
  } catch {
    return undefined;
  }
  if (stats.isDirectory()) return dotGit;

  try {
    const pointer = readFileSync(dotGit, "utf8").match(
      /^gitdir:\s*(.+)$/m,
    )?.[1];
    if (!pointer) return undefined;
    const gitDir = resolve(repoRoot, pointer.trim());
    // `commondir` is how a worktree names the checkout that owns the config;
    // its absence means this gitdir holds the config itself.
    try {
      const common = readFileSync(join(gitDir, "commondir"), "utf8").trim();
      return resolve(gitDir, common);
    } catch {
      return gitDir;
    }
  } catch {
    return undefined;
  }
}
