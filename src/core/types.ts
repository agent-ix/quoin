/**
 * The quoin-core boundary types (FR-097). GENERATED — DO NOT EDIT.
 *
 * Written by `quoin-schemas/quoin-schemas-gen` from the JSON Schema `schemars`
 * reads off the canonical Rust declarations in
 * `rust/crates/quoin-core/src/protocol.rs`, `.../src/ops/` and the
 * domain crates those operations answer from.
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
    "7579931f5172a657e54f65945660187bf844014ead61e25869348a06f95b332c",
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
 * One located gate-that-gates-nothing defect.
 *
 * `subject`, `changeTarget`, `remedy` and `summary` are advisory prose for the
 * operator. Verdict parity is defined on `(kind, obligation, path, line,
 * wiredBy)`; the prose is reproduced faithfully because it is part of the
 * emitted payload, not because its wording is contractual.
 */
export interface EmptyGateFinding {
  /**
   * `path:line`, the exact locus an operator must edit.
   */
  changeTarget: string;
  /**
   * Always `GateThatGatesNothing` today.
   */
  kind: FindingKind;
  /**
   * The 1-based line of the unasserted count.
   */
  line: LineNumber;
  /**
   * The obligation the gate comment claims to enforce.
   */
  obligation: ObligationId;
  /**
   * The shell script holding the unasserted count.
   */
  path: RepoPath;
  /**
   * What to change.
   */
  remedy: string;
  /**
   * Human label for the gate.
   */
  subject: string;
  /**
   * The full three-way join, stated once.
   */
  summary: string;
  /**
   * The build or CI file that wires the script, proving it is a gate.
   */
  wiredBy: RepoPath;
}

/**
 * The class of defect a finding reports.
 *
 * One variant today. It is an enum and not a `&'static str` because the kind is
 * the payload's discriminant: a second validator adds a variant here and every
 * `match` on it becomes a compiler-checked edit site.
 */
export type FindingKind = "gate-that-gates-nothing";

/**
 * A 1-based line number inside a source file.
 *
 * `NonZeroU64` rather than `usize`: line 0 does not exist, and the payload
 * crosses a JSON boundary where a 0 would be read as "unknown".
 */
export type LineNumber = number;

/**
 * A requirement obligation as it was written in the gate comment, e.g.
 * `FR-001-AC-1`.
 *
 * The claim regex is case-insensitive, so this deliberately preserves the
 * author's spelling rather than normalising it: the finding must point at what
 * the file actually says.
 */
export type ObligationId = string;

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

/**
 * A path relative to the repository root, always written with `/` separators.
 *
 * Findings are compared, sorted, and matched against build wiring by this
 * string, so the separator normalisation is part of the identity and not a
 * display concern — a Windows-shaped `scripts\gate.sh` and a POSIX
 * `scripts/gate.sh` are the same gate.
 */
export type RepoPath = string;

/**
 * The payload `validators.run` writes to stdout.
 *
 * One field, named `findings`, because that is the byte shape
 * `quoin validate --json` has always emitted and the cutover is not licence to
 * change a user-visible document.
 */
export interface RunPayload {
  /**
   * Every finding, ordered by `(path, line, obligation)`.
   */
  findings: EmptyGateFinding[];
}

/**
 * The request accepted by `validators.run`.
 *
 * A snapshot of the repository, not a path to it. `deny_unknown_fields` so a
 * caller that misspells a field is refused rather than silently ignored — a
 * field the boundary drops is a field the caller believes it sent.
 */
export interface RunRequest {
  /**
   * Every file the caller found, keyed by repository-relative,
   * `/`-separated path, holding the file's lines split on `\n`.
   *
   * `null` means the caller found the path but could not read it. That is
   * not the same as omitting it: an unreadable file still classifies as a
   * shell script or as wiring, and refuses only if the analysis reaches for
   * its text — which is the on-demand read the TypeScript oracle performs.
   *
   * Lines rather than one string because that is the shape the golden corpus
   * already carries. Joining the lines with a newline reconstructs the file,
   * so a caller must split on a newline alone and leave any carriage return
   * on the line.
   */
  files: Record<string, string[] | null>;
  /**
   * Directories the caller could not list, repository-relative. The empty
   * string names the repository root itself.
   *
   * Separate from an unreadable file because the walk **aborts** on one: a
   * subtree nobody could read means the answer would be computed over a
   * repository nobody has seen, and an empty result must mean the validator
   * looked and found nothing.
   */
  unlistable?: string[];
}

/** The IPC protocol revision this build of the boundary speaks. */
export const PROTOCOL_VERSION = 1;
