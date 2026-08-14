import { chmodSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { UNRESOLVED_ORG_MESSAGE, originOrg, resolveOrg } from "../src/org";

/** A repo root with the given `.git/config` contents, or none at all. */
function repoWithConfig(config?: string): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-org-"));
  if (config !== undefined) {
    mkdirSync(join(root, ".git"), { recursive: true });
    writeFileSync(join(root, ".git", "config"), config);
  }
  return root;
}

function gitConfig(url: string): string {
  return `[core]\n\trepositoryformatversion = 0\n[remote "origin"]\n\turl = ${url}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n`;
}

describe("resolveOrg precedence (FR-025-AC-1)", () => {
  // Trace: FR-023-AC-4
  // Trace: FR-025-AC-1
  it("prefers --org over QUOIN_ORG and the git remote", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(
      resolveOrg(root, { flag: "from-flag", env: { QUOIN_ORG: "from-env" } }),
    ).toEqual({ org: "from-flag", source: "flag" });
  });

  // Trace: FR-023-AC-4
  // Trace: FR-025-AC-1
  it("prefers QUOIN_ORG over the git remote when no flag is given", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(resolveOrg(root, { env: { QUOIN_ORG: "from-env" } })).toEqual({
      org: "from-env",
      source: "env",
    });
  });

  // Trace: FR-025-AC-1
  it("falls through to the git remote when neither flag nor env is set", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(resolveOrg(root, { env: {} })).toEqual({
      org: "from-git",
      source: "git",
    });
  });

  // Trace: FR-025-AC-1
  it("ignores a blank flag and a blank QUOIN_ORG rather than resolving to empty", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(resolveOrg(root, { flag: "   ", env: { QUOIN_ORG: "  " } })).toEqual(
      { org: "from-git", source: "git" },
    );
  });
});

describe("origin remote parsing (FR-025-AC-2, FR-025-AC-3)", () => {
  // Trace: FR-025-AC-2
  it("parses the org from an SSH remote url", () => {
    const root = repoWithConfig(gitConfig("git@github.com:acme/widgets.git"));
    expect(resolveOrg(root, { env: {} }).org).toBe("acme");
  });

  // Trace: FR-025-AC-3
  it("parses the org from an HTTPS remote url", () => {
    const root = repoWithConfig(
      gitConfig("https://github.com/acme/widgets.git"),
    );
    expect(resolveOrg(root, { env: {} }).org).toBe("acme");
  });

  // Trace: FR-025-AC-2
  it("parses urls with no .git suffix and a trailing slash", () => {
    expect(originOrg(gitConfig("https://github.com/acme/widgets/"))).toBe(
      "acme",
    );
    expect(originOrg(gitConfig("git@github.com:acme/widgets"))).toBe("acme");
  });

  // Trace: FR-025-AC-2
  it("reads the origin remote, not another remote that precedes it", () => {
    const config = `[remote "upstream"]\n\turl = git@github.com:wrong/repo.git\n[remote "origin"]\n\turl = git@github.com:right/repo.git\n`;
    expect(originOrg(config)).toBe("right");
  });

  // Trace: FR-025-AC-3
  it("parses a self-hosted https url with a port", () => {
    expect(
      originOrg(gitConfig("https://git.example.com:8443/acme/widgets.git")),
    ).toBe("acme");
    expect(originOrg(gitConfig("ssh://git@host:2222/acme/widgets.git"))).toBe(
      "acme",
    );
  });
});

describe("nested namespaces (FR-025-AC-10)", () => {
  // Trace: FR-025-AC-10
  it("qualifies by the segment immediately preceding the repository", () => {
    // Matches filament-ide-rs repo_identity, so both layers name a repo alike.
    expect(originOrg(gitConfig("https://gitlab.com/top/sub/widgets.git"))).toBe(
      "sub",
    );
  });
});

describe("git config name matching (FR-025-AC-11)", () => {
  // Trace: FR-025-AC-11
  it("matches the section name case-insensitively but the remote name exactly", () => {
    expect(
      originOrg('[REMOTE "origin"]\n\turl = git@github.com:acme/w.git\n'),
    ).toBe("acme");
    expect(
      originOrg('[remote "Origin"]\n\turl = git@github.com:acme/w.git\n'),
    ).toBeUndefined();
  });
});

