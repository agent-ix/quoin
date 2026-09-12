/**
 * The TypeScript oracle for the Stage-0 boundary (quoin#375, FR-101).
 *
 * `quoin-difftest` compares this implementation against `quoin-core` and
 * reports differences. It cannot report that BOTH are wrong, so the contract
 * each is written against is asserted here independently: canonical bytes, the
 * exit taxonomy, and the byte-length ceiling on `echo`.
 */

import { describe, expect, it } from "vitest";

import {
  MAX_ECHO_BYTES,
  canonicalJson,
  reference,
} from "../src/core/reference.js";

describe("canonicalJson", () => {
  it("sorts keys at every depth and emits no insignificant whitespace", () => {
    expect(canonicalJson({ z: 1, a: { y: 2, b: 3 } })).toBe(
      '{"a":{"b":3,"y":2},"z":1}',
    );
  });

  it("leaves array order alone — order is data there, not formatting", () => {
    expect(canonicalJson([{ b: 1, a: 2 }, 3])).toBe('[{"a":2,"b":1},3]');
  });
});

describe("reference", () => {
  it("answers a ping with the canonical payload", () => {
    const outcome = reference(["core.ping"], '{"echo":"corr-7"}');
    expect(outcome.exitCode).toBe(0);
    expect(outcome.payload).toBe(
      '{"core_version":"0.1.0","echo":"corr-7","protocol_version":1}',
    );
    expect(outcome.diagnostics).toEqual([]);
  });

  it("treats empty stdin as the empty request", () => {
    const outcome = reference(["core.ping"], "  \n ");
    expect(outcome.exitCode).toBe(0);
    expect(outcome.payload).toContain('"echo":null');
  });

  it("returns a complete payload alongside a protocol-skew diagnostic", () => {
    const outcome = reference(["core.ping"], '{"expect_protocol":99}');
    expect(outcome.exitCode).toBe(1);
    expect(outcome.payload).toContain('"protocol_version":1');
    expect(outcome.diagnostics[0].code).toBe("CORE_PROTOCOL_SKEW");
  });

  it("bounds echo in BYTES, so the ceiling does not depend on the alphabet", () => {
    // "é" is one UTF-16 unit and two UTF-8 bytes. Measuring `.length` here
    // would put this ceiling and quoin-core's `echo.len()` in different units,
    // and the two would disagree only for non-ASCII callers.
    const justOver = "é".repeat(MAX_ECHO_BYTES / 2 + 1);
    expect(justOver.length).toBeLessThan(MAX_ECHO_BYTES);
    const outcome = reference(
      ["core.ping"],
      JSON.stringify({ echo: justOver }),
    );
    expect(outcome.exitCode).toBe(2);
    expect(outcome.diagnostics[0].code).toBe("CORE_REFUSED");
    expect(outcome.payload).toBeNull();
  });

  it("maps every invalid shape to exit 3 with its own code", () => {
    const cases: Array<[string[], string, string]> = [
      [[], "", "CORE_BAD_USAGE"],
      [["core.ping", "extra"], "", "CORE_BAD_USAGE"],
      [["notdotted"], "", "CORE_BAD_USAGE"],
      [["evidence.record"], "{}", "CORE_UNKNOWN_OP"],
      [["core.ping"], "{oops", "CORE_BAD_JSON"],
      [["core.ping"], "[1,2]", "CORE_BAD_JSON"],
      [["core.ping"], '{"eco":1}', "CORE_BAD_REQUEST"],
      [["core.ping"], '{"echo":7}', "CORE_BAD_REQUEST"],
    ];
    for (const [argv, stdin, code] of cases) {
      const outcome = reference(argv, stdin);
      expect(outcome.exitCode, `${argv.join(" ")} ${stdin}`).toBe(3);
      expect(outcome.payload).toBeNull();
      expect(outcome.diagnostics[0].code).toBe(code);
    }
  });
});
