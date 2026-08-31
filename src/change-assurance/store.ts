/**
 * Content-integrity storage only. These functions run no producer, workflow,
 * Git, network, audit, or identity operation; recorded actor labels remain
 * attribution, while hashes provide content integrity only.
 */

import {
  closeSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeSync,
} from "node:fs";
import { dirname, join } from "node:path";

import { storeRoot } from "../evidence/store.js";
import { attestationBytes, verifyAttestation } from "./attestations.js";
import { blake3Hex, canonicalizeJcs, parseStrictJson } from "./integrity.js";
import { recordBytes, verifyChangeRecord } from "./records.js";
import type { ChangeAssuranceRecord, ProofAttestation } from "./types.js";

const FAMILY = "change-assurance";

export function changeAssuranceRoot(repo: string): string {
  return join(storeRoot(repo), FAMILY);
}

export function recordPath(repo: string, digest: string): string {
  assertPathDigest(digest);
  return join(changeAssuranceRoot(repo), "records", `${digest}.json`);
}

export function attestationPath(repo: string, digest: string): string {
  assertPathDigest(digest);
  return join(changeAssuranceRoot(repo), "attestations", digest);
}

export function writeChangeRecord(
  repo: string,
  record: ChangeAssuranceRecord,
): string {
  const bytes = recordBytes(record);
  const path = recordPath(repo, record.digest);
  mkdirSync(dirname(path), { recursive: true });
  if (existsSync(path)) {
    if (!equalBytes(readFileSync(path), bytes))
      throw new Error("record digest collision");
    return path;
  }
  const temporary = `${path}.tmp-${process.pid}-${Date.now()}`;
  writeDurable(temporary, bytes);
  renameSync(temporary, path);
  fsyncDirectory(dirname(path));
  return path;
}

export function readChangeRecord(
  repo: string,
  digest: string,
): ChangeAssuranceRecord | null {
  const path = recordPath(repo, digest);
  if (!existsSync(path)) return null;
  const value = parseStrictJson(readFileSync(path));
  const record = verifyChangeRecord(value);
  if (record.digest !== digest) throw new Error("record path digest mismatch");
  return record;
}

export interface IntakeOptions {
  /** Test-only failure seam immediately before the single visibility rename. */
  beforeRename?: () => void;
}

export function intakeAttestation(
  repo: string,
  rawAttestation: Uint8Array,
  output: Uint8Array,
  options: IntakeOptions = {},
): string {
  const parsed = parseStrictJson(rawAttestation);
  const attestation = verifyAttestation(parsed);
  const expectedBytes = attestationBytes(attestation);
  if (
    attestation.retained_output.size_bytes !== output.byteLength ||
    attestation.retained_output.digest !== blake3Hex(output)
  ) {
    throw new Error("retained output digest or size mismatch");
  }

  const finalDirectory = attestationPath(repo, attestation.digest);
  const parent = dirname(finalDirectory);
  mkdirSync(parent, { recursive: true });
  if (existsSync(finalDirectory)) {
    const existingAttestation = readFileSync(
      join(finalDirectory, "attestation.json"),
    );
    const existingOutput = readFileSync(join(finalDirectory, "output.bin"));
    if (
      !equalBytes(existingAttestation, expectedBytes) ||
      !equalBytes(existingOutput, output)
    ) {
      throw new Error("attestation digest collision");
    }
    return finalDirectory;
  }

  const staging = mkdtempSync(join(parent, ".tmp-attestation-"));
  try {
    writeDurable(join(staging, "attestation.json"), expectedBytes);
    writeDurable(join(staging, "output.bin"), output);
    fsyncDirectory(staging);
    options.beforeRename?.();
    renameSync(staging, finalDirectory);
    fsyncDirectory(parent);
  } catch (error) {
    // Leave the invisible staging directory for explicit recovery. It is never
    // interpreted as an attestation and cannot expose a half-pair.
    throw error;
  }
  return finalDirectory;
}

export function readAttestation(
  repo: string,
  digest: string,
): { attestation: ProofAttestation; output: Uint8Array } | null {
  const directory = attestationPath(repo, digest);
  if (!existsSync(directory)) return null;
  const raw = readFileSync(join(directory, "attestation.json"));
  const attestation = verifyAttestation(parseStrictJson(raw));
  const output = readFileSync(join(directory, "output.bin"));
  if (attestation.digest !== digest)
    throw new Error("attestation path digest mismatch");
  if (
    attestation.retained_output.size_bytes !== output.byteLength ||
    attestation.retained_output.digest !== blake3Hex(output)
  ) {
    throw new Error("stored output integrity mismatch");
  }
  return { attestation, output };
}

export function recoverChangeAssuranceStaging(repo: string): number {
  const parent = join(changeAssuranceRoot(repo), "attestations");
  if (!existsSync(parent)) return 0;
  const staging = readdirSync(parent).filter((name) =>
    name.startsWith(".tmp-attestation-"),
  );
  for (const name of staging)
    rmSync(join(parent, name), { recursive: true, force: true });
  return staging.length;
}

function writeDurable(path: string, bytes: Uint8Array): void {
  const descriptor = openSync(path, "wx");
  try {
    let offset = 0;
    while (offset < bytes.byteLength) {
      const written = writeSync(
        descriptor,
        bytes,
        offset,
        bytes.byteLength - offset,
      );
      if (written === 0)
        throw new Error(
          "short write while retaining change assurance evidence",
        );
      offset += written;
    }
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

function fsyncDirectory(path: string): void {
  const descriptor = openSync(path, "r");
  try {
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

function equalBytes(left: Uint8Array, right: Uint8Array): boolean {
  return (
    left.byteLength === right.byteLength &&
    left.every((byte, index) => byte === right[index])
  );
}

function assertPathDigest(value: string): void {
  if (!/^[a-f0-9]{64}$/.test(value)) {
    throw new Error(
      "change-assurance path requires a lowercase hexadecimal digest",
    );
  }
}

// Referenced by the static boundary test and documentation: integrity is not
// authentication, authorization, a signature, or a claim about a person.
export const CHANGE_ASSURANCE_INTEGRITY_BOUNDARY = canonicalizeJcs({
  claim: "content integrity and recorded attribution only",
});
