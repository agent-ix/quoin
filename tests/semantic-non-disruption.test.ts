/**
 * NFR-017 non-disruption gates for the semantic module contract (issue #293,
 * TASK-043). TC-1379 (every default module loads) and TC-1380 (warning-only
 * sweep) live with the code they gate; TC-1381 and TC-1382 are the change-set
 * and schema-shape gates.
 */

import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

function changedPaths(): string[] {
  const committed = execFileSync(
    "git",
    ["diff", "--name-only", "origin/main...HEAD"],
    { cwd: repoRoot, encoding: "utf8" },
  );
  const working = execFileSync(
    "git",
    ["status", "--porcelain", "--untracked-files=all"],
    { cwd: repoRoot, encoding: "utf8" },
  )
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => line.slice(3).trim());
  return [...new Set([...committed.split("\n"), ...working])].filter(
    (path) => path.length > 0,
  );
}

describe("NFR-017 non-disruptive manifest evolution", () => {
  // Trace: NFR-017-AC-3
  // Trace: TC-1381
  it("writes nothing into a corpus repository or the retained corpus mirror", () => {
    const paths = changedPaths();
    for (const path of paths) {
      expect(path.startsWith("corpus/"), path).toBe(false);
      expect(path.startsWith(".."), path).toBe(false);
      expect(path.startsWith("/"), path).toBe(false);
    }
    // The pinned config-service copy is the only corpus-derived file, and it
    // lives under tests/fixtures with provenance, never in the source repo.
    const corpusCopies = paths.filter((path) =>
      path.includes("config-service"),
    );
    for (const path of corpusCopies) {
      expect(
        path.startsWith("tests/fixtures/semantic-module/corpus/"),
        path,
      ).toBe(true);
    }
  });

  // Trace: NFR-017-AC-4
  // Trace: TC-1382
  it("keeps package.json and the lockfile untouched", () => {
    const paths = changedPaths();
    expect(paths).not.toContain("package.json");
    expect(paths).not.toContain("pnpm-lock.yaml");
  });
});
