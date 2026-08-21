/**
 * `quoin evidence gc` at the command level (FR-030-AC-8, #171, TC-263).
 *
 * The `gc()` store function was exercised (TC-126, TC-127) and the command
 * wrapping it was not, so the flag surface itself was unasserted. That surface
 * is a decision: gc takes no `--module` where every sibling does, because
 * `--module` supplies the traceability model to quire and gc never invokes
 * quire — it is a repo-scoped store operation. The flag-set assertion below
 * pins that decision the way tests/advise-command.test.ts pins advise's.
 */

import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { beforeAll, describe, expect, it, vi } from "vitest";

import EvidenceGc from "../src/commands/evidence/gc";
import { runPath, writeRun } from "../src/evidence/index.js";
import { STORE_SCHEMA_VERSION } from "../src/evidence/types.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

/** Two runs of one suite: gc retains the newest and deletes the other. */
function seededRepo(): string {
  const repo = mkdtempSync(join(tmpdir(), "quoin-gc-cmd-"));
  const base = {
    schemaVersion: STORE_SCHEMA_VERSION,
    suite: "SUITE-001",
    tool: "t",
    entries: [],
  };
  writeRun(repo, {
    ...base,
    commit: "a".repeat(16),
    timestamp: "2026-08-17T00:00:00Z",
  });
  writeRun(repo, {
    ...base,
    commit: "b".repeat(16),
    timestamp: "2026-08-18T00:00:00Z",
  });
  return repo;
}

function captureLog(): { lines: string[]; restore: () => void } {
  const lines: string[] = [];
  const spy = vi.spyOn(console, "log").mockImplementation((message) => {
    lines.push(String(message));
  });
  return { lines, restore: () => spy.mockRestore() };
}

describe("TC-263 evidence gc through the command (FR-030-AC-8)", () => {
  // TC-263
  it("declares exactly repo, dry-run and json — no --module, deliberately", () => {
    // gc() never shells to quire and never loads a catalog, so --module would
    // be a no-op accepted only for symmetry — a flag that reads as doing
    // something. The absence is the decision #171 confirmed; a new flag here
    // must arrive with semantics, and this fails until it does.
    expect(Object.keys(EvidenceGc.flags).sort()).toEqual([
      "dry-run",
      "json",
      "repo",
    ]);
    // And the help says WHY, so the user who typed --module on five siblings
    // learns the reason rather than just the unknown-flag error.
    expect(EvidenceGc.description).toContain("Takes no --module");
    expect(EvidenceGc.description).toContain("never invokes quire");
  });

  // TC-263
  it("--dry-run --json lists what would go and deletes nothing", async () => {
    const repo = seededRepo();
    const older = runPath(repo, "SUITE-001", "a".repeat(16));
    const { lines, restore } = captureLog();
    try {
      await EvidenceGc.run(["--repo", repo, "--dry-run", "--json"], config);
    } finally {
      restore();
    }
    const payload = JSON.parse(lines.join("\n")) as {
      deleted: string[];
      dryRun: boolean;
    };
    expect(payload.dryRun).toBe(true);
    expect(payload.deleted).toHaveLength(1);
    expect(payload.deleted[0]).toContain("aaaaaaaaaaaa.json");
    expect(existsSync(older)).toBe(true);
  });

  // TC-263
  it("deletes the unreferenced older run and reports each path", async () => {
    const repo = seededRepo();
    const older = runPath(repo, "SUITE-001", "a".repeat(16));
    const newest = runPath(repo, "SUITE-001", "b".repeat(16));
    const { lines, restore } = captureLog();
    try {
      await EvidenceGc.run(["--repo", repo], config);
    } finally {
      restore();
    }
    expect(lines.join("\n")).toContain(`deleted ${older}`);
    expect(existsSync(older)).toBe(false);
    expect(existsSync(newest)).toBe(true);
  });

  // TC-263
  it("says 'nothing to collect' over an absent store", async () => {
    const repo = mkdtempSync(join(tmpdir(), "quoin-gc-empty-"));
    const { lines, restore } = captureLog();
    try {
      await EvidenceGc.run(["--repo", repo], config);
    } finally {
      restore();
    }
    expect(lines.join("\n")).toContain("nothing to collect");
  });
});
