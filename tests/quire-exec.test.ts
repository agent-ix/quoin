/**
 * `runQuire`'s failure contract (FR-029-AC-10..AC-12, #164).
 *
 * No test exercised the throwing path before this file existed, which is how
 * the message got the diagnosis wrong: on an ENOBUFS death `status` is null,
 * the template fell through to "exited abnormally", and it appended whatever
 * quire had written to stderr — on every real repo the harmless
 * DuplicateArchetype first-wins warnings, which were then investigated as the
 * cause of the failure. The child's stderr is only the diagnosis when the
 * child itself exited non-zero.
 *
 * The fake binary is a `quire` script placed FIRST on PATH (the
 * flows.test.ts / cli.test.ts fake-ix-flow pattern), because
 * tests/arch-boundaries.test.ts requires the executed binary to stay the
 * string literal "quire" — the seam is the PATH, not a parameter.
 */

import { chmodSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { QUIRE_MAX_BUFFER, runQuire } from "../src/quire/index.js";

// The stderr noise every real repo produces; asserting it is ABSENT from
// kill-path messages is the point of AC-12.
const NOISE =
  'DuplicateArchetype: \'ADR\' contributed by modules ["spec-artifacts-process", "spec-artifacts-process"]; first-wins';

function fakeQuireDir(script: string): string {
  const dir = mkdtempSync(join(tmpdir(), "quoin-fake-quire-"));
  const bin = join(dir, "quire");
  writeFileSync(bin, `#!/bin/sh\n${script}\n`);
  chmodSync(bin, 0o755);
  return dir;
}

describe("runQuire failure reporting (FR-029-AC-10..AC-12)", () => {
  const savedPath = process.env.PATH;
  afterEach(() => {
    process.env.PATH = savedPath;
  });

  // TC-254
  it("returns a payload larger than Node's 1 MiB default whole — the cap is actually raised", () => {
    // The headline #164 fix, pinned: without `maxBuffer: QUIRE_MAX_BUFFER` at
    // the call site this payload dies ENOBUFS under Node's 1 MiB default —
    // the filament-ide-rs corpus emitted 1,090,714 bytes, 4% over, and all
    // six shelling commands were killed. The kill-path tests below cannot
    // catch a reverted cap (an overrun dies the same way under either limit),
    // so this success path is the one that fails when the raise is lost.
    const bytes = 2 * 1024 * 1024;
    process.env.PATH = `${fakeQuireDir(
      `head -c ${bytes} /dev/zero | tr '\\0' 'x'`,
    )}:${savedPath}`;
    const stdout = runQuire(["coverage", "--json"]);
    expect(stdout).toHaveLength(bytes);
  });

  // TC-254
  it("names the buffer overrun on an ENOBUFS death — not an exit status, not the child's stderr", () => {
    // A child whose output outgrows maxBuffer is killed by Node: status is
    // null, code is ENOBUFS. Flood one MiB past the cap; the real
    // filament-ide-rs payload was 4% over Node's default and the six commands
    // it killed all reported the DuplicateArchetype noise as the cause (#164).
    process.env.PATH = `${fakeQuireDir(
      `echo '${NOISE}' >&2\nhead -c ${QUIRE_MAX_BUFFER + 1024 * 1024} /dev/zero`,
    )}:${savedPath}`;
    let message = "";
    try {
      runQuire(["coverage", "--json"]);
    } catch (err) {
      message = (err as Error).message;
    }
    expect(message).toContain(`more than ${QUIRE_MAX_BUFFER} bytes`);
    expect(message).toContain("ENOBUFS");
    expect(message).not.toContain("exited");
    expect(message).not.toContain("DuplicateArchetype");
  });

  // TC-255
  it("names the signal on a signal death, and does not append unrelated stderr", () => {
    process.env.PATH = `${fakeQuireDir(
      `echo '${NOISE}' >&2\nkill -TERM $$`,
    )}:${savedPath}`;
    let message = "";
    try {
      runQuire(["coverage", "--json"]);
    } catch (err) {
      message = (err as Error).message;
    }
    expect(message).toContain("killed by SIGTERM");
    expect(message).not.toContain("exited");
    expect(message).not.toContain("DuplicateArchetype");
  });

  // TC-256
  it("still surfaces the child's own stderr when the child itself exited non-zero", () => {
    // AC-10's contract, previously verified by inspection only: when quire
    // EXITS with a diagnosis, that diagnosis is the message worth surfacing.
    process.env.PATH = `${fakeQuireDir(
      `echo 'no module in scope declares a traceability: model' >&2\nexit 3`,
    )}:${savedPath}`;
    let message = "";
    try {
      runQuire(["coverage", "--json"]);
    } catch (err) {
      message = (err as Error).message;
    }
    expect(message).toContain("exited 3");
    expect(message).toContain("no module in scope declares");
  });

  // TC-257
  it("reports a binary that could not be run at all, by its cause", () => {
    // No quire anywhere on PATH: no status, no signal — err.code (ENOENT) is
    // the only true statement available, so it is the one made.
    process.env.PATH = mkdtempSync(join(tmpdir(), "quoin-empty-bin-"));
    let message = "";
    try {
      runQuire(["coverage", "--json"]);
    } catch (err) {
      message = (err as Error).message;
    }
    expect(message).toContain("could not be run");
    expect(message).toContain("ENOENT");
    expect(message).not.toContain("exited");
  });
});
