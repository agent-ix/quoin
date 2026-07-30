import { readFileSync } from "node:fs";
import { join } from "node:path";

/**
 * Where a resolved organization came from, or `none` when nothing yielded one.
 *
 * Reported alongside the organization itself so an author can tell an explicitly
 * stated org from one inferred off the repository's remote.
 */
export type OrgSource = "flag" | "env" | "git" | "none";

export interface ResolvedOrg {
  /** The organization, or `undefined` when unresolved. */
  org?: string;
  source: OrgSource;
}

/** Message shown when no source yielded an organization (FR-025, NFR-003). */
export const UNRESOLVED_ORG_MESSAGE =
  "could not determine the authoring organization: no --org flag, no QUOIN_ORG, " +
  'and no [remote "origin"] in .git/config. Pass --org <name>.';

/**
 * Resolve the authoring organization for a repository (FR-025).
 *
 * Precedence, first non-empty wins: the explicit `--org` value, `QUOIN_ORG`,
 * then the `origin` remote in the repository's git config.
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
  options: { flag?: string; env?: NodeJS.ProcessEnv } = {},
): ResolvedOrg {
  const flag = options.flag?.trim();
  if (flag) return { org: flag, source: "flag" };

  const env = (options.env ?? process.env).QUOIN_ORG?.trim();
  if (env) return { org: env, source: "env" };

  const fromGit = orgFromGitConfig(repoRoot);
  if (fromGit) return { org: fromGit, source: "git" };

  return { source: "none" };
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
  let config: string;
  try {
    config = readFileSync(join(repoRoot, ".git", "config"), "utf8");
  } catch {
    return undefined;
  }
  return originOrg(config);
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
      inOrigin = trimmed.replaceAll(/\s/g, "") === '[remote"origin"]';
      continue;
    }
    if (!inOrigin || !trimmed.startsWith("url")) continue;

    const url = trimmed.slice(3).trimStart();
    if (!url.startsWith("=")) continue;

    // The scp-style form (`git@host:org/repo`) puts the path after the last
    // colon; https urls have no colon past the scheme, so taking the last
    // segment covers both.
    const tail = url.slice(1).trim().split(":").pop();
    const segments = tail?.replace(/\/+$/, "").split("/") ?? [];
    const repo = segments.pop()?.replace(/\.git$/, "");
    const org = segments.pop();
    if (!org || !repo) return undefined;
    return org;
  }
  return undefined;
}
