/**
 * The TypeScript side of the differential harness (quoin#375, FR-101).
 *
 * FR-101 retires a capability only after old and new implementations passed at
 * one candidate revision. That rule needs two implementations, so Stage 0's
 * one operation has two: `rust/crates/quoin-core/src/ops/core.rs` and this
 * file. This one is the *oracle* — the retained TypeScript.
 *
 * It is written against the same contract, not against the Rust source: the
 * canonical-JSON rule, the `<domain>.<op>` spelling, the exit taxonomy and the
 * diagnostic shape are all read off `rust/crates/quoin-core/src/protocol.rs`'s
 * documented contract. Two implementations of one contract is the point; a
 * transliteration of one implementation would agree with itself.
 *
 * **`core.ping` is the only operation this oracle answers, and after the
 * assurance and completeness cutover (quoin#445/#447) it is the only one it
 * may answer.** The arms that used to delegate to `../assurance/index.js` went
 * when that module did: FR-101-AC-5 forbids a non-Rust runtime oracle for a
 * domain whose TypeScript is gone, and an oracle reimplementing a deleted
 * capability would be a second implementation written this week, which is the
 * one thing the difftest must not compare. `core.ping` is exempt because it
 * never had a retained TypeScript capability to lose — Stage 0 wrote both
 * sides deliberately. The criteria the assurance arms carried are replayed
 * through the real binary from the frozen corpus in
 * `rust/crates/quoin-assurance/tests/golden/`, by
 * `rust/crates/quoin-core/tests/tc_447_assurance_boundary.rs`.
 */

/** Mirrors `quoin_core::protocol::PROTOCOL_VERSION` — deliberately restated,
 * not imported from the generated `./types.js`: this file is the differential
 * harness's independent oracle (FR-101) and importing the generated surface
 * would make it a transliteration of the thing it exists to check. */
export const PROTOCOL_VERSION = 1;

/** Mirrors `quoin_core::ops::core::MAX_ECHO_BYTES`. */
export const MAX_ECHO_BYTES = 4 * 1024;

/** Mirrors the `quoin-core` crate version. */
export const CORE_VERSION = "0.1.0";

/**
 * Every operation the build answers, as `CORE_UNKNOWN_OP` reports them.
 *
 * `quoin_core::dispatch::OPERATIONS` is a sorted const and the diagnostic
 * builds this string with `join(",")` — no space after the comma. The value is
 * restated here rather than imported, for the reason the whole file is
 * restated: an oracle that read the port's own list would agree with it by
 * construction, including when the port's list has gone stale (#443).
 *
 * **This is the BINARY's surface, not this oracle's.** It stayed whole when
 * the assurance arms below were deleted: the string is what a user reads off a
 * mistyped operation, so it must name every operation `quoin-core` routes,
 * including the ones this file no longer has any implementation of.
 */
export const KNOWN_OPERATIONS = [
  "assurance.build_authored_argument",
  "assurance.build_case",
  "assurance.build_discharge",
  "assurance.parse_argument",
  "assurance.render_authored_argument",
  "assurance.render_case",
  "assurance.render_discharge",
  "assurance.requirement_of",
  "completeness.assess_bundle",
  "completeness.read_frontmatter",
  "completeness.schema_refs",
  "core.ping",
  "validators.run",
].join(",");

/** One entry of the stderr array. */
export interface ReferenceDiagnostic {
  code: string;
  message: string;
  context: Record<string, string>;
}

/** Everything one invocation produces. */
export interface ReferenceOutcome {
  /** Canonical JSON for stdout, or `null` when the outcome carries none. */
  payload: string | null;
  /** The stderr array. */
  diagnostics: ReferenceDiagnostic[];
  /** The exit status. */
  exitCode: number;
}

/**
 * Canonical JSON: object keys sorted at every depth, no insignificant
 * whitespace.
 *
 * `JSON.stringify` preserves insertion order, so the sort is explicit here
 * where on the Rust side it falls out of `serde_json::Map` being a `BTreeMap`.
 * Two different mechanisms reaching the same bytes is what makes the
 * comparison in `quoin-difftest` worth running.
 */
export function canonicalJson(value: unknown): string {
  return JSON.stringify(sortKeys(value));
}

function sortKeys(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortKeys);
  if (value === null || typeof value !== "object") return value;
  const source = value as Record<string, unknown>;
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(source).sort())
    out[key] = sortKeys(source[key]);
  return out;
}

function diagnostic(
  code: string,
  message: string,
  context: Record<string, string> = {},
): ReferenceDiagnostic {
  return { code, message, context };
}

function failure(
  exitCode: number,
  entry: ReferenceDiagnostic,
): ReferenceOutcome {
  return { payload: null, diagnostics: [entry], exitCode };
}

/**
 * Answer one invocation.
 *
 * @param argv - everything after the program name.
 * @param stdin - the raw request text; empty means the empty request `{}`.
 */
