import { readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

import { ConfigService } from "@agent-ix/ix-cli-core";

import {
  QUOIN_ENV_BINDINGS,
  QUOIN_PLUGIN_ID,
  QuoinConfigSchema,
} from "./config-schema.js";

/**
 * Where a resolved organization came from, or `none` when nothing yielded one.
 *
 * Reported alongside the organization itself so an author can tell an explicitly
 * stated org from one inferred off the repository's remote.
 */
export type OrgSource = "flag" | "env" | "config" | "git" | "none";

export interface ResolvedOrg {
  /** The organization, or `undefined` when unresolved. */
  org?: string;
  source: OrgSource;
}

/** Message shown when no source yielded an organization (FR-025, NFR-003). */
export const UNRESOLVED_ORG_MESSAGE =
  "could not determine the authoring organization: no --org flag, no QUOIN_ORG, " +
  'no stored config, and no [remote "origin"] in .git/config. ' +
  "Pass --org <name>, or store one with `quoin config set org <name>`.";

/**
 * Resolve the authoring organization for a repository (FR-025).
 *
 * Precedence, first non-empty wins: the explicit `--org` value, `QUOIN_ORG`,
 * the stored config, then the `origin` remote in the repository's git config.
 *
 * A stored value outranks the remote because it is something a person said,
 * where the remote is only what quoin could infer — but it does not weaken the
 * no-substitution rule below: a config file is an explicit statement, never a
 * value quoin picked. `QUOIN_ORG` is declared to `ConfigService` as an env
 * binding, so it is checked here only when a caller supplies its own `env`
 * (tests); in the normal path ConfigService layers it over the file itself,
 * keeping one precedence rule in one place.
 *
 * Deliberately returns `source: "none"` rather than substituting a default. An
 * org qualifies a repository so same-named repos in different organizations stay
 * distinguishable, so inventing one defeats the point of carrying it — and a
 * declared org is human-facing identity in a published artifact, where a
 * plausible-but-wrong value is harder to notice than an absent one that stops
 * the author and asks. (quire-rs #15 made org a required, caller-supplied input
 * for the same reason; this is quoin's half of that contract.)
 *
 * Note this is a deliberate divergence from filament-ide-rs's `repo_identity`,
 * which falls back to a `local` sentinel — correct there, because a code-symbol
 * qualifier must always produce some name.
 */
export function resolveOrg(
  repoRoot: string,
  options: {
    flag?: string;
    env?: NodeJS.ProcessEnv;
    /**
     * Project-local `.ix` layering overrides. Omit them in normal use:
     * `BaseCommand.init` publishes the runtime context (honouring
     * `--no-project-config`) and `ConfigService` falls back to it, so the
     * project layer works without quoin threading anything. Supplied by tests
     * that need a hermetic root.
     */
    projectConfigRoot?: string;
    projectConfigEnabled?: boolean;
  } = {},
): ResolvedOrg {
  const flag = options.flag?.trim();
  if (flag) return { org: flag, source: "flag" };

  if (options.env) {
    const env = options.env.QUOIN_ORG?.trim();
    if (env) return { org: env, source: "env" };
  } else {
    const stored = storedOrg(options);
    // ConfigService layers QUOIN_ORG over the file via the declared env
    // binding, so a value here came from whichever of the two won.
    if (stored) {
      return {
        org: stored,
        source: process.env.QUOIN_ORG?.trim() === stored ? "env" : "config",
      };
    }
  }

  const fromGit = orgFromGitConfig(repoRoot);
  if (fromGit) return { org: fromGit, source: "git" };

  return { source: "none" };
}

/**
 * Read the organization from quoin's stored config, with `QUOIN_ORG` layered
 * over it by `ConfigService` (FR-027).
 *
 * A malformed config must not stop an author writing specs: ConfigService
 * already returns schema defaults and records the problem for `config doctor`
 * rather than throwing, and any other failure here is treated the same way —
 * no stored org, carry on to the remote.
 */
function storedOrg(options: {
  projectConfigRoot?: string;
  projectConfigEnabled?: boolean;
}): string | undefined {
  // No try/catch here on purpose: `get()` does not throw. ConfigService's
  // contract is that a parse or validation failure returns schema defaults and
  // records an incident for `config doctor`, so a bad file already resolves to
  // "no stored org" and carries on to the remote — which the malformed- and
  // unreadable-config tests both exercise. A guard would be unreachable.
  return ConfigService.forPlugin(QUOIN_PLUGIN_ID, QuoinConfigSchema, {
    envBindings: QUOIN_ENV_BINDINGS,
    ...(options.projectConfigRoot
      ? { projectConfigRoot: options.projectConfigRoot }
      : {}),
    ...(options.projectConfigEnabled === undefined
      ? {}
      : { projectConfigEnabled: options.projectConfigEnabled }),
  })
    .get()
    .org?.trim();
}

/**
 * Read the organization from the `origin` remote in `<repoRoot>/.git/config`.
 *
 * Reads the config file directly rather than shelling out to `git`, so
 * resolution holds where no git executable is present (NFR-004). An unreadable
 * config, an absent `origin`, or an unparsable url yields `undefined` — never an
 * error, since "no org here" is a resolution outcome, not a failure.
 */
function orgFromGitConfig(repoRoot: string): string | undefined {
  const gitDir = resolveGitDir(repoRoot);
  if (!gitDir) return undefined;
  try {
    return originOrg(readFileSync(join(gitDir, "config"), "utf8"));
  } catch {
    return undefined;
  }
}

/**
 * Locate the git directory holding the config for `repoRoot`.
 *
 * In an ordinary checkout that is `<repoRoot>/.git`. In a worktree (and in a
 * submodule) `.git` is a *file* holding `gitdir: <path>`, and the config lives
 * in the shared common directory that `<gitdir>/commondir` points at — so a
 * worktree still resolves the org its main checkout would.
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

/**
 * Parse the org out of the `[remote "origin"]` url in a git config.
 *
 * Handles both `git@host:org/repo.git` and `https://host/org/repo.git`. Exported
 * for direct testing of the url forms.
 */
export function originOrg(config: string): string | undefined {
  let inOrigin = false;
  for (const line of config.split("\n")) {
    const trimmed = line.trim();
    if (trimmed.startsWith("[")) {
      // git treats section names case-insensitively but subsection names (the
      // quoted remote name) case-sensitively.
      inOrigin =
        trimmed.replaceAll(/\s/g, "").toLowerCase().startsWith("[remote") &&
        trimmed.replaceAll(/\s/g, "").slice("[remote".length) === '"origin"]';
      continue;
    }
    if (!inOrigin || !trimmed.startsWith("url")) continue;

    const url = trimmed.slice(3).trimStart();
    if (!url.startsWith("=")) continue;
    return orgFromRemoteUrl(url.slice(1).trim());
  }
  return undefined;
}

/**
 * Extract the owning org from a remote url, or `undefined` when the url does not
 * name one.
 *
 * Only *host-based* remotes carry an org: `scheme://host/org/repo` and the
 * scp-style `[user@]host:org/repo`. A local-path remote (`/srv/git/repo.git`,
 * `../sibling`, `file:///…`) has no org at all, and a host-based url with a
 * single path segment (`https://host/repo.git`) names a repo but no owner.
 *
 * Both cases return `undefined` rather than a best guess. Taking the
 * second-to-last path segment regardless would answer `git` for
 * `/srv/git/repo.git`, `..` for `../sibling`, and the *hostname* for
 * `https://host/repo.git` — a confidently-reported wrong org, which is the exact
 * failure FR-025 exists to prevent.
 */
function orgFromRemoteUrl(url: string): string | undefined {
  let path: string | undefined;

  const scheme = /^[A-Za-z][A-Za-z0-9+.-]*:\/\//.exec(url);
  if (scheme) {
    // file:// is a local path dressed as a url — it names no org.
    if (url.slice(0, scheme[0].length - 3).toLowerCase() === "file") {
      return undefined;
    }
    const afterHost = url.slice(scheme[0].length).indexOf("/");
    if (afterHost === -1) return undefined;
    path = url.slice(scheme[0].length + afterHost + 1);
  } else if (
    !url.startsWith("/") &&
    !url.startsWith(".") &&
    !url.startsWith("~")
  ) {
    // scp-style `[user@]host:org/repo`. A bare path before the colon is a host;
    // no colon at all means a relative local path, which names no org.
    const colon = url.indexOf(":");
    if (colon === -1) return undefined;
    path = url.slice(colon + 1);
  } else {
    return undefined;
  }

  const segments = path.replace(/\/+$/, "").split("/").filter(Boolean);
  if (segments.length < 2) return undefined;

  // The repo is the last segment; its owner is the one before it. Nested
  // namespaces (`org/subgroup/repo`) therefore qualify by the innermost group,
  // matching filament-ide-rs's repo_identity so both layers name a repo alike.
  // The length check above plus the empty-segment filter guarantee this is a
  // non-empty string, so there is no impossible-state branch to guard.
  return segments[segments.length - 2];
}
