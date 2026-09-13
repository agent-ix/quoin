/**
 * The one rule `src/core/org.ts` restates, pinned against the library that
 * owns it (quoin#446).
 *
 * Everything else about resolving an org moved behind `config.resolve_org` and
 * is decided by `quoin_config`, pinned against the filesystem path by
 * `quoin-config`'s `tc_446_026`. One thing could not move: the *location* of
 * the project-local config file. ix-cli-core declares `configPathForRoot` in
 * `paths.d.ts` but does not export it from its package entry, so
 * `src/core/org.ts` spells `<root>/config.d/<plugin-id>.yaml` itself.
 *
 * A copied rule with nothing checking it is how two implementations drift. The
 * first describe writes a file at the derived path and asserts that
 * ix-cli-core's own `ConfigService` — the thing that owns the layout — reads it
 * back, so if the layout changes under us this fails rather than the org
 * silently resolving from the wrong layer. It needs no subprocess.
 *
 * The remaining describes drive `resolveOrg` itself, which now spawns
 * `quoin-core`, so they **skip cleanly** when no binary is reachable — the same
 * guard and the same lane as `tests/core-exec-e2e.test.ts` (quoin#412), and
 * `make rust-e2e` runs them.
 *
 * One of them audits the thing only this side can be wrong about. The request
 * carries three documents, and `quoin_config` decides over the bytes it is
 * given; a Rust test can therefore only re-derive a verdict from a request that
 * has already left something out. A selection grown too **narrow** — a path
 * spelled wrong, a layer skipped — produces a well-formed request with a
 * document missing and an org that silently falls through to the next source,
 * with every Rust test still green. So each document is exercised from here as
 * the only source of an org in its own real tree, with a population floor so a
 * suite that stopped running the cases fails instead of reporting nothing.
 */

import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { ConfigService } from "@agent-ix/ix-cli-core";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  QUOIN_ENV_BINDINGS,
  QUOIN_PLUGIN_ID,
  QuoinConfigSchema,
} from "../src/config-schema.js";
import { quoinCoreExecutable } from "../src/core/exec.js";
import type { OrgOptions, ResolvedOrg } from "../src/core/org.js";
import { resolveOrg, unresolvedOrgMessage } from "../src/core/org.js";

function boundaryAvailable(): boolean {
  try {
    quoinCoreExecutable();
    return true;
  } catch {
    return false;
  }
}

const available = boundaryAvailable();

const priorHome = process.env.XDG_CONFIG_HOME;
const priorOrg = process.env.QUOIN_ORG;
let configHome: string;

beforeEach(() => {
  configHome = mkdtempSync(join(tmpdir(), "quoin-core-org-"));
  process.env.XDG_CONFIG_HOME = configHome;
  delete process.env.QUOIN_ORG;
});

afterEach(() => {
  if (priorHome === undefined) delete process.env.XDG_CONFIG_HOME;
  else process.env.XDG_CONFIG_HOME = priorHome;
  if (priorOrg === undefined) delete process.env.QUOIN_ORG;
  else process.env.QUOIN_ORG = priorOrg;
});

/** A project config root with an org stored at the derived path. */
function projectRootWithOrg(org: string): string {
  const root = join(mkdtempSync(join(tmpdir(), "quoin-core-org-proj-")), ".ix");
  // Spelled exactly as `src/core/org.ts` derives it, and nowhere else in this
  // file, so the assertion below is about the derivation and not about two
  // independent spellings agreeing.
  mkdirSync(join(root, "config.d"), { recursive: true });
  writeFileSync(
    join(root, "config.d", `${QUOIN_PLUGIN_ID}.yaml`),
    `org: ${org}\n`,
  );
  return root;
}

/** A git config naming `org` on its `origin` remote, or naming no remote. */
function gitConfig(org?: string): string {
  const core = `[core]\n\trepositoryformatversion = 0\n`;
  return org === undefined
    ? core
    : `${core}[remote "origin"]\n\turl = git@github.com:${org}/repo.git\n`;
}

/** A repo root whose `origin` remote names `org` (default `from-git`). */
function repoWithRemote(org: string = "from-git"): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-core-org-repo-"));
  mkdirSync(join(root, ".git"), { recursive: true });
  writeFileSync(join(root, ".git", "config"), gitConfig(org));
  return root;
}

/** A repo root with a git config that names no remote at all. */
function repoWithoutRemote(): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-core-org-bare-"));
  mkdirSync(join(root, ".git"), { recursive: true });
  writeFileSync(join(root, ".git", "config"), gitConfig());
  return root;
}

/** Write `org` into the user-level config, wherever ix-cli-core puts it. */
function userConfigWithOrg(org: string): void {
  const path = ConfigService.forPlugin(QUOIN_PLUGIN_ID, QuoinConfigSchema, {
    envBindings: QUOIN_ENV_BINDINGS,
  }).filePath();
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `org: ${org}\n`);
}

