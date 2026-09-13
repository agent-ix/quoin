/**
 * `src/core/exec.ts` against the REAL `quoin-core` binary (quoin#375, FR-096).
 *
 * `tests/core-exec.test.ts` proves the caller's failure contract with fake
 * binaries, which is the only way to produce an ENOBUFS death or a SIGTERM on
 * demand. It cannot prove that the two sides agree about the thing they were
 * built for, because a fake agrees with whatever the test wrote into it.
 *
 * This file closes that gap: one operation, end to end, over a real pipe.
 *
 * It **skips cleanly** when `QUOIN_CORE` is unset, because a plain `vitest run`
 * has no Rust build. `make rust-e2e` builds the workspace and sets it; that is
 * the lane this test belongs to, and `make rust-gate` runs it.
 */

import {
  accessSync,
  constants,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadConfig } from "@agent-ix/ix-cli-core";
import { describe, expect, it, vi } from "vitest";

import Validate from "../src/commands/validate.js";
import { CORE_EXIT, runCore, runCoreAllowFailure } from "../src/core/index.js";
import { repoSnapshot } from "../src/core/snapshot.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

function binary(): string | null {
  const path = process.env.QUOIN_CORE;
  if (!path) return null;
  try {
    accessSync(path, constants.X_OK);
    return path;
  } catch {
    return null;
  }
}

const available = binary() !== null;

describe.skipIf(!available)("src/core/exec.ts ↔ quoin-core", () => {
  it("round trips core.ping", () => {
    expect(runCore("core.ping", { echo: "corr-7" })).toEqual({
      core_version: "0.1.0",
      echo: "corr-7",
      protocol_version: 1,
    });
  });

  it("returns the payload of an exit-1 run and its diagnostic", () => {
    // The contract that cannot be checked on either side alone: quoin-core
    // decides this is exit 1 with a payload, and src/core/exec.ts must read it
    // as one. A disagreement here is the #103 defect reconstituted.
    const result = runCoreAllowFailure("core.ping", { expect_protocol: 99 });
    expect(result.exitCode).toBe(CORE_EXIT.PARTIAL);
    expect(result.ok).toBe(false);
    expect(result.payload).toEqual({
      core_version: "0.1.0",
      echo: null,
      protocol_version: 1,
    });
    expect(result.diagnostics[0].code).toBe("CORE_PROTOCOL_SKEW");
    expect(result.diagnostics[0].context.actual).toBe("1");
  });

  it("reports quoin-core's refusal as a refusal with no payload", () => {
    const result = runCoreAllowFailure("core.ping", {
      echo: "x".repeat(4 * 1024 + 1),
    });
    expect(result.exitCode).toBe(CORE_EXIT.REFUSED);
    expect(result.payload).toBeNull();
    expect(result.diagnostics[0].code).toBe("CORE_REFUSED");
  });

  it("throws on an operation quoin-core does not implement", () => {
    expect(() => runCore("evidence.record", {})).toThrow(/CORE_UNKNOWN_OP/);
  });

  it("accepts the real binary under its own pinned digest", () => {
    // The bytes-pinning path, exercised against bytes that actually exist —
    // tests/core-exec.test.ts can only pin a shell script.
    const path = binary() as string;
    const digest = createHash("sha256")
      .update(readFileSync(path))
      .digest("hex");
    const saved = process.env.QUOIN_EXPECTED_CORE_SHA256;
    process.env.QUOIN_EXPECTED_CORE_SHA256 = `sha256:${digest}`;
    try {
      expect(runCore("core.ping", {})).toMatchObject({ protocol_version: 1 });
      process.env.QUOIN_EXPECTED_CORE_SHA256 = `sha256:${"0".repeat(64)}`;
      expect(() => runCore("core.ping", {})).toThrow(/digest mismatch/);
    } finally {
      if (saved === undefined) delete process.env.QUOIN_EXPECTED_CORE_SHA256;
      else process.env.QUOIN_EXPECTED_CORE_SHA256 = saved;
    }
  });
});

/**
 * `quoin validate` over the real boundary (quoin#412, FR-101).
 *
 * `tests/gate-validator.test.ts` used to assert this against the TypeScript
 * implementation that has now been retired. The criteria it carried about the
 * ANALYSIS are restated in Rust — `quoin-validators`' golden corpus and
 * `quoin-core`'s `tc_412_*` boundary tests. The criteria it carried about the
 * COMMAND cannot move there, because what is asserted is that the shipped oclif
 * command still prints what it always printed; they live here, in the one lane
 * that has a built `quoin-core` to talk to.
 */
