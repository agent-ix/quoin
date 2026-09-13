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
  buildAuthoredArgumentView,
  buildCase,
  buildDischargeReport,
  parseAssuranceArgument,
  renderAuthoredArgument,
  renderCase,
  renderDischargeReport,
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

/**
 * Every operation the build answers, as `CORE_UNKNOWN_OP` reports them.
 *
 * `quoin_core::dispatch::OPERATIONS` is a sorted const and the diagnostic
 * builds this string with `join(",")` — no space after the comma. The value is
 * restated here rather than imported, for the reason the whole file is
 * restated: an oracle that read the port's own list would agree with it by
 * construction, including when the port's list has gone stale (#443).
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
  if (op === "assurance.build_authored_argument") {
    return assuranceBuildAuthoredArgument(parsed);
  }
  if (op === "assurance.render_authored_argument") {
    return assuranceRenderAuthoredArgument(parsed);
  }
  if (op === "assurance.build_discharge") {
    return assuranceBuildDischarge(parsed);
  }
  if (op === "assurance.render_discharge") {
    return assuranceRenderDischarge(parsed);
  }
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

/**
 * A JSON object — what every nested record below must be.
 *
 * `typeof null` is `"object"` and an array is one too, so both are excluded
 * explicitly: serde reads neither into a struct.
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** An array whose every entry is a string, which is serde's `Vec<String>`. */
function isStringArray(value: unknown): boolean {
  return (
    Array.isArray(value) && value.every((item) => typeof item === "string")
  );
}

/** A `BTreeMap<String, String>` — an object whose every value is a string. */
function isStringMap(value: unknown): boolean {
  return (
    isRecord(value) &&
    Object.values(value).every((item) => typeof item === "string")
  );
}

/** Every key named in `keys` is a string on `entry`. */
function requiredStrings(
  entry: unknown,
  what: string,
  keys: string[],
): string | null {
  if (!isRecord(entry)) return `${what} must be a JSON object`;
  for (const key of keys) {
    if (typeof entry[key] !== "string")
      return `${what}.${key} must be a string`;
  }
  return null;
}

/** An `Option<String>`: absent or `null` is fine, anything else is not. */
function optionalString(
  entry: Record<string, unknown>,
  key: string,
  what: string,
): string | null {
  const value = entry[key];
  if (value === undefined || value === null || typeof value === "string") {
    return null;
  }
  return `${what}.${key} must be a string when present`;
}

/** One member of a CLOSED serde enum, which refuses anything unlisted. */
function oneOfLiteral(
  value: unknown,
  what: string,
  allowed: Set<string>,
): string | null {
  if (typeof value !== "string" || !allowed.has(value)) {
    return `${what} must be one of ${[...allowed].join(", ")}`;
  }
  return null;
}

/** `quoin_quire_types::ClauseForce`, `snake_case`. */
const CLAUSE_FORCES = new Set(["mandatory", "recommended", "permitted"]);
/** `quoin_quire_types::ClauseBindingOutcome`, `snake_case`. */
const CLAUSE_OUTCOMES = new Set(["binding", "not_binding", "unresolved"]);
/** `quoin_assurance::DischargeState`, `snake_case`. */
const DISCHARGE_STATES = new Set([
  "direct",
  "disposition",
  "open",
  "unresolved",
  "not_binding",
]);
/** `quoin_assurance::FactKind`, `lowercase`. */
const FACT_KINDS = new Set(["direct", "disposition"]);
/** `quoin_assurance::DispositionDecision`, `snake_case`. */
const DISPOSITION_DECISIONS = new Set([
  "accepted_risk",
  "temporary_exception",
  "delegated",
]);
/** `quoin_assurance::UnusedFactReason`, `snake_case`. */
const UNUSED_FACT_REASONS = new Set([
  "unknown_clause",
  "not_binding",
  "unresolved",
]);
/** `quoin_assurance::ViewStatus`, `lowercase`. */
const VIEW_STATUSES = new Set(["supported", "open"]);
/** `quoin_assurance::ChallengeViewStatus`, `lowercase`. */
const CHALLENGE_VIEW_STATUSES = new Set(["resolved", "open"]);
/** `quoin_assurance::DecisionState`, `lowercase`. */
const DECISION_STATES = new Set(["satisfied", "open"]);
/** `quoin_assurance::ArgumentStatus`, `lowercase`. */
const ARGUMENT_STATUSES = new Set(["proposed", "active", "retired"]);
/** `quoin_assurance::AssumptionStatus`, `lowercase`. */
const ASSUMPTION_STATUSES = new Set(["open", "accepted", "invalidated"]);
/** `quoin_assurance::ChallengeStatus`, `kebab-case`. */
const CHALLENGE_STATUSES = new Set(["open", "resolved", "accepted-risk"]);
/** `quoin_assurance::RelationshipType`, `lowercase`. */
const RELATIONSHIP_TYPES = new Set(["supports", "challenges", "references"]);

