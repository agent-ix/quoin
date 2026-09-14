import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { carriesPayload, runCoreAllowFailure } from "../../core/exec.js";
import { canonicalizeJcs } from "../../integrity.js";

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
 * Lowercase hex of exact bytes, the encoding every document crosses the
 * `quoin-core` boundary in (quoin#457).
 *
 * A document is NOT pre-parsed on this side. The strict reader the sealed
 * contracts use takes its decisions — a duplicate member, a BOM, a non-finite
 * number, trailing content — over the producer's own bytes, and a value parsed
 * here would already have had a duplicate member resolved last-wins before the
 * engine ever saw it.
 */
export function hexOf(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString("hex");
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
 * Ask `quoin-core` one change-assurance question.
 *
 * Every refusal the engine reports — a malformed document (3), a contradicting
 * digest or an unretained selection (2), an engine fault (4) — reaches the
 * caller as this surface's own exit 2, because FR-068-AC-6 makes 1 mean "the
 * receipt is not valid" and nothing else. The engine's status and its
 * diagnostic codes are named in the message so the distinction is not lost,
 * only re-graded.
 *
 * `fail` is the command's `this.error`, which never returns.
 */
export function askCore(
  op: string,
  request: unknown,
  what: string,
  fail: (message: string) => never,
): Record<string, unknown> {
  const result = runCoreAllowFailure(op, request);
  if (!carriesPayload(result.exitCode)) {
    const detail = result.diagnostics
      .map((d) => `${d.code}: ${d.message}`)
      .join("\n");
    fail(
      `${what}: quoin-core ${op} exited ${result.exitCode}` +
        (detail ? `:\n${detail}` : " with no diagnostic on stderr."),
    );
  }
  return result.payload as Record<string, unknown>;
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