export function reference(argv: string[], stdin: string): ReferenceOutcome {
  if (argv.length !== 1) {
    return failure(
      3,
      diagnostic(
        "CORE_BAD_USAGE",
        "usage: quoin-core <domain>.<op>  (JSON request on stdin)",
        { argument_count: String(argv.length) },
      ),
    );
  }
  const op = argv[0];
  if (!/^[^.]+\.[^.]+$/.test(op)) {
    return failure(
      3,
      diagnostic("CORE_BAD_USAGE", "an operation is spelled <domain>.<op>", {
        argument: op,
      }),
    );
  }
  // Parse ONCE, before dispatch, because that is where `quoin-core` parses.
  // The Rust dispatcher reads stdin into a `Value` and reports `CORE_BAD_JSON`
  // itself, so malformed input is a TRANSPORT verdict for every operation and
  // never the domain's — and, since the parse precedes routing, it outranks
  // `CORE_UNKNOWN_OP` too: malformed bytes sent to an operation that does not
  // exist are still `CORE_BAD_JSON`. Handling it per operation produced
  // exactly the divergence this harness exists to find: `assurance.build_case`
  // answered `CORE_BAD_REQUEST` where the binary answered `CORE_BAD_JSON`, and
  // `assurance.requirement_of` carried the same defect unnoticed because its
  // nine difftest cases never sent it malformed bytes (quoin#384).
  let parsed: Record<string, unknown> = {};
  if (stdin.trim() !== "") {
    let value: unknown;
    try {
      value = JSON.parse(stdin) as unknown;
    } catch (cause) {
      return failure(3, diagnostic("CORE_BAD_JSON", (cause as Error).message));
    }
    if (value === null || typeof value !== "object" || Array.isArray(value)) {
      return failure(
        3,
        diagnostic("CORE_BAD_JSON", "a request must be a JSON object", {
          observed_type: Array.isArray(value) ? "array" : typeof value,
        }),
      );
    }
    parsed = value as Record<string, unknown>;
  }

  // Everything but `core.ping` — including the twelve operations
  // `KNOWN_OPERATIONS` names and this file does not implement — falls through
  // to `CORE_UNKNOWN_OP`, and that is deliberate rather than an oversight. The
  // verdict is honest for an oracle that has no such operation, and no
  // difftest case may reach it: `quoin-difftest`'s table carries `core.ping`
  // and one deliberately unroutable name, nothing else. A case for a ported
  // operation would compare the binary's real answer against this refusal and
  // report a difference that means only "the TypeScript is gone".
  if (op !== "core.ping") {
    return failure(
      3,
      diagnostic("CORE_UNKNOWN_OP", "no such operation in this build", {
        // `quoin_core::dispatch::OPERATIONS.join(",")` — the const's own
        // order, which is sorted, and its own separator, which carries no
        // space. Restated rather than imported for the reason the whole file
        // is: an oracle that read the port's list would agree with it by
        // construction.
        known: KNOWN_OPERATIONS,
        op,
      }),
    );
  }

  return ping(parsed);
}

function ping(request: Record<string, unknown>): ReferenceOutcome {
  const unknownFields = Object.keys(request).filter(
    (key) => key !== "echo" && key !== "expect_protocol",
  );
  if (unknownFields.length > 0) {
    return failure(
      3,
      diagnostic(
        "CORE_BAD_REQUEST",
        `unknown field, expected \`echo\` or \`expect_protocol\``,
        { op: "core.ping" },
      ),
    );
  }
  const echo = request.echo;
  if (echo !== undefined && echo !== null && typeof echo !== "string") {
    return failure(
      3,
      diagnostic("CORE_BAD_REQUEST", "echo must be a string", {
        op: "core.ping",
      }),
    );
  }
  const expectProtocol = request.expect_protocol;
  if (
    expectProtocol !== undefined &&
    expectProtocol !== null &&
    !Number.isInteger(expectProtocol)
  ) {
    return failure(
      3,
      diagnostic("CORE_BAD_REQUEST", "expect_protocol must be an integer", {
        op: "core.ping",
      }),
    );
  }

  // Byte length, not UTF-16 code units: the Rust side bounds `echo.len()`,
  // which is bytes. `"é".length` is 1 and its byte length is 2, so comparing
  // `.length` here would put the two implementations' ceilings in different
  // units and make the boundary's behaviour depend on the caller's alphabet.
  if (typeof echo === "string") {
    const bytes = Buffer.byteLength(echo, "utf8");
    if (bytes > MAX_ECHO_BYTES) {
      return failure(
        2,
        diagnostic(
          "CORE_REFUSED",
          "echo exceeds the accepted size for a correlation token",
          {
            limit_bytes: String(MAX_ECHO_BYTES),
            observed_bytes: String(bytes),
            op: "core.ping",
          },
        ),
      );
    }
  }

  const payload = canonicalJson({
    core_version: CORE_VERSION,
    echo: echo ?? null,
    protocol_version: PROTOCOL_VERSION,
  });

  if (
    typeof expectProtocol === "number" &&
    expectProtocol !== PROTOCOL_VERSION
  ) {
    return {
      payload,
      diagnostics: [
        diagnostic(
          "CORE_PROTOCOL_SKEW",
          "caller expects a protocol revision this build does not speak; " +
            "the payload is complete and names the revision it does speak",
          {
            actual: String(PROTOCOL_VERSION),
            expected: String(expectProtocol),
            op: "core.ping",
          },
        ),
      ],
      exitCode: 1,
    };
  }
  return { payload, diagnostics: [], exitCode: 0 };
}
