/**
 * The retained gate-capability validator (agent-ix/quoin#224).
 *
 * `quoin validate` no longer calls this code: it asks `quoin-core` for
 * `validators.run` (quoin#412). What is left here covers the RETAINED
 * TypeScript while it is still the difftest oracle, and it is retired with it.
 *
 * The command-level criteria this file used to carry moved to
 * `tests/core-exec-e2e.test.ts`, which is the only lane with a built
 * `quoin-core` to ask; asserting them here would now assert nothing about the
 * shipped command.
 */

import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { describe, expect, it } from "vitest";

import { inspectEmptyGates } from "../src/validators/index.js";

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
  it("joins a gate claim, wiring, and an unasserted count at an exact locus", () => {
    const root = workspace();
    badGate(root);
    expect(inspectEmptyGates(root)).toEqual([
      expect.objectContaining({
        kind: "gate-that-gates-nothing",
        obligation: "FR-001-AC-1",
        path: "scripts/check_unwrap.sh",
        line: 4,
        wiredBy: "Makefile",
        subject: "gate for FR-001-AC-1",
        changeTarget: "scripts/check_unwrap.sh:4",
        remedy: expect.stringContaining("compare the count"),
      }),
    ]);
    expect(inspectEmptyGates(root)[0].summary).toContain(
      "compare the count to zero",
    );
  });

  it("ignores an unwired report with identical shell text", () => {
    const root = workspace();
    badGate(root);
    write(root, "Makefile", "report:\n\t@echo report only\n");
    expect(inspectEmptyGates(root)).toEqual([]);
  });

  it("keeps a wired gate with an explicit failure path silent", () => {
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

  it("ignores a wired count script that makes no gate claim", () => {
    const root = workspace();
    write(root, "Makefile", "report:\n\t./scripts/count_unwrap.sh\n");
    write(
      root,
      "scripts/count_unwrap.sh",
      '#!/bin/sh\ngrep -rn "unwrap()" src/ | wc -l\n',
    );
    expect(inspectEmptyGates(root)).toEqual([]);
  });
});
