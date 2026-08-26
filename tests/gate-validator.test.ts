import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { beforeAll, describe, expect, it, vi } from "vitest";

import Validate from "../src/commands/validate.js";
import { inspectEmptyGates } from "../src/validators/index.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

function workspace(): string {
  return mkdtempSync(join(tmpdir(), "quoin-gate-validator-"));
}

function write(root: string, path: string, source: string): void {
  const target = join(root, path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, source);
}

function badGate(root: string): void {
  write(root, "Makefile", "gate:\n\t./scripts/check_unwrap.sh\n");
  write(
    root,
    "scripts/check_unwrap.sh",
    [
      "#!/usr/bin/env bash",
      "# Gate for FR-001-AC-1: no production symbol shall call `unwrap`.",
      "set -euo pipefail",
      'grep -rn "unwrap()" src/ | wc -l',
      "exit 0",
      "",
    ].join("\n"),
  );
}

describe("gate capability validator (agent-ix/quoin#224)", () => {
  it("TC-1067 joins a gate claim, wiring, and an unasserted count at an exact locus", () => {
    const root = workspace();
    badGate(root);
    expect(inspectEmptyGates(root)).toEqual([
      expect.objectContaining({
        kind: "gate-that-gates-nothing",
        obligation: "FR-001-AC-1",
        path: "scripts/check_unwrap.sh",
        line: 4,
        wiredBy: "Makefile",
      }),
    ]);
    expect(inspectEmptyGates(root)[0].summary).toContain(
      "compare the count to zero",
    );
  });

  it("TC-1068 ignores an unwired report with identical shell text", () => {
    const root = workspace();
    badGate(root);
    write(root, "Makefile", "report:\n\t@echo report only\n");
    expect(inspectEmptyGates(root)).toEqual([]);
  });

  it("TC-1069 keeps a wired gate with an explicit failure path silent", () => {
    const root = workspace();
    write(root, "Makefile", "gate:\n\t./scripts/check_unwrap.sh\n");
    write(
      root,
      "scripts/check_unwrap.sh",
      [
        "#!/usr/bin/env bash",
        "# Gate for FR-001-AC-1: no production symbol shall call `unwrap`.",
        'if grep -rn "unwrap()" src/; then',
        "  exit 1",
        "fi",
        "",
      ].join("\n"),
    );
    expect(inspectEmptyGates(root)).toEqual([]);
  });

  it("TC-1070 ignores a wired count script that makes no gate claim", () => {
    const root = workspace();
    write(root, "Makefile", "report:\n\t./scripts/count_unwrap.sh\n");
    write(
      root,
      "scripts/count_unwrap.sh",
      '#!/bin/sh\ngrep -rn "unwrap()" src/ | wc -l\n',
    );
    expect(inspectEmptyGates(root)).toEqual([]);
  });

  it("TC-1071 reports the same finding through the shipped quoin validate command", async () => {
    const root = workspace();
    badGate(root);
    const lines: string[] = [];
    const spy = vi.spyOn(console, "log").mockImplementation((line) => {
      lines.push(String(line));
    });
    try {
      await Validate.run(["--repo", root, "--json"], config);
    } finally {
      spy.mockRestore();
    }
    const payload = JSON.parse(lines.join("\n")) as { findings: unknown[] };
    expect(payload.findings).toEqual([
      expect.objectContaining({ kind: "gate-that-gates-nothing", line: 4 }),
    ]);

    const human: string[] = [];
    const humanSpy = vi.spyOn(console, "log").mockImplementation((line) => {
      human.push(String(line));
    });
    try {
      await expect(
        Validate.run(["--repo", root], config),
      ).resolves.toBeUndefined();
      expect(human.join("\n")).toContain("scripts/check_unwrap.sh:4");
      await expect(
        Validate.run(["--repo", root, "--strict"], config),
      ).rejects.toMatchObject({ oclif: { exit: 1 } });
    } finally {
      humanSpy.mockRestore();
    }
  });
});
