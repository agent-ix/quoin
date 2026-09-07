/**
 * #350 B1: ordered audit module selection, banked before implementation.
 * Transport doubles below test argv only. The native section executes the real
 * selected Quire against authored criteria; it never fabricates suite results.
 */
import { execFileSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import {
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";

import EvidenceAudit from "../src/commands/evidence/audit.js";
import * as advisor from "../src/advisor/index.js";
import { quireExecutable } from "../src/quire/exec.js";

let config: Config;
let nativeQuire: string;
let scratch: string;
let repo: string;
let alpha: string;
let beta: string;

beforeAll(async () => {
  config = await loadConfig({
    root: join(dirname(fileURLToPath(import.meta.url)), ".."),
  });
  // The normal make test-with-quire gate selects this on PATH. Direct focused
  // runs can use QUOIN_QUIRE; absence is a failed prerequisite, not a skip.
  nativeQuire = quireExecutable();
});

function moduleAt(root: string, name: string, type: string): string {
  mkdirSync(root, { recursive: true });
  writeFileSync(
    join(root, "manifest.yaml"),
    `manifest_version: 1.0.0
name: ${name}
version: 0.0.0
artifact_types:
  - name: ${type}
traceability:
  trace_targets:
    - name: ${name}-criterion
      archetype: ${type}
      section: Acceptance Criteria
      id_column: ID
  obligations:
    - name: ${name}-criterion
      target: ${name}-criterion
      statement_column: Criteria
      method_column: Verification
verification_catalog:
  ${name}:
    name: ${name}
    class: Test
    definition: Execute the controlled ${name} check.
    evidence_kind: Unit
`,
  );
  return root;
}

beforeEach(() => {
  scratch = mkdtempSync(join(tmpdir(), "quoin-audit-modules-"));
  repo = join(scratch, "repository");
  mkdirSync(repo);
  alpha = moduleAt(join(scratch, "alpha with spaces"), "alpha", "FR");
  beta = moduleAt(join(scratch, "beta"), "beta", "NFR");
  const installed = moduleAt(
    join(scratch, "home", "filament", "modules", "poison"),
    "poison",
    "StR",
  );
  vi.stubEnv("IX_HOME", join(scratch, "home"));
  vi.stubEnv("QUOIN_MODULE_PATHS", installed);
  vi.stubEnv("IX_FILAMENT_MODULES_PATH", installed);
  vi.stubEnv("QUOIN_EXPECTED_QUIRE_SHA256", "");
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
  rmSync(scratch, { recursive: true, force: true });
});

function payload(methods = ["alpha", "beta", "poison"]): object {
  return {
    unbacked_rows: [],
    status_lies: [],
    untracked_symbols: [],
    groups: [],
    totals: { backed: 0, total: methods.length },
    obligations: methods.map((method, index) => ({
      source: "controlled-criterion",
      id: `FR-001-AC-${index + 1}`,
      document: "spec/FR-001.md",
      statement: `The fixture shall verify ${method}.`,
      statement_hash: "a".repeat(64),
      method,
    })),
  };
}

function transportDouble(refuse = false): () => string[][] {
  const executable = join(scratch, "quire-transport-double");
  const log = join(scratch, "arguments.jsonl");
  writeFileSync(
    executable,
    `#!${process.execPath}
const fs = require("node:fs");
const args = process.argv.slice(2);
if (args[0] === "--version") { console.log("quire 0.31.0"); process.exit(0); }
fs.appendFileSync(${JSON.stringify(log)}, JSON.stringify(args) + "\\n");
if (${JSON.stringify(refuse)}) {
  console.error("controlled older producer: repeated --module is unsupported");
  process.exit(2);
}
console.log(${JSON.stringify(JSON.stringify(payload()))});
`,
  );
  chmodSync(executable, 0o755);
  vi.stubEnv("QUOIN_QUIRE", executable);
  return () =>
    existsSync(log)
      ? readFileSync(log, "utf8")
          .trim()
          .split("\n")
          .map((line) => JSON.parse(line))
      : [];
}

async function report(modules: string[] = []) {
  const lines: string[] = [];
  vi.spyOn(console, "log").mockImplementation((line) =>
    lines.push(String(line)),
  );
  await EvidenceAudit.run(
    [
      "--repo",
      repo,
      "--json",
      ...modules.flatMap((root) => ["--module", root]),
    ],
    config,
  );
  return JSON.parse(lines.join("\n")) as {
    findings: Array<{ kind: string; obligation: string }>;
    healthy: unknown[];
  };
}

describe("ordered audit module transport", () => {
  // Trace: TC-1598, FR-032-AC-12
  it("forwards both roots in caller order and excludes the ambient catalog", async () => {
    const calls = transportDouble();
    const catalog = vi.spyOn(advisor, "loadMethodCatalog");
    const result = await report([beta, alpha]);
    expect(catalog).toHaveBeenCalledExactlyOnceWith([beta, alpha]);
    expect(calls()).toEqual([
      [
        "coverage",
        "--scope",
        repo,
        "--json",
        "--module",
        beta,
        "--module",
        alpha,
      ],
    ]);
    expect(
      result.findings.filter((item) => item.kind === "unknown-method"),
    ).toEqual([expect.objectContaining({ obligation: "FR-001-AC-3" })]);
    expect(
      result.findings.filter((item) => item.kind === "undischarged"),
    ).toHaveLength(3);
  });

  // Trace: TC-1599, FR-032-AC-12
  it("preserves single-root catalog and coverage selection", async () => {
    const calls = transportDouble();
    const result = await report([alpha]);
    expect(calls()).toEqual([
      ["coverage", "--scope", repo, "--json", "--module", alpha],
    ]);
    expect(
      result.findings
        .filter((item) => item.kind === "unknown-method")
        .map((item) => item.obligation),
    ).toEqual(["FR-001-AC-2", "FR-001-AC-3"]);
  });

  // Trace: TC-1599, FR-032-AC-12
  it("preserves ordinary discovery when no module was supplied", async () => {
    const calls = transportDouble();
    const result = await report();
    expect(calls()).toEqual([["coverage", "--scope", repo, "--json"]]);
    expect(
      result.findings
        .filter((item) => item.kind === "unknown-method")
        .map((item) => item.obligation),
    ).toEqual(["FR-001-AC-1", "FR-001-AC-2"]);
  });

  // Trace: TC-1599, FR-032-AC-12
  it("surfaces an unsupported producer invocation once, without dropping module arguments", async () => {
    const calls = transportDouble(true);
    await expect(report([alpha, beta])).rejects.toThrow(
      "controlled older producer",
    );
    expect(calls()).toEqual([
      [
        "coverage",
        "--scope",
        repo,
        "--json",
        "--module",
        alpha,
        "--module",
        beta,
      ],
    ]);
  });
});

describe("native controlled audit module join", () => {
  function nativeFixture(): void {
    vi.stubEnv("QUOIN_QUIRE", nativeQuire);
    mkdirSync(join(repo, "spec"));
    for (const [type, method] of [
      ["FR", "alpha"],
      ["NFR", "beta"],
      ["StR", "poison"],
    ]) {
      writeFileSync(
        join(repo, "spec", `${type}-001.md`),
        `---
id: ${type}-001
type: ${type}
title: Controlled ${method} criterion
---

# ${type}-001: Controlled ${method} criterion

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| ${type}-001-AC-1 | The fixture shall verify ${method}. | ${method} |
`,
      );
    }
  }

  // Trace: TC-1600, FR-032-AC-12
  it("derives both real populations and consults both catalogs without adding ambient criteria", async () => {
    nativeFixture();
    const native = JSON.parse(
      execFileSync(
        nativeQuire,
        [
          "coverage",
          "--scope",
          repo,
          "--json",
          "--module",
          beta,
          "--module",
          alpha,
        ],
        { encoding: "utf8" },
      ),
    );
    expect(
      native.obligations.map((item: { id: string }) => item.id).sort(),
    ).toEqual(["FR-001-AC-1", "NFR-001-AC-1"]);
    const result = await report([beta, alpha]);
    expect(result.findings.map((item) => [item.obligation, item.kind])).toEqual(
      [
        ["FR-001-AC-1", "undischarged"],
        ["NFR-001-AC-1", "undischarged"],
      ],
    );
    expect(result.healthy).toEqual([]);
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });

  // Trace: TC-1600, FR-032-AC-12
  it("retains a healthy real single-module control", async () => {
    nativeFixture();
    const result = await report([alpha]);
    expect(result.findings.map((item) => [item.obligation, item.kind])).toEqual(
      [["FR-001-AC-1", "undischarged"]],
    );
  });

  // Trace: TC-1600, FR-032-AC-12
  it("does not replace an absent supplied root with installed modules", async () => {
    nativeFixture();
    await expect(report([join(scratch, "missing-module")])).rejects.toThrow(
      /manifest|module|directory/i,
    );
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });

  // Trace: TC-1600, FR-032-AC-12
  it("refuses a partially valid explicit set instead of auditing only its readable member", async () => {
    nativeFixture();
    await expect(
      report([alpha, join(scratch, "missing-module")]),
    ).rejects.toThrow("missing-module");
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });

  // Trace: TC-1600, FR-032-AC-12
  it("does not replace a malformed supplied module with installed modules", async () => {
    nativeFixture();
    writeFileSync(
      join(alpha, "manifest.yaml"),
      "traceability: [unterminated\n",
    );
    await expect(report([alpha])).rejects.toThrow(/manifest|YAML|parse/i);
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });
});