/** `quoin_quire_types::ClauseSetKey`. */
function clauseSetKeyShape(value: unknown, where: string): string | null {
  return requiredStrings(value, where, ["authority", "id", "version"]);
}

/** `quoin_quire_types::ClauseBinding`. */
function clauseBindingShape(value: unknown, where: string): string | null {
  const shapeError = requiredStrings(value, where, ["clauseId"]);
  if (shapeError) return shapeError;
  const clause = value as Record<string, unknown>;
  const force = oneOfLiteral(clause.force, `${where}.force`, CLAUSE_FORCES);
  if (force) return force;
  const outcome = oneOfLiteral(
    clause.outcome,
    `${where}.outcome`,
    CLAUSE_OUTCOMES,
  );
  if (outcome) return outcome;
  if (!Array.isArray(clause.reasons))
    return `${where}.reasons must be an array`;
  for (const item of clause.reasons) {
    const reasonError = requiredStrings(item, `${where}.reasons[]`, [
      "code",
      "message",
    ]);
    if (reasonError) return reasonError;
    const dimension = optionalString(
      item as Record<string, unknown>,
      "dimension",
      `${where}.reasons[]`,
    );
    if (dimension) return dimension;
  }
  if (!isStringArray(clause.expectedOutputs)) {
    return `${where}.expectedOutputs must be an array of strings`;
  }
  return null;
}

/**
 * `quoin_quire_types::ClauseBindingReport`, which the retained
 * `buildDischargeReport` reads structurally and the port deserialises.
 *
 * **This is parity, not defensiveness** — the same rule as
 * `assuranceBuildCase`. The retained function takes a TYPED parameter and
 * trusts it, so a `force` of `7` would reach the partition on this side and be
 * a bad request on the other. `schemaVersion` is CLOSED for the reason
 * `quoin-quire-types` states: a `clause-binding-v2` payload must fail to read,
 * not be read with v1 semantics and reported as a clean partition.
 */
function clauseBindingReportShape(
  value: unknown,
  where: string,
): string | null {
  if (!isRecord(value)) return `${where} must be a JSON object`;
  if (value.schemaVersion !== "clause-binding-v1") {
    return `${where}.schemaVersion must be clause-binding-v1`;
  }
  const key = clauseSetKeyShape(value.clauseSet, `${where}.clauseSet`);
  if (key) return key;
  if (typeof value.clauseSetDigest !== "string") {
    return `${where}.clauseSetDigest must be a string`;
  }
  if (!isStringMap(value.context)) {
    return `${where}.context must be an object of strings`;
  }
  if (!Array.isArray(value.clauses)) return `${where}.clauses must be an array`;
  for (const clause of value.clauses) {
    const clauseError = clauseBindingShape(clause, `${where}.clauses[]`);
    if (clauseError) return clauseError;
  }
  const engine = value.engine;
  if (engine !== undefined && engine !== null) {
    const engineError = requiredStrings(engine, `${where}.engine`, [
      "cli",
      "engine",
    ]);
    if (engineError) return engineError;
    if (!isStringArray((engine as Record<string, unknown>).capabilities)) {
      return `${where}.engine.capabilities must be an array of strings`;
    }
  }
  return null;
}

/** `quoin_assurance::DischargeAttestation`. */
function attestationShape(value: unknown, where: string): string | null {
  return requiredStrings(value, where, [
    "attestedBy",
    "authority",
    "attestedAt",
    "expiresAt",
    "sourceRevision",
    "evidenceDigest",
  ]);
}