describe.skipIf(!available)("quoin validate \u2194 quoin-core", () => {
  /** The TC-1067 fixture: a claim, its wiring, and an unasserted count. */
  function badGate(root: string): void {
    const write = (path: string, source: string): void => {
      const target = join(root, path);
      mkdirSync(dirname(target), { recursive: true });
      writeFileSync(target, source);
    };
    write("Makefile", "gate:\n\t./scripts/check_unwrap.sh\n");
    write(
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

  async function run(argv: string[]): Promise<string[]> {
    const lines: string[] = [];
    const spy = vi.spyOn(console, "log").mockImplementation((line) => {
      lines.push(String(line));
    });
    try {
      await Validate.run(argv, await loadConfig({ root: repoRoot }));
    } finally {
      spy.mockRestore();
    }
    return lines;
  }

  it("reports the finding at an exact locus, as JSON and as prose", async () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-validate-e2e-"));
    badGate(root);

    const payload = JSON.parse(
      (await run(["--repo", root, "--json"])).join("\n"),
    ) as { findings: Record<string, unknown>[] };
    expect(payload.findings).toEqual([
      expect.objectContaining({
        changeTarget: "scripts/check_unwrap.sh:4",
        kind: "gate-that-gates-nothing",
        line: 4,
        obligation: "FR-001-AC-1",
        path: "scripts/check_unwrap.sh",
        remedy: expect.stringContaining("exit non-zero"),
        subject: "gate for FR-001-AC-1",
        wiredBy: "Makefile",
      }),
    ]);
    const summary = String(payload.findings[0].summary);
    expect(summary).toContain("compare the count to zero");

    const human = await run(["--repo", root]);
    expect(human).toEqual([
      `[warning] gate-that-gates-nothing: scripts/check_unwrap.sh:4: ${summary}`,
      "1 gate finding(s)",
    ]);
  });

  /**
   * The one place the cutover DID change the bytes, pinned so it is a decision
   * and not an accident.
   *
   * The prose lines and every value are identical to what `src/validators/`
   * printed. The `--json` document's KEY ORDER is not: the payload is now
   * parsed out of `quoin-core`'s stdout, which is canonical JSON with object
   * keys sorted at every depth (rust-style, "the boundary contract"), and
   * `JSON.stringify` preserves that parse order. The retained TypeScript built
   * the object literal in `kind, obligation, path, line, wiredBy, subject,
   * changeTarget, remedy, summary` order.
   *
   * Restoring the old order would mean hand-writing a field list in TypeScript
   * for a type FR-097 says TypeScript never declares, so the new order is the
   * contract. It is asserted here rather than left implicit because a consumer
   * diffing `quoin validate --json` output byte for byte sees it.
   */
  it("emits the payload in canonical key order", async () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-validate-e2e-"));
    badGate(root);
    const payload = JSON.parse(
      (await run(["--repo", root, "--json"])).join("\n"),
    ) as { findings: Record<string, unknown>[] };
    expect(Object.keys(payload.findings[0])).toEqual([
      "changeTarget",
      "kind",
      "line",
      "obligation",
      "path",
      "remedy",
      "subject",
      "summary",
      "wiredBy",
    ]);
  });

  it("says so when there is nothing to report", async () => {
    // TC-1068: identical shell text with no wiring is a report, not a gate.
    const root = mkdtempSync(join(tmpdir(), "quoin-validate-e2e-"));
    badGate(root);
    writeFileSync(join(root, "Makefile"), "report:\n\t@echo report only\n");
    expect(await run(["--repo", root])).toEqual([
      "repository QA gates: no findings",
    ]);
  });

  it("makes --strict the caller's policy, not the boundary's verdict", async () => {
    // A finding is a SUCCESSFUL answer: quoin-core exits 0 and the payload
    // carries the findings. The exit 1 below is this command's own decision,
    // which is why a clean repository under --strict still resolves.
    const root = mkdtempSync(join(tmpdir(), "quoin-validate-e2e-"));
    badGate(root);
    await expect(run(["--repo", root, "--strict"])).rejects.toMatchObject({
      oclif: { exit: 1 },
    });

    const clean = mkdtempSync(join(tmpdir(), "quoin-validate-e2e-"));
    await expect(run(["--repo", clean, "--strict"])).resolves.toEqual([
      "repository QA gates: no findings",
    ]);
  });
});

