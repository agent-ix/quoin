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

  // Trace: FR-043-AC-20
  // TC-1071's command half. `tests/gate-validator.test.ts` carried it and is
  // gone; `tc_377_019` names the criterion but calls `GateReport` in process
  // and never reaches the oclif command, so before quoin#448 the matrix row
  // read ✅ over a test that did not exercise it (FND-004).
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

  // Trace: FR-043-AC-20
  it("says so when there is nothing to report", async () => {
    // TC-1068: identical shell text with no wiring is a report, not a gate.
    const root = mkdtempSync(join(tmpdir(), "quoin-validate-e2e-"));
    badGate(root);
    writeFileSync(join(root, "Makefile"), "report:\n\t@echo report only\n");
    expect(await run(["--repo", root])).toEqual([
      "repository QA gates: no findings",
    ]);
  });

  /**
   * The bytes a caller sees, pinned against the oracle they used to be
   * (quoin#448 FND-002).
   *
   * `--json` is still `JSON.stringify({ findings }, null, 2)`, but the findings
   * now arrive through `protocol::canonical_json`, which sorts object keys at
   * every depth. So the document a consumer diffs CHANGED, and the review found
   * nothing pinning it: `golden_parity.rs::tc_377_019` compares
   * `to_string_pretty(&report)` — declaration order, a path the shipped command
   * no longer takes — and the case above uses `objectContaining`, which is
   * order-insensitive.
   *
   * The expectation here is DERIVED, not authored: it is the TypeScript
   * oracle's own captured stdout (`tests/golden/expected.json`, taken from
   * quoin `4d27dcf`) with its keys recursively sorted. So the assertion states
   * exactly one permitted difference — key order — over the whole document,
   * values, indentation and all. Writing the field list out by hand would also
   * be a type declaration in TypeScript, which FR-097 forbids.
   */
  it("emits the oracle's document, byte for byte, modulo key order", async () => {
    const goldenDir = join(
      repoRoot,
      "rust/crates/quoin-validators/tests/golden",
    );
    const corpus = JSON.parse(
      readFileSync(join(goldenDir, "cases.json"), "utf8"),
    ) as { cases: { name: string; files: Record<string, string[]> }[] };
    const oracle = JSON.parse(
      readFileSync(join(goldenDir, "expected.json"), "utf8"),
    ) as { cases: { name: string; json: string }[] };

    const sortKeys = (value: unknown): unknown => {
      if (Array.isArray(value)) return value.map(sortKeys);
      if (value === null || typeof value !== "object") return value;
      const source = value as Record<string, unknown>;
      const out: Record<string, unknown> = {};
      for (const key of Object.keys(source).sort())
        out[key] = sortKeys(source[key]);
      return out;
    };

    // Cases chosen by what they contain, not by name: one finding, several
    // findings across several files, and none. A single case would let a
    // regression in ordering ACROSS findings pass.
    const chosen = [
      "baseline-bad-gate",
      "multiple-files-sorted",
      "no-gate-claim",
    ];
    let findingsAsserted = 0;
    for (const name of chosen) {
      const spec = corpus.cases.find((c) => c.name === name);
      const expectation = oracle.cases.find((c) => c.name === name);
      expect(spec, `corpus case ${name}`).toBeDefined();
      expect(expectation, `oracle case ${name}`).toBeDefined();

      const root = mkdtempSync(join(tmpdir(), "quoin-validate-bytes-"));
      for (const [relative, lines] of Object.entries(spec!.files)) {
        const target = join(root, relative);
        mkdirSync(dirname(target), { recursive: true });
        writeFileSync(target, lines.join("\n"));
      }

      const wanted = `${JSON.stringify(sortKeys(JSON.parse(expectation!.json)), null, 2)}`;
      expect(
        (await run(["--repo", root, "--json"])).join("\n"),
        `case ${name}`,
      ).toBe(wanted);
      findingsAsserted += (
        JSON.parse(expectation!.json) as { findings: unknown[] }
      ).findings.length;
    }

    // A population floor. Every comparison above is an equality, and two empty
    // documents are equal: if the corpus lost its findings, or the chosen names
    // stopped matching, this would agree perfectly about `{"findings": []}`.
    expect(findingsAsserted).toBeGreaterThanOrEqual(3);
  });

  /**
   * An unusable `--repo` exits 2 and names the repository (quoin#448 FND-008).
   *
   * A user-visible change this PR makes, measured rather than asserted from
   * memory: on `main` at `4d27dcf`, `readdirSync`'s `ENOENT` escaped
   * `inspectEmptyGates` with no `oclif.exit`, and oclif's handler exits 1
   * (`@oclif/core/lib/errors/handle.js`: `err.oclif?.exit ?? 1`). Measured on
   * that tree: `node bin/quoin.js validate --repo /nope/does/not/exist` exits
   * **1**. It now exits **2**, because the unlistable root reaches the boundary
   * and comes back `CORE_REFUSED`.
   *
   * The message half matters too: paths on the wire are repository-relative, so
   * the far side can only call the root `""` and its own text reads "repository
   * root  is not a readable directory". The command names the `--repo` it was
   * given, because otherwise this PR replaces a message containing the typo
   * with one containing a blank.
   */
  it("exits 2 and names the repository when the root cannot be listed", async () => {
    const missing = join(tmpdir(), "quoin-no-such-repo-", String(process.pid));
    await expect(run(["--repo", missing])).rejects.toMatchObject({
      oclif: { exit: 2 },
      message: expect.stringContaining(missing),
    });
  });

  // Trace: FR-043-AC-20
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

    // One `quoin-core` spawn per candidate path, and the candidate set is the
    // cross product of NAMES and DIRECTORIES — so the wall clock scales with
    // the classifier surface this test exists to cover, not with the machine.
    // It needs ~8s against vitest's 5s default and timed out in `make rust-e2e`
    // on this tree (agent-ix/quoin#447). The budget is stated rather than
    // inherited; every assertion below is unchanged, and the population floor
    // still refuses a run that swept nothing.
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
    }, 60_000);
  },
);
