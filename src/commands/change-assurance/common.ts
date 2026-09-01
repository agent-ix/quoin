import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import {
  canonicalizeJcs,
  parseStrictJson,
} from "../../change-assurance/index.js";

/** Repository root holding the evidence store. */
export const repoFlag = Flags.string({
  description: "Repository root for retained Quoin state.",
  default: ".",
});

/** Canonical machine output. */
export const jsonFlag = Flags.boolean({ description: "Emit canonical JSON." });

/**
 * Read one command input as exact bytes. `-` reads standard input.
 *
 * The bytes are returned unmodified so intake can retain what the producer
 * actually supplied rather than a re-serialized copy of it.
 */
export function readInputBytes(source: string): Uint8Array {
  return source === "-" ? readFileSync(0) : readFileSync(source);
}

/**
 * Read one command input as strictly parsed JSON.
 *
 * Strict parsing is the same reader the sealed contracts use, so a duplicate
 * key or a non-finite number is refused here rather than silently normalized
 * into a record that then seals cleanly.
 */
export function readInputJson(source: string): unknown {
  return parseStrictJson(readInputBytes(source));
}

/** Canonical JCS text for machine output, with a trailing newline. */
export function canonicalOutput(value: unknown): string {
  return canonicalizeJcs(value);
}

/** The message of an unknown thrown value, for a `this.error` hand-off. */
export function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * Refuse a body that supplies a field the caller must not state.
 *
 * The sealing functions delete a supplied `digest` before hashing, so without
 * this check a caller could hand in a wrong digest and receive a cleanly
 * sealed record back, having been told nothing.
 */
export function refuseSuppliedFields(
  value: unknown,
  fields: readonly string[],
): string | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return "input must be a JSON object";
  }
  const supplied = fields.filter((field) =>
    Object.prototype.hasOwnProperty.call(value, field),
  );
  if (supplied.length === 0) return null;
  return `input must not supply ${supplied.join(", ")}; it is derived when sealing`;
}

/** Parse one repeated `--select <proof-id>=<attestation-digest>` mapping. */
export function parseSelection(
  raw: string,
): { proof_id: string; attestation_digest: string } | null {
  const separator = raw.indexOf("=");
  if (separator <= 0) return null;
  const proofId = raw.slice(0, separator).trim();
  const attestationDigest = raw.slice(separator + 1).trim();
  if (proofId === "" || !/^[a-f0-9]{64}$/.test(attestationDigest)) return null;
  return { proof_id: proofId, attestation_digest: attestationDigest };
}
