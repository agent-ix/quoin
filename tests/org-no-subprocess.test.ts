import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// FR-025-AC-7: resolution executes no subprocess.
//
// Asserting that directly, rather than that PATH is unused: setting PATH to ""
// proves nothing, because `execFileSync("/usr/bin/git", …)` succeeds with an
// empty PATH and would pass such a test unchanged. Every child_process entry
// point is replaced with a recorder, so any attempt to shell out — by absolute
// path or otherwise — is caught.
//
// This lives in its own file because vi.mock applies for a whole module graph,
// and nothing else here should run against a stubbed child_process.
const calls: string[] = [];

vi.mock("node:child_process", () => {
  const record =
    (name: string) =>
    (...args: unknown[]) => {
      calls.push(`${name}(${String(args[0])})`);
      throw new Error(`unexpected subprocess: ${name}`);
    };
  return {
    execSync: record("execSync"),
    execFileSync: record("execFileSync"),
    spawnSync: record("spawnSync"),
    exec: record("exec"),
    execFile: record("execFile"),
    spawn: record("spawn"),
    fork: record("fork"),
    default: {},
  };
});

const { resolveOrg } = await import("../src/org");

test("resolves the org from a git remote without executing any subprocess", () => {
  const root = mkdtempSync(join(tmpdir(), "quoin-org-nosub-"));
  mkdirSync(join(root, ".git"), { recursive: true });
  writeFileSync(
    join(root, ".git", "config"),
    '[remote "origin"]\n\turl = git@github.com:acme/widgets.git\n',
  );

  expect(resolveOrg(root, { env: {} })).toEqual({
    org: "acme",
    source: "git",
  });
  expect(calls).toEqual([]);
});

test("reports unresolved without executing any subprocess", () => {
  const root = mkdtempSync(join(tmpdir(), "quoin-org-nosub-empty-"));
  expect(resolveOrg(root, { env: {} })).toEqual({ source: "none" });
  expect(calls).toEqual([]);
});