/**
 * `quoin_assurance::DischargeFact`, an internally tagged enum on `kind`.
 *
 * serde picks the arm from the tag before reading the payload, so a `direct`
 * fact carrying `approvalRef` is refused for its missing `evidenceRefs` rather
 * than for the extra key — the enum's arms carry no `deny_unknown_fields`.
 */
function dischargeFactShape(value: unknown, where: string): string | null {
  if (!isRecord(value)) return `${where} must be a JSON object`;
  const kind = oneOfLiteral(value.kind, `${where}.kind`, FACT_KINDS);
  if (kind) return kind;
  const identity = requiredStrings(value, where, ["clauseId"]);
  if (identity) return identity;
  const attestation = attestationShape(
    value.attestation,
    `${where}.attestation`,
  );
  if (attestation) return attestation;
  if (value.kind === "direct") {
    if (!isStringArray(value.evidenceRefs)) {
      return `${where}.evidenceRefs must be an array of strings`;
    }
    return null;
  }
  const decision = oneOfLiteral(
    value.decision,
    `${where}.decision`,
    DISPOSITION_DECISIONS,
  );
  if (decision) return decision;
  return requiredStrings(value, where, ["rationale", "approvalRef"]);
}

/** `quoin_assurance::ClauseDischarge`. */
function clauseDischargeShape(value: unknown, where: string): string | null {
  const identity = requiredStrings(value, where, ["clauseId"]);
  if (identity) return identity;
  const entry = value as Record<string, unknown>;
  const force = oneOfLiteral(entry.force, `${where}.force`, CLAUSE_FORCES);
  if (force) return force;
  const state = oneOfLiteral(entry.state, `${where}.state`, DISCHARGE_STATES);
  if (state) return state;
  if (!isStringArray(entry.expectedOutputs)) {
    return `${where}.expectedOutputs must be an array of strings`;
  }
  const reason = optionalString(entry, "reason", where);
  if (reason) return reason;
  if (entry.fact !== undefined && entry.fact !== null) {
    return dischargeFactShape(entry.fact, `${where}.fact`);
  }
  return null;
}

/** `quoin_assurance::DischargeReport`, `assurance.build_discharge`'s payload. */
function dischargeReportShape(value: unknown, where: string): string | null {
  if (!isRecord(value)) return `${where} must be a JSON object`;
  if (value.schemaVersion !== "clause-discharge-v1") {
    return `${where}.schemaVersion must be clause-discharge-v1`;
  }
  const key = clauseSetKeyShape(value.clauseSet, `${where}.clauseSet`);
  if (key) return key;
  const strings = requiredStrings(value, where, ["clauseSetDigest", "asOf"]);
  if (strings) return strings;
  if (!isStringMap(value.context)) {
    return `${where}.context must be an object of strings`;
  }
  const binding = value.binding;
  if (!isRecord(binding)) return `${where}.binding must be a JSON object`;
  for (const partition of ["direct", "dispositions", "open"]) {
    const entries = binding[partition];
    if (!Array.isArray(entries)) {
      return `${where}.binding.${partition} must be an array`;
    }
    for (const entry of entries) {
      const entryError = clauseDischargeShape(
        entry,
        `${where}.binding.${partition}[]`,
      );
      if (entryError) return entryError;
    }
  }
  for (const partition of ["unresolved", "notBinding"]) {
    const entries = value[partition];
    if (!Array.isArray(entries)) {
      return `${where}.${partition} must be an array`;
    }
    for (const entry of entries) {
      const entryError = clauseDischargeShape(entry, `${where}.${partition}[]`);
      if (entryError) return entryError;
    }
  }
  if (!Array.isArray(value.unusedFacts)) {
    return `${where}.unusedFacts must be an array`;
  }
  for (const entry of value.unusedFacts) {
    const factError = requiredStrings(entry, `${where}.unusedFacts[]`, [
      "clauseId",
    ]);
    if (factError) return factError;
    const unused = entry as Record<string, unknown>;
    const kind = oneOfLiteral(
      unused.kind,
      `${where}.unusedFacts[].kind`,
      FACT_KINDS,
    );
    if (kind) return kind;
    const reason = oneOfLiteral(
      unused.reason,
      `${where}.unusedFacts[].reason`,
      UNUSED_FACT_REASONS,
    );
    if (reason) return reason;
  }
  return null;
}

