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

import { accessSync, constants, readFileSync } from "node:fs";
import { createHash } from "node:crypto";

import { describe, expect, it } from "vitest";

import { CORE_EXIT, runCore, runCoreAllowFailure } from "../src/core/index.js";

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
