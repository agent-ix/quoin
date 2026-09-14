/**
 * The shell's own canonical printing.
 *
 * Two spellings, both of them *output* formats and neither of them a decision:
 * `canonicalJson` is the two-space, key-sorted form a `--format json` command
 * prints, and `canonicalizeJcs` is the compact RFC 8785 form a machine-readable
 * change-assurance command prints. Determinism is the point in both — two runs
 * over identical inputs produce identical bytes, so a diff of the output *is*
 * the delta rather than serialization noise.
 *
 * # Why this is not an operation
 *
 * The boundary rule (ADR-0003) says the unit of IPC is a command-shaped
 * operation, never a function. `canonical_json` exists in `quoin-store` and is
 * what writes every retained byte; exposing it over IPC as well would create a
 * boundary that outlives every caller, to save two small pure functions that
 * format a payload the shell has already been handed.
 *
 * So these stay here, and Stage 9 deletes them with the oclif shell that needs
 * them. The retained *decisions* — the strict reader, the digests, the store
 * writer — left with `src/store/` at quoin#504; what is left is printing.
 */

/**
 * Canonical JSON: keys sorted at every level, two-space indent, trailing
 * newline.
 */
export function canonicalJson(value: unknown): string {
  return `${JSON.stringify(sortKeys(value), null, 2)}\n`;
}

function sortKeys(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortKeys);
  if (value === null || typeof value !== "object") return value;
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(value as Record<string, unknown>).sort()) {
    out[key] = sortKeys((value as Record<string, unknown>)[key]);
  }
  return out;
}

/**
 * Canonical JCS (RFC 8785): keys sorted, no whitespace, no trailing newline.
 *
 * It refuses rather than emitting something a strict reader would reject —
 * a non-finite number, an `undefined` member, a lone surrogate — because a
 * canonical form that can produce input `quoin-core` will not read is not
 * canonical. `quoin_store::canonicalize_jcs` takes the same refusals, and
 * `rust/crates/quoin-store/src/json/parse.rs` asserts them.
 */
export function canonicalizeJcs(value: unknown): string {
  if (value === null) return "null";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error("non-finite I-JSON number");
    return JSON.stringify(value);
  }
  if (typeof value === "string") {
    assertValidUnicode(value);
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) return `[${value.map(canonicalizeJcs).join(",")}]`;
  if (typeof value !== "object" || value === undefined) {
    throw new Error(`unsupported JSON value: ${typeof value}`);
  }
  const record = value as Record<string, unknown>;
  return `{${Object.keys(record)
    .sort()
    .map((key) => {
      if (record[key] === undefined) throw new Error(`undefined member ${key}`);
      assertValidUnicode(key);
      return `${JSON.stringify(key)}:${canonicalizeJcs(record[key])}`;
    })
    .join(",")}}`;
}

/**
 * Refuse a lone surrogate.
 *
 * This has no counterpart in `quoin-store` and deliberately gets none: a JS
 * string is UTF-16 code units with no well-formedness requirement, so a lone
 * surrogate is constructible here, while a Rust `String` is well-formed UTF-8,
 * so it is not. The Rust refusal lives in the parser instead, where a lone
 * surrogate can still be written as an escape. See
 * `rust/crates/quoin-store/COMPATIBILITY.md`.
 */
function assertValidUnicode(value: string): void {
  for (let index = 0; index < value.length; index++) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff)) {
        throw new Error("invalid Unicode lone high surrogate");
      }
      index++;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      throw new Error("invalid Unicode lone low surrogate");
    }
  }
}