// A remote that names no owner must yield nothing rather than a best guess.
// Taking the second-to-last path segment regardless would answer `git` for
// /srv/git/repo.git, `..` for ../sibling, and the hostname for
// https://host/repo.git -- a confidently-reported wrong org, the exact failure
// FR-025 exists to prevent.
describe("remotes that name no organization (FR-025-AC-9)", () => {
  // Trace: FR-025-AC-9
  it.each([
    ["a host-based url with no owner segment", "https://github.com/repo.git"],
    ["an absolute local path", "/srv/git/myrepo.git"],
    ["a relative local path", "../sibling-repo"],
    ["a file:// url", "file:///srv/git/myrepo.git"],
    ["a home-relative path", "~/repos/thing.git"],
  ])("yields no org for %s", (_label, url) => {
    expect(originOrg(gitConfig(url))).toBeUndefined();
  });

  // Trace: FR-025-AC-9
  it("reports unresolved rather than a wrong org for a local-path remote", () => {
    const root = repoWithConfig(gitConfig("/srv/git/myrepo.git"));
    expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  });
});

describe("worktree and submodule checkouts (FR-025-AC-8)", () => {
  // Trace: FR-025-AC-8
  it("resolves through a .git file pointing at a worktree gitdir", () => {
    // Mirror git's layout: the worktree's .git is a file naming a gitdir under
    // the main checkout, whose commondir points back at the shared .git that
    // actually holds the config.
    const main = mkdtempSync(join(tmpdir(), "quoin-org-main-"));
    const commonGit = join(main, ".git");
    const wtGitDir = join(commonGit, "worktrees", "feature");
    mkdirSync(wtGitDir, { recursive: true });
    writeFileSync(
      join(commonGit, "config"),
      gitConfig("git@github.com:acme/widgets.git"),
    );
    writeFileSync(join(wtGitDir, "commondir"), "../..\n");

    const worktree = mkdtempSync(join(tmpdir(), "quoin-org-wt-"));
    writeFileSync(join(worktree, ".git"), `gitdir: ${wtGitDir}\n`);

    expect(resolveOrg(worktree, { env: {} })).toEqual({
      org: "acme",
      source: "git",
    });
  });

  // Trace: FR-025-AC-8
  it("yields no org when the .git file points nowhere useful", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-org-broken-"));
    writeFileSync(join(root, ".git"), "gitdir: /nonexistent/path\n");
    expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  });

  // Trace: FR-025-AC-8
  it("yields no org when .git is a file with no gitdir pointer", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-org-nogitdir-"));
    writeFileSync(join(root, ".git"), "not a gitdir pointer\n");
    expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  });
});

describe("unresolved organization (FR-025-AC-4, FR-025-AC-5)", () => {
  // Trace: FR-025-AC-4
  it("resolves to none when the repo has no .git/config", () => {
    expect(resolveOrg(repoWithConfig(), { env: {} })).toEqual({
      source: "none",
    });
  });

  // Trace: FR-025-AC-4
  it("resolves to none when the config declares no origin remote", () => {
    const root = repoWithConfig(
      `[core]\n\trepositoryformatversion = 0\n[remote "upstream"]\n\turl = git@github.com:other/repo.git\n`,
    );
    expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  });

  // Trace: FR-025-AC-4
  it("resolves to none when the origin url has no org/repo tail", () => {
    const root = repoWithConfig(gitConfig("https://github.com/"));
    expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  });

  // Trace: FR-025-AC-5
  it("never substitutes a default organization", () => {
    // Every source is absent, so any org at all would be invented. The remedy
    // reaching the author is asserted on the rendered pack in write.test.ts;
    // here the point is that no value is produced and none is implied.
    expect(resolveOrg(repoWithConfig(), { env: {} })).toEqual({
      source: "none",
    });
    expect(UNRESOLVED_ORG_MESSAGE).toMatch(/pass --org/i);
  });
});

describe("unreadable git metadata", () => {
  it("yields no org when the .git file stats but cannot be read", () => {
    // Exercises the read failure that survives a successful stat -- e.g. a
    // .git file the process may see but not open.
    const root = mkdtempSync(join(tmpdir(), "quoin-org-unreadable-"));
    const dotGit = join(root, ".git");
    writeFileSync(dotGit, "gitdir: /somewhere\n");
    chmodSync(dotGit, 0o000);
    try {
      expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
    } finally {
      chmodSync(dotGit, 0o644);
    }
  });
});

// Malformed or exotic config lines must fall through to "no organization"
// rather than throw or guess (FR-025-AC-4, FR-025-AC-9).
describe("malformed remote entries", () => {
  it("ignores a url key that is not an assignment", () => {
    expect(originOrg('[remote "origin"]\n\turlsomething\n')).toBeUndefined();
  });

  it("yields no org for a scheme url with no path at all", () => {
    expect(originOrg('[remote "origin"]\n\turl = https://github.com\n')).toBe(
      undefined,
    );
  });

  it("yields no org for a bare host with no colon and no scheme", () => {
    expect(originOrg('[remote "origin"]\n\turl = justahostname\n')).toBe(
      undefined,
    );
  });

  it("yields no org when the path has an owner but an empty repo", () => {
    expect(
      originOrg('[remote "origin"]\n\turl = https://github.com/acme//\n'),
    ).toBeUndefined();
  });
});

