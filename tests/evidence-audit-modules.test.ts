/**
 * #350 B1: ordered audit module selection, banked before implementation.
 *
 * The transport doubles below test the REQUEST only. Since quoin#502 the
 * engine is linked into `quoin-core` rather than spawned as `quire`, so the
 * ordered selection no longer travels as repeated `--module` argv: it is the
 * `modules` array of one `quire.coverage` request. The double therefore stands
 * in for `quoin-core` and delegates every other operation to the real binary,
 * because `evidence audit` asks it four more questions in the same run.
 *
 * The native section runs that real binary against authored criteria; it never
 * fabricates suite results.
 */
import {
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
import * as methodCatalog from "../src/method-catalog.js";
import { coreDouble } from "./support/core-double.js";

let config: Config;
let scratch: string;
let repo: string;
let alpha: string;
let beta: string;

beforeAll(async () => {
  config = await loadConfig({
    root: join(dirname(fileURLToPath(import.meta.url)), ".."),
  });
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
  vi.stubEnv("QUOIN_EXPECTED_CORE_SHA256", "");
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
  rmSync(scratch, { recursive: true, force: true });
});

/** One `quire.coverage` payload, as the boundary shapes it. */
function payload(methods = ["alpha", "beta", "poison"]): object {
  return {
    diagnostics: [],
    obligations: methods.map((method, index) => ({
      id: `FR-001-AC-${index + 1}`,
      statement: `The fixture shall verify ${method}.`,
      statement_hash: "a".repeat(64),
      method,
    })),
  };
}

function transportDouble(refuse = false): () => object[] {
  const log = join(scratch, "requests.jsonl");
  vi.stubEnv(
    "QUOIN_CORE",
    coreDouble({
      answers: { "quire.coverage": JSON.stringify(payload()) },
      requestLog: log,
      ...(refuse
        ? {
            refuseWith:
              "controlled older producer: a closed module set is unsupported",
          }
        : {}),
    }),
  );
  return () =>
    existsSync(log)
      ? readFileSync(log, "utf8")
          .trim()
          .split("\n")
          .map((line) => JSON.parse(line) as object)
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
  // Trace: FR-032-AC-12
  it("forwards both roots in caller order and excludes the ambient catalog", async () => {
    const calls = transportDouble();
    const catalog = vi.spyOn(methodCatalog, "loadMethodCatalog");
    const result = await report([beta, alpha]);
    expect(catalog).toHaveBeenCalledExactlyOnceWith([beta, alpha]);
    expect(calls()).toEqual([{ scope: repo, modules: [beta, alpha] }]);
    expect(
      result.findings.filter((item) => item.kind === "unknown-method"),
    ).toEqual([expect.objectContaining({ obligation: "FR-001-AC-3" })]);
    expect(
      result.findings.filter((item) => item.kind === "undischarged"),
    ).toHaveLength(3);
  });

  // Trace: FR-032-AC-12
  it("preserves single-root catalog and coverage selection", async () => {
    const calls = transportDouble();
    const result = await report([alpha]);
    expect(calls()).toEqual([{ scope: repo, modules: [alpha] }]);
    expect(
      result.findings
        .filter((item) => item.kind === "unknown-method")
        .map((item) => item.obligation),
    ).toEqual(["FR-001-AC-2", "FR-001-AC-3"]);
  });

  // Trace: FR-032-AC-12
  it("preserves ordinary discovery when no module was supplied", async () => {
    const calls = transportDouble();
    const result = await report();
    // Ambient discovery is the ABSENCE of the key, not an empty array: an
    // empty closed set would be "no modules" and derive nothing.
    expect(calls()).toEqual([{ scope: repo }]);
    expect(
      result.findings
        .filter((item) => item.kind === "unknown-method")
        .map((item) => item.obligation),
    ).toEqual(["FR-001-AC-1", "FR-001-AC-2"]);
  });

  // Trace: FR-032-AC-12
  it("surfaces an unsupported producer invocation once, without dropping module arguments", async () => {
    const calls = transportDouble(true);
    await expect(report([alpha, beta])).rejects.toThrow(
      "controlled older producer",
    );
    expect(calls()).toEqual([{ scope: repo, modules: [alpha, beta] }]);
  });
});

describe("native controlled audit module join", () => {
  function nativeFixture(): void {
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

  // Trace: FR-032-AC-12
  it("derives both real populations and consults both catalogs without adding ambient criteria", async () => {
    nativeFixture();
    const result = await report([beta, alpha]);
    // The population is the ANTI-VACUITY floor: the fixture authors three
    // criteria and the closed set admits two of them, so a run that derived
    // nothing would report no findings and read as a clean audit.
    expect(result.findings.map((item) => [item.obligation, item.kind])).toEqual(
      [
        ["FR-001-AC-1", "undischarged"],
        ["NFR-001-AC-1", "undischarged"],
      ],
    );
    expect(result.healthy).toEqual([]);
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });

  // Trace: FR-032-AC-12
  it("retains a healthy real single-module control", async () => {
    nativeFixture();
    const result = await report([alpha]);
    expect(result.findings.map((item) => [item.obligation, item.kind])).toEqual(
      [["FR-001-AC-1", "undischarged"]],
    );
  });

  // Trace: FR-032-AC-12
  it("does not replace an absent supplied root with installed modules", async () => {
    nativeFixture();
    await expect(report([join(scratch, "missing-module")])).rejects.toThrow(
      /manifest|module|directory/i,
    );
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });

  // Trace: FR-032-AC-12
  it("refuses a partially valid explicit set instead of auditing only its readable member", async () => {
    nativeFixture();
    await expect(
      report([alpha, join(scratch, "missing-module")]),
    ).rejects.toThrow("missing-module");
    expect(existsSync(join(repo, "spec", "evidence"))).toBe(false);
  });

  // Trace: FR-032-AC-12
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
