import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SCRIPT = "scripts/release-drift.js";

function run(args: string[], env: NodeJS.ProcessEnv = {}) {
  try {
    const stdout = execFileSync("node", [SCRIPT, ...args], {
      encoding: "utf8",
      env: { ...process.env, ...env },
    });
    return { status: 0, stdout };
  } catch (error) {
    const failure = error as { status: number; stdout: string };
    return { status: failure.status, stdout: failure.stdout };
  }
}

/**
 * A throwaway repo with one release tag, so drift can be produced on demand.
 * Mirrors the real package shape closely enough for path derivation to apply.
 */
function fixtureRepo() {
  const root = mkdtempSync(join(tmpdir(), "quoin-drift-"));
  const git = (...args: string[]) =>
    execFileSync("git", args, { cwd: root, encoding: "utf8" });

  git("init", "-q", "-b", "main");
  git("config", "user.email", "test@example.com");
  git("config", "user.name", "test");

  writeFileSync(
    join(root, "package.json"),
    JSON.stringify(
      { name: "fixture", files: ["dist/", "default-modules.yaml", "skills/"] },
      null,
      2,
    ),
  );
  writeFileSync(
    join(root, "default-modules.yaml"),
    "schemaVersion: 1\nentries:\n  - name: mod\n    source:\n      url: agent-ix/mod\n      ref: v0.1.0\n",
  );
  mkdirSync(join(root, "src"));
  writeFileSync(join(root, "src", "index.ts"), "export const x = 1;\n");
  git("add", "-A");
  git("commit", "-qm", "release");
  git("tag", "v0.1.0");

  return {
    root,
    git,
    cleanup: () => rmSync(root, { recursive: true, force: true }),
  };
}

test("release paths are derived from the shipped file set, not hardcoded", () => {
  const { stdout } = run(["paths"]);
  const paths = stdout.trim().split("\n");

  // dist/ is built, so the guard watches its source instead.
  expect(paths).toContain("src");
  expect(paths).not.toContain("dist");
  // package.json carries version, bin map and dependency ranges.
  expect(paths).toContain("package.json");
  // The files that decide published behaviour beyond compiled code.
  expect(paths).toContain("default-modules.yaml");
  expect(paths).toContain("skills");
  expect(paths).toContain("bin");
});

test("a pin-only change since the last tag trips the guard", () => {
  const repo = fixtureRepo();
  try {
    expect(run(["check"], { QUOIN_DRIFT_ROOT: repo.root }).status).toBe(0);

    // The exact v0.12.2 failure: only default-modules.yaml moves.
    writeFileSync(
      join(repo.root, "default-modules.yaml"),
      "schemaVersion: 1\nentries:\n  - name: mod\n    source:\n      url: agent-ix/mod\n      ref: v0.2.0\n",
    );
    repo.git("add", "-A");
    repo.git("commit", "-qm", "bump pin");

    const result = run(["check"], { QUOIN_DRIFT_ROOT: repo.root });
    expect(result.status).toBe(1);
    expect(result.stdout).toContain("default-modules.yaml");
  } finally {
    repo.cleanup();
  }
});

test("a change outside the shipped file set does not trip the guard", () => {
  const repo = fixtureRepo();
  try {
    writeFileSync(join(repo.root, "NOTES.md"), "not shipped\n");
    repo.git("add", "-A");
    repo.git("commit", "-qm", "notes");

    expect(run(["check"], { QUOIN_DRIFT_ROOT: repo.root }).status).toBe(0);
  } finally {
    repo.cleanup();
  }
});

test("pins reports behind, current and unresolved without failing", () => {
  const repo = fixtureRepo();
  try {
    const behind = run(["pins"], {
      QUOIN_DRIFT_ROOT: repo.root,
      QUOIN_DRIFT_LATEST_TAGS: JSON.stringify({ "agent-ix/mod": "v0.9.0" }),
    });
    expect(behind.status).toBe(0);
    expect(behind.stdout).toContain("behind");
    expect(behind.stdout).toContain("::warning::");

    const current = run(["pins"], {
      QUOIN_DRIFT_ROOT: repo.root,
      QUOIN_DRIFT_LATEST_TAGS: JSON.stringify({ "agent-ix/mod": "v0.1.0" }),
    });
    expect(current.stdout).toContain("current");
    expect(current.stdout).not.toContain("::warning::");

    // A failed lookup must read as unverified, never as a pass.
    const unresolved = run(["pins"], {
      QUOIN_DRIFT_ROOT: repo.root,
      QUOIN_DRIFT_LATEST_TAGS: "{}",
    });
    expect(unresolved.stdout).toContain("unknown");
    expect(unresolved.stdout).toContain("1 unresolved");
  } finally {
    repo.cleanup();
  }
});