/**
 * `assurance.build_discharge`, answered by the RETAINED implementation.
 *
 * The only logic here is the boundary's: bound the request, check what serde
 * checks, hand it over. The partition itself is built by
 * `src/assurance/discharge.ts` unchanged, and every refusal it raises — a
 * malformed fact, a non-instant `asOf`, two facts naming one clause
 * (FR-046-AC-5) — arrives as `CORE_BAD_REQUEST`, matching the port's decision
 * that a document failing the contract is a caller mistake and not something
 * this process declined to read.
 */
function assuranceBuildDischarge(
  fields: Record<string, unknown>,
): ReferenceOutcome {
  const op = "assurance.build_discharge";
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

  const allowed = new Set(["binding", "facts", "asOf"]);
  for (const key of Object.keys(fields)) {
    if (!allowed.has(key)) return bad(`unknown field \`${key}\``);
  }
  const bindingError = clauseBindingReportShape(fields.binding, "binding");
  if (bindingError) return bad(bindingError);
  // `facts` is `Vec<serde_json::Value>` on the port, so serde checks that it
  // is an array and nothing more. Each entry is validated by the ported
  // `parseFact`, which is the thing under comparison.
  if (!Array.isArray(fields.facts)) return bad("`facts` must be an array");
  if (typeof fields.asOf !== "string") return bad("`asOf` must be a string");

  try {
    return {
      exitCode: 0,
      payload: canonicalJson(
        buildDischargeReport({
          binding: fields.binding as never,
          facts: fields.facts as never,
          asOf: fields.asOf,
        }),
      ),
      diagnostics: [],
    };
  } catch (cause) {
    return bad((cause as Error).message);
  }
}

/**
 * `assurance.render_discharge`, answered by the RETAINED implementation.
 *
 * The input is the OUTPUT of `assurance.build_discharge`, so the ceiling is
 * deliberately the same constant, for the reason `assuranceRenderCase` states:
 * a report that could be built and then could not be rendered would be a
 * boundary contradicting itself.
 */
function assuranceRenderDischarge(
  fields: Record<string, unknown>,
): ReferenceOutcome {
  const op = "assurance.render_discharge";

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

  const shapeError = dischargeReportShape(fields, "report");
  if (shapeError) {
    return failure(3, diagnostic("CORE_BAD_REQUEST", shapeError, { op }));
  }

  const rendered = renderDischargeReport(fields as never);
  return { exitCode: 0, payload: canonicalJson({ rendered }), diagnostics: [] };
}

/** `quoin_assurance::SufficiencyDecision`, as it appears inside a built view. */
function sufficiencyDecisionShape(
  value: unknown,
  where: string,
): string | null {
  const strings = requiredStrings(value, where, [
    "reasoningId",
    "criterion",
    "decidedBy",
    "authority",
    "decidedAt",
    "expiresAt",
    "sourceRevision",
    "evidenceDigest",
  ]);
  if (strings) return strings;
  const decision = value as Record<string, unknown>;
  const state = oneOfLiteral(decision.state, `${where}.state`, DECISION_STATES);
  if (state) return state;
  if (!isStringArray(decision.evidenceRefs)) {
    return `${where}.evidenceRefs must be an array of strings`;
  }
  return optionalString(decision, "rationale", where);
}

/**
 * `quoin_assurance::AuthoredArgumentView`, the payload of
 * `assurance.build_authored_argument`.
 *
 * Deeper than `assuranceRenderCase`'s validation, for the same reason: the
 * port deserialises a whole typed document, and eight of its fields are CLOSED
 * enums minted by the builder rather than supplied by a caller. serde refuses
 * an unrecognised one, and this side must refuse it too or the two disagree on
 * exactly the inputs a malformed pipeline produces.
 */