describe("the project config path src/core/org.ts derives", () => {
  // Trace: FR-027-AC-9
  it("is the path ix-cli-core's own ConfigService reads", () => {
    // The pin. If ix-cli-core moves the project layer, `get()` stops seeing
    // `project-level` and this fails here, next to the comment explaining why
    // the path is spelled in our tree at all.
    const projectConfigRoot = projectRootWithOrg("project-level");
    const config = ConfigService.forPlugin(QUOIN_PLUGIN_ID, QuoinConfigSchema, {
      envBindings: QUOIN_ENV_BINDINGS,
      projectConfigRoot,
      projectConfigEnabled: true,
    }).get();
    expect(config.org).toBe("project-level");
  });
});

describe.skipIf(!available)(
  "every document the request is supposed to carry",
  () => {
    /**
     * The three documents, each the only source of an org in its own tree.
     *
     * This is the property a Rust-side check structurally cannot hold. The
     * boundary decides over the bytes it is *given*, so `quoin-config` can only
     * ever re-derive a verdict from a request that already left something out;
     * a selection that grew too **narrow** — a path spelled wrong, a layer
     * skipped, a read that started throwing — produces a well-formed request with
     * a document missing, and every Rust test still passes while the answer
     * silently falls through to the next source. So the selection is exercised
     * from this side, over a real tree, one document at a time: if the document
     * is not read and sent, the org it names cannot come back.
     */
    const documents: [
      string,
      () => { repoRoot: string; expected: ResolvedOrg; options: OrgOptions },
    ][] = [
      [
        "the user config",
        () => {
          userConfigWithOrg("from-user");
          return {
            repoRoot: repoWithoutRemote(),
            options: { projectConfigEnabled: false },
            expected: { org: "from-user", source: "config", degraded: false },
          };
        },
      ],
      [
        "the project config",
        () => ({
          repoRoot: repoWithoutRemote(),
          options: {
            projectConfigRoot: projectRootWithOrg("from-project"),
            projectConfigEnabled: true,
          },
          expected: { org: "from-project", source: "config", degraded: false },
        }),
      ],
      [
        "the repository's git config",
        () => ({
          repoRoot: repoWithRemote("from-git"),
          options: { projectConfigEnabled: false },
          expected: { org: "from-git", source: "git", degraded: false },
        }),
      ],
    ];

    let reached = 0;

    it.each(documents)("reaches %s", (_name, build) => {
      const { repoRoot, options, expected } = build();
      expect(resolveOrg(repoRoot, options)).toEqual(expected);
      reached += 1;
    });

    // Trace: FR-025-AC-1, FR-027-AC-9
    it("left none of them unexercised", () => {
      // The population floor. A `describe` whose cases all stopped running would
      // otherwise report a green selection audit over nothing.
      expect(reached).toBe(3);
    });

    // Trace: FR-025-AC-1, FR-025-AC-8
    it("follows a worktree's .git file to the common directory", () => {
      // Not a variation on the same case: in a worktree `.git` is a *file*, and
      // the config lives in the main checkout that `commondir` names. A selection
      // that stopped at the pointer sends no git config at all, so the org this
      // asserts is the one only the full walk can reach.
      const common = mkdtempSync(join(tmpdir(), "quoin-core-org-common-"));
      mkdirSync(join(common, ".git"), { recursive: true });
      writeFileSync(join(common, ".git", "config"), gitConfig("from-worktree"));
      const gitDir = join(common, ".git", "worktrees", "wt");
      mkdirSync(gitDir, { recursive: true });
      writeFileSync(join(gitDir, "commondir"), "../..\n");

      const root = mkdtempSync(join(tmpdir(), "quoin-core-org-wt-"));
      writeFileSync(join(root, ".git"), `gitdir: ${gitDir}\n`);

      expect(resolveOrg(root, { projectConfigEnabled: false })).toEqual({
        org: "from-worktree",
        source: "git",
        degraded: false,
      });
    });
  },
);

describe.skipIf(!available)("resolveOrg ↔ quoin-core", () => {
  // Trace: FR-027-AC-9
  it("reads the project layer at the derived path", () => {
    const projectConfigRoot = projectRootWithOrg("project-level");
    expect(
      resolveOrg(repoWithRemote(), {
        projectConfigRoot,
        projectConfigEnabled: true,
      }),
    ).toEqual({ org: "project-level", source: "config", degraded: false });
  });

  // Trace: FR-027-AC-9
  it("ignores the project layer when the invocation disables it", () => {
    const projectConfigRoot = projectRootWithOrg("project-level");
    expect(
      resolveOrg(repoWithRemote(), {
        projectConfigRoot,
        projectConfigEnabled: false,
      }),
    ).toEqual({ org: "from-git", source: "git", degraded: false });
  });

  // Trace: FR-025-AC-4
  it("serves the unresolved-org sentence over the boundary, once", () => {
    // A function rather than the `const` `src/org.ts` exported: reading the
    // sentence spawns `quoin-core`, and a module-level const would have made
    // every importer of the package pay for it at import time. The second call
    // is the memoisation: same string, no second spawn.
    const first = unresolvedOrgMessage();
    expect(first).toMatch(/--org/);
    expect(unresolvedOrgMessage()).toBe(first);
  });
});
