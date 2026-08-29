/**
 * `quoin evidence audit --ratchet` reports whether it actually ratcheted
 * (FR-032-AC-13, #169, TC-258..TC-260).
 *
 * The label and the JSON `ratchet` field were keyed on the FLAG, not on
 * whether a baseline was found. A missing baseline silently degrades the run
 * to a full report — reasonable — but a day-one reader then saw their entire
 * backlog printed under "(new violations only)" and concluded either that the
 * ratchet was broken or that they had just introduced 33 problems.
 *
 * quire is faked on PATH (the tests/quire-exec.test.ts pattern): the command
 * needs a version answer and one coverage payload, and what is under test is
 * the reporting, not the engine.
 */

import { execFileSync } from "node:child_process";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import EvidenceAudit from "../src/commands/evidence/audit";
import EvidenceInspectMocks from "../src/commands/evidence/inspect-mocks";
import {
  baselinePath,
  writeBaseline,
  writeBindings,
  writeRun,
} from "../src/evidence/index.js";
import { STORE_SCHEMA_VERSION } from "../src/evidence/types.js";
import { packageVersion } from "../src/version.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

/** One obligation, no bindings: the audit reports it undischarged. */
const COVERAGE_PAYLOAD = JSON.stringify({
  unbacked_rows: [],
  status_lies: [],
  untracked_symbols: [],
  groups: [],
  totals: { backed: 0, total: 1 },
  obligations: [
    {
      source: "acceptance-criteria",
      id: "FR-001-AC-1",
      document: "spec/functional/FR-001.md",
      statement: "Every finding defaults to warning.",
      statement_hash: "a".repeat(64),
    },
  ],
});

/** A fake `quire` answering `--version` and `coverage --json`, first on PATH. */
function fakeQuireDir(payload: string = COVERAGE_PAYLOAD): string {
  const dir = mkdtempSync(join(tmpdir(), "quoin-fake-quire-"));
  const bin = join(dir, "quire");
  writeFileSync(
    bin,
    [
      "#!/bin/sh",
      'if [ "$1" = "--version" ]; then echo "quire 0.41.0"; exit 0; fi',
      `cat <<'PAYLOAD'`,
      payload,
      "PAYLOAD",
    ].join("\n"),
  );
  chmodSync(bin, 0o755);
  return dir;
}

function captureLog(): { lines: string[]; restore: () => void } {
  const lines: string[] = [];
  const spy = vi.spyOn(console, "log").mockImplementation((message) => {
    lines.push(String(message));
  });
  return { lines, restore: () => spy.mockRestore() };
}