function authoredArgumentViewShape(
  value: unknown,
  where: string,
): string | null {
  if (!isRecord(value)) return `${where} must be a JSON object`;
  if (value.schemaVersion !== "authored-assurance-view-v1") {
    return `${where}.schemaVersion must be authored-assurance-view-v1`;
  }
  if (typeof value.asOf !== "string") return `${where}.asOf must be a string`;

  const summary = requiredStrings(value.argument, `${where}.argument`, [
    "id",
    "title",
    "owner",
    "profile",
  ]);
  if (summary) return summary;
  const argumentStatus = oneOfLiteral(
    (value.argument as Record<string, unknown>).status,
    `${where}.argument.status`,
    ARGUMENT_STATUSES,
  );
  if (argumentStatus) return argumentStatus;

  const topClaim = requiredStrings(value.topClaim, `${where}.topClaim`, [
    "id",
    "statement",
    "subject",
  ]);
  if (topClaim) return topClaim;
  const claim = value.topClaim as Record<string, unknown>;
  const claimStatus = oneOfLiteral(
    claim.status,
    `${where}.topClaim.status`,
    VIEW_STATUSES,
  );
  if (claimStatus) return claimStatus;
  if (!isStringArray(claim.reasons)) {
    return `${where}.topClaim.reasons must be an array of strings`;
  }

  if (!Array.isArray(value.reasoning)) {
    return `${where}.reasoning must be an array`;
  }
  for (const step of value.reasoning) {
    const stepError = requiredStrings(step, `${where}.reasoning[]`, [
      "id",
      "statement",
      "supports",
    ]);
    if (stepError) return stepError;
    const reasoning = step as Record<string, unknown>;
    const status = oneOfLiteral(
      reasoning.status,
      `${where}.reasoning[].status`,
      VIEW_STATUSES,
    );
    if (status) return status;
    if (!Array.isArray(reasoning.criteria)) {
      return `${where}.reasoning[].criteria must be an array`;
    }
    for (const entry of reasoning.criteria) {
      const criterionError = requiredStrings(
        entry,
        `${where}.reasoning[].criteria[]`,
        ["criterion"],
      );
      if (criterionError) return criterionError;
      const criterion = entry as Record<string, unknown>;
      const criterionStatus = oneOfLiteral(
        criterion.status,
        `${where}.reasoning[].criteria[].status`,
        VIEW_STATUSES,
      );
      if (criterionStatus) return criterionStatus;
      const reason = optionalString(
        criterion,
        "reason",
        `${where}.reasoning[].criteria[]`,
      );
      if (reason) return reason;
      if (criterion.decision !== undefined && criterion.decision !== null) {
        const decisionError = sufficiencyDecisionShape(
          criterion.decision,
          `${where}.reasoning[].criteria[].decision`,
        );
        if (decisionError) return decisionError;
      }
    }
  }

  if (!Array.isArray(value.assumptions)) {
    return `${where}.assumptions must be an array`;
  }
  for (const entry of value.assumptions) {
    const assumptionError = requiredStrings(entry, `${where}.assumptions[]`, [
      "id",
      "statement",
      "owner",
      "reviewBy",
    ]);
    if (assumptionError) return assumptionError;
    const assumption = entry as Record<string, unknown>;
    const status = oneOfLiteral(
      assumption.status,
      `${where}.assumptions[].status`,
      VIEW_STATUSES,
    );
    if (status) return status;
    const declared = oneOfLiteral(
      assumption.declaredStatus,
      `${where}.assumptions[].declaredStatus`,
      ASSUMPTION_STATUSES,
    );
    if (declared) return declared;
    const reason = optionalString(
      assumption,
      "reason",
      `${where}.assumptions[]`,
    );
    if (reason) return reason;
  }

  if (!Array.isArray(value.participants)) {
    return `${where}.participants must be an array`;
  }
  for (const entry of value.participants) {
    const participantError = requiredStrings(entry, `${where}.participants[]`, [
      "id",
      "role",
      "authority",
      "independence",
    ]);
    if (participantError) return participantError;
  }

  if (!Array.isArray(value.challenges)) {
    return `${where}.challenges must be an array`;
  }
  for (const entry of value.challenges) {
    const challengeError = requiredStrings(entry, `${where}.challenges[]`, [
      "id",
      "target",
      "statement",
      "owner",
    ]);
    if (challengeError) return challengeError;
    const challenge = entry as Record<string, unknown>;
    const status = oneOfLiteral(
      challenge.status,
      `${where}.challenges[].status`,
      CHALLENGE_VIEW_STATUSES,
    );
    if (status) return status;
    const declared = oneOfLiteral(
      challenge.declaredStatus,
      `${where}.challenges[].declaredStatus`,
      CHALLENGE_STATUSES,
    );
    if (declared) return declared;
    if (!isStringArray(challenge.resolutionRefs)) {
      return `${where}.challenges[].resolutionRefs must be an array of strings`;
    }
    const expiresAt = optionalString(
      challenge,
      "expiresAt",
      `${where}.challenges[]`,
    );
    if (expiresAt) return expiresAt;
    const reason = optionalString(challenge, "reason", `${where}.challenges[]`);
    if (reason) return reason;
  }

  if (!Array.isArray(value.relationships)) {
    return `${where}.relationships must be an array`;
  }
  for (const entry of value.relationships) {
    const relationshipError = requiredStrings(
      entry,
      `${where}.relationships[]`,
      ["target"],
    );
    if (relationshipError) return relationshipError;
    const type = oneOfLiteral(
      (entry as Record<string, unknown>).type,
      `${where}.relationships[].type`,
      RELATIONSHIP_TYPES,
    );
    if (type) return type;
  }

  if (!Array.isArray(value.unusedDecisions)) {
    return `${where}.unusedDecisions must be an array`;
  }
  for (const entry of value.unusedDecisions) {
    const unusedError = requiredStrings(entry, `${where}.unusedDecisions[]`, [
      "reasoningId",
      "criterion",
    ]);
    if (unusedError) return unusedError;
  }

  if (value.discharge !== undefined && value.discharge !== null) {
    return dischargeReportShape(value.discharge, `${where}.discharge`);
  }
  return null;
}

