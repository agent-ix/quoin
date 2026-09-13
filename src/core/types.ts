/**
 * The quoin-core boundary types (FR-097). GENERATED — DO NOT EDIT.
 *
 * Written by `quoin-schemas/quoin-schemas-gen` from the JSON Schema `schemars`
 * reads off the canonical Rust declarations in
 * `rust/crates/quoin-core/src/protocol.rs` and `.../src/ops/core.rs`.
 * Rust is the source of truth: where this file and a Rust type
 * disagree, the Rust type is right and this file is stale.
 *
 * Regenerate with `make types`. A hand edit does not survive review
 * and does not survive the gate: `quoin-schemas` asserts the digest
 * below against BOTH these bytes and a fresh render, so an edit here
 * fails `make rust-test` at an unchanged path, and a Rust type change
 * that was never regenerated fails it too.
 *
 * The machine-readable provenance is the `CORE_TYPES_PROVENANCE`
 * record below, and it is the ONLY copy: a second, prose copy up
 * here would be one more thing that can disagree with the artefact
 * it describes.
 */

/**
 * What produced this file, readable by the TypeScript side.
 *
 * Generated status is established by THIS record, not by the
 * artefact's directory name (FR-097-CON-2): moving the file does not
 * make it hand-written, and writing a file into a `generated/`
 * directory does not make it generated.
 */
export const CORE_TYPES_PROVENANCE = {
  generator: "quoin-schemas/quoin-schemas-gen",
  generatorVersion: "0.1.0",
  sourceSchemaSha256:
    "eecd4353aa98673bdf373a274c1037dbff46a87b8caee975cf50438dc4915920",
} as const;

/**
 * One entry of the stderr array.
 *
 * A `Serialize` struct rather than a hand-built `serde_json::Value`: the
 * field list is then reviewable, and the compiler checks that every branch
 * populated it.
 */
export interface Diagnostic {
  /**
   * The stable code, from the catalogued enum — never a literal invented
   * at the call site.
   */
  code: string;
  /**
   * Ordered context. `BTreeMap` for byte-stable serialisation.
   */
  context: Record<string, string>;
  /**
   * A sentence for an operator.
   */
  message: string;
}

/**
 * The payload `core.ping` writes to stdout.
 */
export interface PingPayload {
  /**
   * The `quoin-core` crate version.
   */
  core_version: string;
  /**
   * Whatever `echo` held, unchanged.
   */
  echo?: string | null;
  /**
   * The protocol revision this build speaks.
   */
  protocol_version: number;
}

/**
 * The request accepted by `core.ping`.
 *
 * `deny_unknown_fields` so a caller that misspells a field is refused rather
 * than silently ignored — a field the boundary drops is a field the caller
 * believes it sent.
 */
export interface PingRequest {
  /**
   * An opaque token returned unchanged, for correlating a call with its
   * answer across the pipe. Absent is fine.
   */
  echo?: string | null;
  /**
   * The protocol revision the caller believes it is speaking. When it
   * disagrees with this build's, the answer is still complete — it is how
   * the caller learns which revision it is actually talking to — so the
   * disagreement is reported as `Partial`, not as a
   * failure.
   */
  expect_protocol?: number | null;
}

/** The IPC protocol revision this build of the boundary speaks. */
export const PROTOCOL_VERSION = 1;
