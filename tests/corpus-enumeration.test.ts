import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import {
  assertOutputOutsideCorpus,
  enumerateCorpus,
} from "../src/measurement/enumerate.js";

function repo(root: string, name: string, withSpec = true): string {
  const dir = join(root, name);
  mkdirSync(dir, { recursive: true });
  execFileSync("git", ["init", "-q"], { cwd: dir });
  if (withSpec) {
    mkdirSync(join(dir, "spec"), { recursive: true });
    writeFileSync(join(dir, "spec", "a.md"), "# a\n");
  }
  return dir;
}

describe("TC-1500..1505 corpus enumeration states its own population", () => {
  // TC-1500
  it("counts a repository only when it carries both .git and spec", () => {
    const root = mkdtempSync(join(tmpdir(), "corpus-"));
    repo(root, "with-spec");
    repo(root, "no-spec", false);
    mkdirSync(join(root, "plain"), { recursive: true });

    const record = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: [],
      corpusId: "test",
    });

    expect(record.repositories.map((r) => r.path.split("/").pop())).toEqual([
      "with-spec",
    ]);
    expect(
      record.excluded.filter((e) => e.rule === "no-spec-directory"),
    ).toHaveLength(1);
    expect(
      record.excluded.filter((e) => e.rule === "not-a-repository"),
    ).toHaveLength(1);
  });

  // TC-1501
  it("excludes a .git file as git-link-file rather than counting the repository twice", () => {
    const root = mkdtempSync(join(tmpdir(), "corpus-"));
    const linked = join(root, "worktree-like");
    mkdirSync(join(linked, "spec"), { recursive: true });
    writeFileSync(join(linked, ".git"), "gitdir: /elsewhere\n");

    const record = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: [],
      corpusId: "test",
    });

    expect(record.repositories).toHaveLength(0);
    expect(record.excluded).toContainEqual({
      path: linked,
      rule: "git-link-file",
    });
  });

  // TC-1502
  it("records the declared vocabulary verbatim, because the count depends on it", () => {
    const root = mkdtempSync(join(tmpdir(), "corpus-"));
    repo(root, "kept");
    repo(root, "skipped");

    const record = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: ["skipped"],
      corpusId: "test",
    });

    expect(record.exclusionVocabulary).toEqual(["skipped"]);
    expect(record.repositories).toHaveLength(1);
    expect(record.excluded).toContainEqual({
      path: join(root, "skipped"),
      rule: "excluded-directory",
    });
  });

  // TC-1503
  it("refuses an output directory inside an enumerated repository, before reading", () => {
    const root = mkdtempSync(join(tmpdir(), "corpus-"));
    const inside = repo(root, "target");

    expect(() =>
      assertOutputOutsideCorpus(join(inside, "reports"), root, []),
    ).toThrow(/inside the enumerated repository/);

    expect(() =>
      assertOutputOutsideCorpus(join(root, "reports"), root, []),
    ).not.toThrow();
  });

  // TC-1504
  it("reports a clean repository as stable and its document count", () => {
    const root = mkdtempSync(join(tmpdir(), "corpus-"));
    const dir = repo(root, "clean-one");
    execFileSync("git", ["add", "-A"], { cwd: dir });
    execFileSync(
      "git",
      ["-c", "user.email=t@x", "-c", "user.name=t", "commit", "-qm", "init"],
      { cwd: dir },
    );

    const record = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: [],
      corpusId: "test",
    });

    expect(record.repositories[0]?.documents).toBe(1);
    expect(record.repositories[0]?.clean).toBe(true);
    expect(record.repositories[0]?.stable).toBe(true);
  });
});