describe("evidence audit --ratchet with and without a baseline (FR-032-AC-13)", () => {
  const savedPath = process.env.PATH;
  afterEach(() => {
    process.env.PATH = savedPath;
  });

  function workspace(): string {
    process.env.PATH = `${fakeQuireDir()}:${savedPath}`;
    return mkdtempSync(join(tmpdir(), "quoin-audit-"));
  }

  // TC-258
  it("day one: names the missing baseline and does not claim '(new violations only)'", async () => {
    const root = workspace();
    const { lines, restore } = captureLog();
    try {
      await EvidenceAudit.run(["--repo", root, "--ratchet"], config);
    } finally {
      restore();
    }
    const output = lines.join("\n");
    expect(output).toContain(baselinePath(root));
    expect(output).toContain("quoin evidence baseline");
    expect(output).toContain("undischarged");
    expect(output).not.toContain("(new violations only)");
  });

  // TC-259
  it("with a baseline present, ratchets and says so — and prints no missing-baseline notice", async () => {
    const root = workspace();
    // An EMPTY accepted set: everything the audit finds is genuinely new, so
    // the label is truthful and the full-report path stays distinguishable.
    writeBaseline(root, {
      schemaVersion: STORE_SCHEMA_VERSION,
      commit: "b".repeat(40),
      accepted: [],
    });
    const { lines, restore } = captureLog();
    try {
      await EvidenceAudit.run(["--repo", root, "--ratchet"], config);
    } finally {
      restore();
    }
    const output = lines.join("\n");
    expect(output).toContain("(new violations only)");
    expect(output).not.toContain("does not exist");
  });

  // TC-260
  it("JSON `ratchet` reflects whether ratcheting was applied, not what was asked for", async () => {
    const root = workspace();
    const bare = captureLog();
    try {
      await EvidenceAudit.run(["--repo", root, "--ratchet", "--json"], config);
    } finally {
      bare.restore();
    }
    // The notice is suppressed under --json, so stdout stays one parseable
    // payload for the machine consumer the field exists for.
    const withoutBaseline = JSON.parse(bare.lines.join("\n")) as {
      ratchet: boolean;
      findings: unknown[];
    };
    expect(withoutBaseline.ratchet).toBe(false);
    expect(withoutBaseline.findings).toHaveLength(1);

    writeBaseline(root, {
      schemaVersion: STORE_SCHEMA_VERSION,
      commit: "b".repeat(40),
      accepted: ["undischarged:FR-001-AC-1"],
    });
    const ratcheted = captureLog();
    try {
      await EvidenceAudit.run(["--repo", root, "--ratchet", "--json"], config);
    } finally {
      ratcheted.restore();
    }
    const withBaseline = JSON.parse(ratcheted.lines.join("\n")) as {
      ratchet: boolean;
      findings: unknown[];
    };
    expect(withBaseline.ratchet).toBe(true);
    // And the accepted finding is ratcheted away, so the two runs differ in
    // exactly the way the field claims.
    expect(withBaseline.findings).toHaveLength(0);
  });
});

describe("TC-265 audit reports uncatalogued methods with no evidence store at all (FR-032-AC-14)", () => {
  const savedPath = process.env.PATH;
  afterEach(() => {
    process.env.PATH = savedPath;
  });

  /** The day-one shape (#165): a Verification value in no catalog, no store. */
  const UNCATALOGUED_PAYLOAD = JSON.stringify({
    unbacked_rows: [],
    status_lies: [],
    untracked_symbols: [],
    groups: [],
    totals: { backed: 0, total: 1 },
    obligations: [
      {
        source: "acceptance-criteria",
        id: "NFR-022-M-12",
        document: "spec/non-functional/NFR-022.md",
        statement: "PR-tier CI wall clock stays under budget.",
        statement_hash: "a".repeat(64),
        method: "CI Measurement",
      },
    ],
  });

  /** A module whose catalog does NOT carry `CI Measurement`. */
  function moduleDir(): string {
    const dir = mkdtempSync(join(tmpdir(), "quoin-tc265-module-"));
    writeFileSync(
      join(dir, "manifest.yaml"),
      [
        "manifest_version: 1",
        "name: tc265-fixture",
        "version: 0.0.0",
        "verification_catalog:",
        "  unit-testing:",
        "    verification_class: Test",
        "    evidence_kind: Unit",
        "    summary: the only method this catalog declares",
        "",
      ].join("\n"),
    );
    return dir;
  }

  // TC-265
  it("an unadopted repository hears about its uncatalogued methods, not only that nothing is bound", async () => {
    // Before #165 this run reported the obligation ONLY as undischarged: the
    // unknown-method check lived past the binding guard, so the one check
    // that pays off before any evidence exists required evidence to run.
    process.env.PATH = `${fakeQuireDir(UNCATALOGUED_PAYLOAD)}:${savedPath}`;
    const root = mkdtempSync(join(tmpdir(), "quoin-audit-"));
    const { lines, restore } = captureLog();
    try {
      await EvidenceAudit.run(
        ["--repo", root, "--module", moduleDir()],
        config,
      );
    } finally {
      restore();
    }
    const output = lines.join("\n");
    expect(output).toContain("undischarged");
    expect(output).toContain("unknown-method");
    expect(output).toContain("CI Measurement");
  });
});

