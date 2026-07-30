import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
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
  it("prefers --org over QUOIN_ORG and the git remote", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(
      resolveOrg(root, { flag: "from-flag", env: { QUOIN_ORG: "from-env" } }),
    ).toEqual({ org: "from-flag", source: "flag" });
  });

  it("prefers QUOIN_ORG over the git remote when no flag is given", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(resolveOrg(root, { env: { QUOIN_ORG: "from-env" } })).toEqual({
      org: "from-env",
      source: "env",
    });
  });

  it("falls through to the git remote when neither flag nor env is set", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(resolveOrg(root, { env: {} })).toEqual({
      org: "from-git",
      source: "git",
    });
  });

  it("ignores a blank flag and a blank QUOIN_ORG rather than resolving to empty", () => {
    const root = repoWithConfig(gitConfig("git@github.com:from-git/repo.git"));
    expect(resolveOrg(root, { flag: "   ", env: { QUOIN_ORG: "  " } })).toEqual(
      { org: "from-git", source: "git" },
    );
  });
});

describe("origin remote parsing (FR-025-AC-2, FR-025-AC-3)", () => {
  it("parses the org from an SSH remote url", () => {
    const root = repoWithConfig(gitConfig("git@github.com:acme/widgets.git"));
    expect(resolveOrg(root, { env: {} }).org).toBe("acme");
  });

  it("parses the org from an HTTPS remote url", () => {
    const root = repoWithConfig(
      gitConfig("https://github.com/acme/widgets.git"),
    );
    expect(resolveOrg(root, { env: {} }).org).toBe("acme");
  });

  it("parses urls with no .git suffix and a trailing slash", () => {
    expect(originOrg(gitConfig("https://github.com/acme/widgets/"))).toBe(
      "acme",
    );
    expect(originOrg(gitConfig("git@github.com:acme/widgets"))).toBe("acme");
  });

  it("reads the origin remote, not another remote that precedes it", () => {
    const config = `[remote "upstream"]\n\turl = git@github.com:wrong/repo.git\n[remote "origin"]\n\turl = git@github.com:right/repo.git\n`;
    expect(originOrg(config)).toBe("right");
  });

  it("parses a self-hosted https url with a port", () => {
    expect(
      originOrg(gitConfig("https://git.example.com:8443/acme/widgets.git")),
    ).toBe("acme");
    expect(originOrg(gitConfig("ssh://git@host:2222/acme/widgets.git"))).toBe(
      "acme",
    );
  });

  it("qualifies a nested namespace by its innermost group", () => {
    // Matches filament-ide-rs repo_identity, so both layers name a repo alike.
    expect(originOrg(gitConfig("https://gitlab.com/top/sub/widgets.git"))).toBe(
      "sub",
    );
  });

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
describe("remotes that name no organization (FR-025-AC-5)", () => {
  it.each([
    ["a host-based url with no owner segment", "https://github.com/repo.git"],
    ["an absolute local path", "/srv/git/myrepo.git"],
    ["a relative local path", "../sibling-repo"],
    ["a file:// url", "file:///srv/git/myrepo.git"],
    ["a home-relative path", "~/repos/thing.git"],
  ])("yields no org for %s", (_label, url) => {
    expect(originOrg(gitConfig(url))).toBeUndefined();
  });

  it("reports unresolved rather than a wrong org for a local-path remote", () => {
    const root = repoWithConfig(gitConfig("/srv/git/myrepo.git"));
    expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  });
});

describe("worktree and submodule checkouts (FR-025-AC-2)", () => {
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

  it("yields no org when the .git file points nowhere useful", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-org-broken-"));
    writeFileSync(join(root, ".git"), "gitdir: /nonexistent/path\n");
    expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  });

  it("yields no org when .git is a file with no gitdir pointer", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-org-nogitdir-"));
    writeFileSync(join(root, ".git"), "not a gitdir pointer\n");
    expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  });
});

describe("unresolved organization (FR-025-AC-4, FR-025-AC-5)", () => {
  it("resolves to none when the repo has no .git/config", () => {
    expect(resolveOrg(repoWithConfig(), { env: {} })).toEqual({
      source: "none",
    });
  });

  it("resolves to none when the config declares no origin remote", () => {
    const root = repoWithConfig(
      `[core]\n\trepositoryformatversion = 0\n[remote "upstream"]\n\turl = git@github.com:other/repo.git\n`,
    );
    expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  });

  it("resolves to none when the origin url has no org/repo tail", () => {
    const root = repoWithConfig(gitConfig("https://github.com/"));
    expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  });

  it("never substitutes a default organization", () => {
    const resolved = resolveOrg(repoWithConfig(), { env: {} });
    expect(resolved.org).toBeUndefined();
    expect(UNRESOLVED_ORG_MESSAGE).toContain("--org");
  });
});

describe("standalone resolution (FR-025-AC-7)", () => {
  it("resolves from the git remote with no git executable on PATH", () => {
    const root = repoWithConfig(gitConfig("git@github.com:acme/widgets.git"));
    const path = process.env.PATH;
    process.env.PATH = "";
    try {
      expect(resolveOrg(root, { env: {} }).org).toBe("acme");
    } finally {
      process.env.PATH = path;
    }
  });
});
