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
      originOrg(gitConfig("https://git.example.com/acme/widgets.git")),
    ).toBe("acme");
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