/**
 * `assurance.build_authored_argument`, answered by the RETAINED implementation.
 *
 * `argument` and `decisions` are handed over UNVALIDATED, exactly as the port
 * hands over a `serde_json::Value` and a `Vec<serde_json::Value>`: the
 * seventeen predicates in `src/assurance/argument.ts` are the contract, and a
 * shape check here would be a second door past the thing under comparison.
 * What is checked is only what serde checks on the wrapper — the closed key
 * set, `decisions` being an array, `asOf` being a string, and a supplied
 * `discharge` being a report.
 */
function assuranceBuildAuthoredArgument(
  fields: Record<string, unknown>,
): ReferenceOutcome {
  const op = "assurance.build_authored_argument";
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

  const allowed = new Set(["argument", "decisions", "asOf", "discharge"]);
  for (const key of Object.keys(fields)) {
    if (!allowed.has(key)) return bad(`unknown field \`${key}\``);
  }
  // `argument` has no serde default, so an absent key is a bad request even
  // though its VALUE is unconstrained — including an explicit `null`, which
  // `parseAssuranceArgument` then refuses as "argument must be an object".
  if (!("argument" in fields)) return bad("missing field `argument`");
  if (!Array.isArray(fields.decisions)) {
    return bad("`decisions` must be an array");
  }
  if (typeof fields.asOf !== "string") return bad("`asOf` must be a string");
  // Absent and `null` mean the same thing — no report — because the retained
  // signature makes the parameter optional and `#[serde(default)]` maps a JSON
  // null onto `None`.
  if (fields.discharge !== undefined && fields.discharge !== null) {
    const dischargeError = dischargeReportShape(fields.discharge, "discharge");
    if (dischargeError) return bad(dischargeError);
  }

  try {
    return {
      exitCode: 0,
      payload: canonicalJson(
        buildAuthoredArgumentView({
          argument: fields.argument,
          decisions: fields.decisions as never,
          asOf: fields.asOf,
          ...(fields.discharge ? { discharge: fields.discharge as never } : {}),
        }),
      ),
      diagnostics: [],
    };
  } catch (cause) {
    return bad((cause as Error).message);
  }
}

/**
 * `assurance.render_authored_argument`, answered by the RETAINED
 * implementation.
 *
 * The input is the OUTPUT of `assurance.build_authored_argument`, so the
 * ceiling is the same constant for the reason `assuranceRenderDischarge`
 * states.
 */
function assuranceRenderAuthoredArgument(
  fields: Record<string, unknown>,
): ReferenceOutcome {
  const op = "assurance.render_authored_argument";

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

  const shapeError = authoredArgumentViewShape(fields, "view");
  if (shapeError) {
    return failure(3, diagnostic("CORE_BAD_REQUEST", shapeError, { op }));
  }

  const rendered = renderAuthoredArgument(fields as never);
  return { exitCode: 0, payload: canonicalJson({ rendered }), diagnostics: [] };
}