// FR-027: a stored config value outranks the git remote, because it is
// something a person said where the remote is only what quoin inferred.
describe("stored configuration (FR-027)", () => {
  const priorHome = process.env.XDG_CONFIG_HOME;
  const priorOrg = process.env.QUOIN_ORG;
  let configHome: string;

  beforeEach(() => {
    configHome = mkdtempSync(join(tmpdir(), "quoin-org-cfg-"));
    process.env.XDG_CONFIG_HOME = configHome;
    delete process.env.QUOIN_ORG;
  });

  afterEach(() => {
    if (priorHome === undefined) delete process.env.XDG_CONFIG_HOME;
    else process.env.XDG_CONFIG_HOME = priorHome;
    if (priorOrg === undefined) delete process.env.QUOIN_ORG;
    else process.env.QUOIN_ORG = priorOrg;
  });

  function storeOrg(org: string): void {
    const dir = join(configHome, "ix", "config.d");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "quoin.yaml"), `org: ${org}\n`);
  }

  // Trace: FR-027-AC-1
  it("prefers a stored org over the git remote", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    storeOrg("from-config");
    expect(resolveOrg(root)).toEqual({ org: "from-config", source: "config" });
  });

  // Trace: FR-027-AC-4
  it("falls through to the git remote when nothing is stored", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(resolveOrg(root)).toEqual({ org: "from-git", source: "git" });
  });

  // Trace: FR-027-AC-2
  it("prefers an explicit --org over a stored value", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    storeOrg("from-config");
    expect(resolveOrg(root, { flag: "from-flag" })).toEqual({
      org: "from-flag",
      source: "flag",
    });
  });

  // Trace: FR-027-AC-3
  it("lets QUOIN_ORG layer over the stored value, reported as env", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    storeOrg("from-config");
    process.env.QUOIN_ORG = "from-env";
    expect(resolveOrg(root)).toEqual({ org: "from-env", source: "env" });
  });

  // Trace: FR-027-AC-4
  it("resolves to none when nothing is stored and there is no remote", () => {
    expect(resolveOrg(repoWithConfig())).toEqual({ source: "none" });
  });

  // Trace: FR-027-AC-9
  it("prefers a project-local org over the user-level one (FR-027-AC-9)", () => {
    // The per-repo layer is what lets one checkout declare a different org
    // without touching global state.
    storeOrg("user-level");
    const project = mkdtempSync(join(tmpdir(), "quoin-org-proj-"));
    const projectRoot = join(project, ".ix");
    mkdirSync(join(projectRoot, "config.d"), { recursive: true });
    writeFileSync(
      join(projectRoot, "config.d", "quoin.yaml"),
      "org: project-level\n",
    );

    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(
      resolveOrg(root, {
        projectConfigRoot: projectRoot,
        projectConfigEnabled: true,
      }),
    ).toEqual({ org: "project-level", source: "config" });
  });

  // Trace: FR-027-AC-9
  it("ignores the project layer when the invocation disables it (FR-027-AC-9)", () => {
    storeOrg("user-level");
    const project = mkdtempSync(join(tmpdir(), "quoin-org-proj-off-"));
    const projectRoot = join(project, ".ix");
    mkdirSync(join(projectRoot, "config.d"), { recursive: true });
    writeFileSync(
      join(projectRoot, "config.d", "quoin.yaml"),
      "org: project-level\n",
    );

    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(
      resolveOrg(root, {
        projectConfigRoot: projectRoot,
        projectConfigEnabled: false,
      }),
    ).toEqual({ org: "user-level", source: "config" });
  });

  it("ignores an unreadable config rather than failing the command", () => {
    // ConfigService returns defaults for a malformed file, but a read that
    // throws outright (permissions) must be absorbed the same way -- an author
    // has to be able to keep writing specs.
    const dir = join(configHome, "ix", "config.d");
    mkdirSync(dir, { recursive: true });
    const file = join(dir, "quoin.yaml");
    writeFileSync(file, "org: from-config\n");
    chmodSync(file, 0o000);
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    try {
      expect(resolveOrg(root)).toEqual({ org: "from-git", source: "git" });
    } finally {
      chmodSync(file, 0o644);
    }
  });

  // Trace: FR-027-AC-5
  it("ignores a malformed config rather than failing the command", () => {
    // ConfigService records the problem for `config doctor` and returns schema
    // defaults; an author must still be able to write specs.
    const dir = join(configHome, "ix", "config.d");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "quoin.yaml"), "org: [unterminated\n");
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(resolveOrg(root)).toEqual({ org: "from-git", source: "git" });
  });
});