describe("mocked-confirmation production command path (agent-ix/quoin#204)", () => {
  const savedPath = process.env.PATH;
  afterEach(() => {
    process.env.PATH = savedPath;
  });

  it("TC-1065 TC-1124 inspect-mocks records its version and audit reports a located finding", async () => {
    process.env.PATH = `${fakeQuireDir(
      JSON.stringify({
        unbacked_rows: [],
        status_lies: [],
        untracked_symbols: [],
        groups: [],
        totals: { backed: 1, total: 1 },
        obligations: [
          {
            source: "acceptance-criteria",
            id: "FR-001-AC-1",
            document: "spec/FR-001.md",
            statement:
              "The shell shall obtain the user's confirmation before granting a root.",
            statement_hash: "a".repeat(64),
          },
        ],
      }),
    )}:${savedPath}`;
    const root = mkdtempSync(join(tmpdir(), "quoin-mock-command-"));
    mkdirSync(join(root, "src"), { recursive: true });
    writeFileSync(
      join(root, "src", "lib.rs"),
      `#[cfg(test)] mod tests {\n#[test]\nfn confirms() {\n  assert!(grant_root(Confirmation::allow()));\n}\n}`,
    );
    execFileSync("git", ["init", "-q", root]);
    execFileSync("git", [
      "-C",
      root,
      "config",
      "user.email",
      "test@example.com",
    ]);
    execFileSync("git", ["-C", root, "config", "user.name", "Test"]);
    execFileSync("git", ["-C", root, "add", "src/lib.rs"]);
    execFileSync("git", ["-C", root, "commit", "-qm", "fixture"]);
    const commit = execFileSync("git", ["-C", root, "rev-parse", "HEAD"], {
      encoding: "utf8",
    }).trim();

    writeRun(root, {
      schemaVersion: 1,
      suite: "SUITE-001",
      commit,
      tool: "cargo test",
      timestamp: "2026-08-26T00:00:00Z",
      entries: [{ symbol: "confirms", outcome: "pass" }],
    });
    writeBindings(root, {
      bindings: [
        {
          obligation: "FR-001-AC-1",
          statementHashAtBinding: "a".repeat(64),
          suite: "SUITE-001",
          commit,
          symbols: ["confirms"],
        },
      ],
    });

    await EvidenceInspectMocks.run(
      [
        "--repo",
        root,
        "--suite",
        "SUITE-001",
        "--commit",
        commit,
        "--timestamp",
        "2026-08-26T00:00:00Z",
        "--json",
      ],
      config,
    );
    const inspectionPath = join(
      root,
      "spec",
      "evidence",
      "mock-inspections",
      "SUITE-001",
      `${commit.slice(0, 12)}.json`,
    );
    expect(JSON.parse(readFileSync(inspectionPath, "utf8"))).toMatchObject({
      tool: `quoin mock-inspection ${packageVersion()}`,
    });
    const output = captureLog();
    try {
      await EvidenceAudit.run(["--repo", root, "--json"], config);
    } finally {
      output.restore();
    }
    const report = JSON.parse(output.lines.join("\n")) as {
      findings: Array<{
        kind: string;
        summary: string;
        subject?: string;
        changeTarget?: string;
        nextDiagnosticStep?: string;
      }>;
      unevaluated: unknown[];
    };
    expect(report.unevaluated).toEqual([]);
    expect(report.findings).toEqual([
      expect.objectContaining({
        kind: "mocked-confirmation",
        summary: expect.stringContaining(
          "src/lib.rs:4 confirms injects Confirmation::allow",
        ),
        subject: "FR-001-AC-1 evidence",
        changeTarget: "src/lib.rs:4 in confirms",
        nextDiagnosticStep: expect.stringContaining(
          "add or bind an independent test",
        ),
      }),
    ]);
  });
});
