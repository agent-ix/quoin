/**
 * The TypeScript side of the differential harness (quoin#375, FR-101).
 *
 * FR-101 retires a capability only after old and new implementations passed at
 * one candidate revision. That rule needs two implementations, so Stage 0's
 * one operation has two: `rust/crates/quoin-core/src/ops/core.rs` and this
 * file. This one is the *oracle* — the retained TypeScript — exactly as the
 * 109 retained TS test files are the oracle for the domains that follow.
 *
 * It is written against the same contract, not against the Rust source: the
 * canonical-JSON rule, the `<domain>.<op>` spelling, the exit taxonomy and the
 * diagnostic shape are all read off `rust/crates/quoin-core/src/protocol.rs`'s
 * documented contract. Two implementations of one contract is the point; a
 * transliteration of one implementation would agree with itself.
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
  if (op !== "core.ping") {
    return failure(
      3,
      diagnostic("CORE_UNKNOWN_OP", "no such operation in this build", {
        known: "core.ping",
        op,
      }),
    );
  }

  let request: unknown = {};
  if (stdin.trim() !== "") {
    try {
      request = JSON.parse(stdin) as unknown;
    } catch (cause) {
      return failure(3, diagnostic("CORE_BAD_JSON", (cause as Error).message));
    }
    if (
      request === null ||
      typeof request !== "object" ||
      Array.isArray(request)
    ) {
      return failure(
        3,
        diagnostic("CORE_BAD_JSON", "a request must be a JSON object", {
          observed_type: Array.isArray(request) ? "array" : typeof request,
        }),
      );
    }
  }
  return ping(request as Record<string, unknown>);
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
