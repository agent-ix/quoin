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

import {
  buildCase,
  parseAssuranceArgument,
  renderCase,
  requirementOf,
} from "../assurance/index.js";

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
  // `assurance.requirement_of` is the first REAL capability on this side of
  // the harness, and it is deliberately a CALL rather than a reimplementation.
  // `core.ping` has two implementations because Stage 0 had no retained
  // capability to compare against; every operation after it does, and FR-101's
  // rule is that the retained implementation IS the oracle. A second copy here
  // would make the difftest prove that two things written this week agree with
  // each other, which is the one thing it must not prove.
  // Parse ONCE, before dispatch, because that is where `quoin-core` parses.
  // The Rust dispatcher reads stdin into a `Value` and reports `CORE_BAD_JSON`
  // itself, so malformed input is a TRANSPORT verdict for every operation and
  // never the domain's. Handling it inside each handler produced exactly the
  // divergence this harness exists to find: `assurance.build_case` answered
  // `CORE_BAD_REQUEST` where the binary answered `CORE_BAD_JSON`, and
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

  if (op === "assurance.requirement_of") {
    return assuranceRequirementOf(parsed);
  }
  if (op === "assurance.build_case") {
    return assuranceBuildCase(parsed);
  }
  if (op === "assurance.render_case") {
    return assuranceRenderCase(parsed);
  }
  if (op === "assurance.parse_argument") {
    return assuranceParseArgument(parsed);
  }
  if (op !== "core.ping") {
    return failure(
      3,
      diagnostic("CORE_UNKNOWN_OP", "no such operation in this build", {
        known:
          "assurance.build_case, assurance.parse_argument, assurance.render_case, assurance.requirement_of, core.ping",
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

/** The largest obligation id accepted, mirroring `MAX_OBLIGATION_ID_BYTES`. */
export const MAX_OBLIGATION_ID_BYTES = 4 * 1024;

/**
 * `assurance.requirement_of`, answered by the RETAINED implementation.
 *
 * The only logic here is the boundary's: parse the request, enforce the size
 * bound, shape the payload. The answer itself comes from
 * `src/assurance/graph.ts` unchanged.
 */
function assuranceRequirementOf(
  fields: Record<string, unknown>,
): ReferenceOutcome {
  for (const key of Object.keys(fields)) {
    if (key !== "obligation_id") {
      return failure(
        3,
        diagnostic("CORE_BAD_REQUEST", `unknown field \`${key}\``, {
          op: "assurance.requirement_of",
        }),
      );
    }
  }
  const id = fields.obligation_id;
  if (typeof id !== "string") {
    return failure(
      3,
      diagnostic("CORE_BAD_REQUEST", "`obligation_id` must be a string", {
        op: "assurance.requirement_of",
      }),
    );
  }
  if (id.length > MAX_OBLIGATION_ID_BYTES) {
    return failure(
      2,
      diagnostic("CORE_REFUSED", "obligation id exceeds the accepted size", {
        limit_bytes: String(MAX_OBLIGATION_ID_BYTES),
        observed_bytes: String(id.length),
        op: "assurance.requirement_of",
      }),
    );
  }
  return {
    exitCode: 0,
    payload: canonicalJson({ requirement: requirementOf(id) }),
    diagnostics: [],
  };
}

/** The largest `assurance.build_case` request accepted, mirroring Rust. */
export const MAX_BUILD_CASE_BYTES = 16 * 1024 * 1024;

/** The request fields `assurance.build_case` accepts, in the wire spelling. */
const BUILD_CASE_FIELDS = new Set([
  "documents",
  "obligations",
  "findings",
  "claim_types",
  "unreadable",
  "producer_trust",
  "evidence_independence",
]);

/**
 * `assurance.build_case`, answered by the RETAINED implementation.
 *
 * Like `assurance.requirement_of` above, the only logic here is the
 * boundary's: parse, bound, rename the request's snake_case keys to the
 * camelCase `CaseInput` the retained module declares, and hand it over. The
 * case itself is built by `src/assurance/graph.ts` unchanged.
 *
 * **The validation below is not defensive programming, it is parity.** Rust's
 * `serde` refuses a request whose `obligations[].id` is a number, and returns
 * `BadRequest`. TypeScript would coerce it and build a case. Without these
 * checks the two sides disagree on every malformed input, and the difftest
 * would be a gate over well-formed requests only — which is the half that
 * never breaks.
 */
function assuranceBuildCase(fields: Record<string, unknown>): ReferenceOutcome {
  const op = "assurance.build_case";
  const bad = (message: string): ReferenceOutcome =>
    failure(3, diagnostic("CORE_BAD_REQUEST", message, { op }));

  // Measured on the re-serialised request, matching the Rust side: the bound
  // is what the DOMAIN refuses, and measuring raw stdin would make whitespace
  // part of the limit on one side only.
  const size = Buffer.byteLength(JSON.stringify(fields), "utf8");
  if (size > MAX_BUILD_CASE_BYTES) {
    return failure(
      2,
      diagnostic("CORE_REFUSED", "request exceeds the accepted size", {
        limit_bytes: String(MAX_BUILD_CASE_BYTES),
        observed_bytes: String(size),
        op,
      }),
    );
  }
  for (const key of Object.keys(fields)) {
    if (!BUILD_CASE_FIELDS.has(key)) return bad(`unknown field \`${key}\``);
  }

  // The three required arrays. `serde` fails on a missing field rather than
  // defaulting it, and a defaulted empty array here would build an empty case
  // where Rust reports a bad request.
  for (const key of ["documents", "obligations", "findings"]) {
    if (!Array.isArray(fields[key])) return bad(`\`${key}\` must be an array`);
  }
  // The optional ones: absent is fine, present-but-wrong is not.
  for (const key of [
    "claim_types",
    "unreadable",
    "producer_trust",
    "evidence_independence",
  ]) {
    const value = fields[key];
    if (value !== undefined && value !== null && !Array.isArray(value)) {
      return bad(`\`${key}\` must be an array`);
    }
  }

  /** Every named field of `entry` is a string, as the Rust type requires. */
  const requireStrings = (
    entries: unknown[],
    what: string,
    keys: string[],
  ): string | null => {
    for (const entry of entries) {
      if (typeof entry !== "object" || entry === null || Array.isArray(entry)) {
        return `each ${what} must be a JSON object`;
      }
      for (const key of keys) {
        if (typeof (entry as Record<string, unknown>)[key] !== "string") {
          return `${what}.${key} must be a string`;
        }
      }
    }
    return null;
  };

  const obligations = fields.obligations as unknown[];
  const findings = fields.findings as unknown[];
  const claimTypes = (fields.claim_types ?? undefined) as unknown[] | undefined;
  const unreadable = (fields.unreadable ?? undefined) as unknown[] | undefined;

  const shapeError =
    requireStrings(obligations, "obligation", ["id", "statement"]) ??
    requireStrings(findings, "finding", ["obligation", "kind", "summary"]) ??
    (unreadable
      ? requireStrings(unreadable, "unreadable", ["path", "reason"])
      : null);
  if (shapeError) return bad(shapeError);
  if (claimTypes && claimTypes.some((t) => typeof t !== "string")) {
    return bad("`claim_types` must be an array of strings");
  }

  const built = buildCase({
    documents: fields.documents as never,
    obligations: obligations as never,
    findings: findings as never,
    ...(claimTypes ? { claimTypes: claimTypes as string[] } : {}),
    ...(unreadable ? { unreadable: unreadable as never } : {}),
    ...(fields.producer_trust
      ? { producerTrust: fields.producer_trust as never }
      : {}),
    ...(fields.evidence_independence
      ? { evidenceIndependence: fields.evidence_independence as never }
      : {}),
  });
  return { exitCode: 0, payload: canonicalJson(built), diagnostics: [] };
}

/**
 * `assurance.parse_argument`, answered by the RETAINED implementation.
 *
 * **The request IS the argument**, with no wrapper object. Every other
 * operation here takes named fields because the retained function does; this
 * one takes a single `unknown` and its first act is to refuse any key outside
 * a closed set of twelve. A wrapper would have introduced a thirteenth key
 * that exists on neither side of the retained API, and the closed set already
 * gives the boundary the "unknown field" verdict a wrapper would have added.
 *
 * **There is no parity validation here, and that is the difference from
 * `assuranceBuildCase`.** The two operations above lean on serde to reject a
 * malformed request, so the reference had to grow a hand-written copy of what
 * serde refuses or the sides would disagree on every bad input. This operation
 * has no derive: the Rust port reads `serde_json::Value` directly and
 * reimplements all seventeen of the retained predicates, because three of them
 * — the asymmetric `null`, JavaScript's trim set, and the impossible-date
 * check — are ones no derive can express. So the validation this function
 * would otherwise duplicate is already the thing under comparison.
 */
function assuranceParseArgument(
  fields: Record<string, unknown>,
): ReferenceOutcome {
  const op = "assurance.parse_argument";

  const size = Buffer.byteLength(JSON.stringify(fields), "utf8");
  if (size > MAX_BUILD_CASE_BYTES) {
    return failure(
      2,
      diagnostic("CORE_REFUSED", "request exceeds the accepted size", {
        limit_bytes: String(MAX_BUILD_CASE_BYTES),
        observed_bytes: String(size),
        op,
      }),
    );
  }

  try {
    return {
      exitCode: 0,
      payload: canonicalJson(parseAssuranceArgument(fields)),
      diagnostics: [],
    };
  } catch (cause) {
    return failure(
      3,
      diagnostic("CORE_BAD_REQUEST", (cause as Error).message, { op }),
    );
  }
}

/**
 * `assurance.render_case`, answered by the RETAINED implementation.
 *
 * The input to this operation is the OUTPUT of `assurance.build_case`, so the
 * size ceiling is deliberately the same constant: a case that could be built
 * and then could not be rendered would be a boundary contradicting itself.
 *
 * **The validation is parity, not defensiveness** — the same reason it is in
 * `assuranceBuildCase`, and more of it here because the Rust side deserialises
 * a deeper structure. `CaseNode.kind` and `CaseNode.status` are CLOSED enums
 * on the Rust side and legitimately so: unlike `Finding.kind`, they are minted
 * by `build_case` rather than supplied by a caller, so the set really is
 * closed. serde refuses an unrecognised one, and the reference must refuse it
 * too or the two sides disagree on exactly the inputs a malformed pipeline
 * produces.
 */
function assuranceRenderCase(
  fields: Record<string, unknown>,
): ReferenceOutcome {
  const op = "assurance.render_case";
  const bad = (message: string): ReferenceOutcome =>
    failure(3, diagnostic("CORE_BAD_REQUEST", message, { op }));

  const size = Buffer.byteLength(JSON.stringify(fields), "utf8");
  if (size > MAX_BUILD_CASE_BYTES) {
    return failure(
      2,
      diagnostic("CORE_REFUSED", "request exceeds the accepted size", {
        limit_bytes: String(MAX_BUILD_CASE_BYTES),
        observed_bytes: String(size),
        op,
      }),
    );
  }

  const isObject = (v: unknown): v is Record<string, unknown> =>
    typeof v === "object" && v !== null && !Array.isArray(v);
  const strings = (v: unknown): boolean =>
    Array.isArray(v) && v.every((e) => typeof e === "string");

  /** Every named key of `entry` is a string. */
  const shaped = (
    entry: unknown,
    what: string,
    keys: string[],
  ): string | null => {
    if (!isObject(entry)) return `each ${what} must be a JSON object`;
    for (const key of keys) {
      if (typeof entry[key] !== "string")
        return `${what}.${key} must be a string`;
    }
    return null;
  };

  // `CaseNode`, recursively. `children` has no serde default, so it is
  // required at every depth — an interior node without it is a bad request on
  // the Rust side and must be one here.
  const NODE_KINDS = new Set(["goal", "strategy", "solution"]);
  const NODE_STATUSES = new Set(["supported", "open"]);
  const node = (entry: unknown, where: string): string | null => {
    const shapeError = shaped(entry, where, ["id", "statement"]);
    if (shapeError) return shapeError;
    const n = entry as Record<string, unknown>;
    if (typeof n.kind !== "string" || !NODE_KINDS.has(n.kind)) {
      return `${where}.kind must be goal, strategy or solution`;
    }
    if (typeof n.status !== "string" || !NODE_STATUSES.has(n.status)) {
      return `${where}.status must be supported or open`;
    }
    if (n.because !== undefined && typeof n.because !== "string") {
      return `${where}.because must be a string when present`;
    }
    if (!Array.isArray(n.children)) return `${where}.children must be an array`;
    for (const child of n.children) {
      const error = node(child, `${where}.children[]`);
      if (error) return error;
    }
    return null;
  };

  if (!Array.isArray(fields.claims)) return bad("`claims` must be an array");
  for (const claim of fields.claims) {
    const error = node(claim, "claim");
    if (error) return bad(error);
  }
  if (fields.reason !== undefined && typeof fields.reason !== "string") {
    return bad("`reason` must be a string when present");
  }
  if (!strings(fields.unreachable)) {
    return bad("`unreachable` must be an array of strings");
  }
  if (!Array.isArray(fields.unreadable)) {
    return bad("`unreadable` must be an array");
  }
  for (const entry of fields.unreadable) {
    const error = shaped(entry, "unreadable", ["path", "reason"]);
    if (error) return bad(error);
  }

  // `producerTrust` is REQUIRED, and `evidenceIndependence` is not, because
  // the retained renderer reads `assurance.producerTrust.length` directly and
  // reaches for `evidenceIndependence` through `?.`. The difftest found the
  // asymmetry: defaulting the first made quoin-core render a case the retained
  // implementation throws on, which is the port being more permissive than
  // what it replaces.
  const trust = fields.producerTrust;
  {
    if (!Array.isArray(trust)) return bad("`producerTrust` must be an array");
    for (const entry of trust) {
      const error = shaped(entry, "producerTrust", ["id", "useId", "status"]);
      if (error) return bad(error);
      const t = entry as Record<string, unknown>;
      if (!strings(t.triggeredBy)) {
        return bad("producerTrust.triggeredBy must be an array of strings");
      }
      if (!strings(t.limitations)) {
        return bad("producerTrust.limitations must be an array of strings");
      }
    }
  }

  const independence = fields.evidenceIndependence;
  if (independence !== undefined && independence !== null) {
    if (!Array.isArray(independence)) {
      return bad("`evidenceIndependence` must be an array");
    }
    for (const entry of independence) {
      const error = shaped(entry, "evidenceIndependence", [
        "profile",
        "requirement",
        "obligation",
        "status",
        "summary",
      ]);
      if (error) return bad(error);
      const a = entry as Record<string, unknown>;
      if (!Array.isArray(a.dimensions)) {
        return bad("evidenceIndependence.dimensions must be an array");
      }
      for (const d of a.dimensions) {
        const dimensionError = shaped(d, "dimension", ["dimension"]);
        if (dimensionError) return bad(dimensionError);
        const dim = d as Record<string, unknown>;
        if (!strings(dim.values)) {
          return bad("dimension.values must be an array of strings");
        }
        if (!strings(dim.missingSuites)) {
          return bad("dimension.missingSuites must be an array of strings");
        }
      }
    }
  }

  const rendered = renderCase(fields as never);
  return { exitCode: 0, payload: canonicalJson({ rendered }), diagnostics: [] };
}