/**
 * The superset property, measured rather than enumerated (quoin#412).
 *
 * `src/core/snapshot.ts` is sound only if it is a strict SUPERSET of what
 * `quoin-validators` classifies: a path it drops that `is_shell_file` or
 * `is_wiring_file` would have accepted is a finding `quoin validate` silently
 * stops reporting. `tests/core-snapshot.test.ts` pins that with a list of
 * shapes, and a list is exactly as complete as whoever wrote it — narrowing
 * `couldMatter`'s `taskfile` arm to the literal `taskfile.yml` drops
 * `Taskfile.yaml`, which the far side DOES call wiring, and every test in this
 * repository stayed green.
 *
 * So this one does not hold an expectation about the classifier at all. It asks
 * the real binary, path by path, whether the classifier accepts it, and asserts
 * `repoSnapshot` sent every path that came back yes. The oracle is the
 * implementation under test's far side, not a second transcription of its
 * rules — which is the whole reason classification stayed in Rust.
 */
describe.skipIf(!available)(
  "repoSnapshot ⊇ quoin-validators (quoin#412)",
  () => {
    /** A script whose claim, count and pattern make one finding when it is wired. */
    const GATE = [
      "#!/bin/sh",
      "# Gate for FR-001-AC-1: no production symbol shall call `unwrap`.",
      'grep -rn "unwrap()" src/ | wc -l',
    ];

    function findings(
      files: Record<string, string[]>,
    ): { path: string; wiredBy: string }[] {
      const payload = runCore("validators.run", { files }) as {
        findings: { path: string; wiredBy: string }[];
      };
      return payload.findings;
    }

    /**
     * Does the far side classify `path` as a shell script? Asked by wiring it
     * from a Makefile and seeing whether a finding comes back against it — the
     * analysis only ever reaches a file it classified.
     */
    function classifiedAsScript(path: string): boolean {
      if (path === "Makefile") return false;
      return findings({
        Makefile: ["gate:", `\t./${path}`],
        [path]: GATE,
      }).some((f) => f.path === path);
    }

    /** The same question for wiring: is `path` the file that wires the script? */
    function classifiedAsWiring(path: string): boolean {
      if (path === "scripts/check.sh") return false;
      return findings({
        [path]: ["gate:", "\t./scripts/check.sh"],
        "scripts/check.sh": GATE,
      }).some((f) => f.wiredBy === path);
    }

    const NAMES = [
      ".sh",
      "CHECK.SH",
      "JUSTFILE",
      "MAKEFILE",
      "Makefile",
      "Makefile.",
      "Makefile.ci",
      "PACKAGE.JSON",
      "TASKFILE.YML",
      "Taskfile.toml",
      "Taskfile.yaml",
      "Taskfile.yml",
      "check.sh",
      "ci.YML",
      "ci.yaml",
      "ci.yml",
      "gate.SH",
      "gate.bash",
      "gate.sh",
      "justfile",
      "makefile",
      "makefile.txt",
      "package-lock.json",
      "package.json",
      "x.b.sh",
    ];
    const DIRECTORIES = [
      "",
      "scripts/",
      "deep/nested/",
      ".github/workflows/",
      ".github/actions/",
    ];

    it("sends every path the far side would classify", () => {
      const root = mkdtempSync(join(tmpdir(), "quoin-superset-"));
      try {
        const candidates: string[] = [];
        for (const directory of DIRECTORIES) {
          for (const name of NAMES) {
            const path = `${directory}${name}`;
            const target = join(root, path);
            mkdirSync(dirname(target), { recursive: true });
            writeFileSync(target, "x\n");
            candidates.push(path);
          }
        }

        const sent = new Set(Object.keys(repoSnapshot(root).files));
        const classified = candidates.filter(
          (path) => classifiedAsScript(path) || classifiedAsWiring(path),
        );
        // A comparison over an empty population is green and says nothing: if the
        // probes stopped reaching the binary, every path would read as "not
        // classified" and the assertion below would pass over nothing.
        expect(classified.length).toBeGreaterThan(40);
        expect(classified.filter((path) => !sent.has(path))).toEqual([]);
      } finally {
        rmSync(root, { force: true, recursive: true });
      }
    });
  },
);
