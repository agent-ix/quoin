import { assertDigest, canonicalBytes, digestValue } from "./integrity.js";
import {
  array,
  digest,
  exact,
  identity,
  nonempty,
  object,
  compareUtf16,
  validateCommand,
} from "./records.js";
import type { ProofAttestation } from "./types.js";

export function sealAttestation(
  input:
    | Omit<ProofAttestation, "digest">
    | (Omit<ProofAttestation, "digest"> & { digest?: undefined }),
): ProofAttestation {
  const rest = { ...input } as Record<string, unknown>;
  delete rest.digest;
  const normalized = normalizeAttestation(
    rest as unknown as Omit<ProofAttestation, "digest">,
  );
  validateAttestationShape(normalized, false);
  const digest = digestValue(normalized as unknown as Record<string, unknown>);
  return verifyAttestation({ ...normalized, digest });
}

export function verifyAttestation(value: unknown): ProofAttestation {
  validateAttestationShape(value, true);
  assertDigest(value as unknown as Record<string, unknown>);
  return value as ProofAttestation;
}

export function attestationBytes(value: ProofAttestation): Uint8Array {
  return canonicalBytes(verifyAttestation(value));
}

function normalizeAttestation(
  input: Omit<ProofAttestation, "digest">,
): Omit<ProofAttestation, "digest"> {
  const value = structuredClone(input);
  const environment = Object.fromEntries(
    Object.entries(value.environment).sort(([left], [right]) =>
      compareUtf16(left, right),
    ),
  );
  return { ...value, environment };
}

function validateAttestationShape(
  value: unknown,
  sealed: boolean,
): asserts value is ProofAttestation {
  const root = object(value, "attestation");
  exact(root, [
    "schema_version",
    "record_type",
    "attestation_id",
    ...(sealed ? ["digest"] : []),
    "record_digest",
    "candidate_revision",
    "proof_id",
    "command",
    "tool",
    "environment",
    "observed_at",
    "result",
    "retained_output",
  ]);
  if (root.schema_version !== 1) fail("schema_version");
  if (root.record_type !== "proof_attestation") fail("record_type");
  identity(root.attestation_id, "attestation_id");
  if (sealed) digest(root.digest, "digest");
  digest(root.record_digest, "record_digest");
  nonempty(root.candidate_revision, "candidate_revision");
  identity(root.proof_id, "proof_id");
  validateCommand(root.command);

  const tool = object(root.tool, "tool");
  exact(tool, ["identity", "version", "configuration_digest"]);
  nonempty(tool.identity, "tool.identity");
  const version = nonempty(tool.version, "tool.version");
  if (
    !/^(?:v?\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|[a-f0-9]{64})$/.test(
      version,
    )
  ) {
    fail("tool.version is not immutable");
  }
  digest(tool.configuration_digest, "tool.configuration_digest");

  const environment = object(root.environment, "environment");
  if (Object.keys(environment).length === 0) fail("environment");
  for (const [key, entry] of Object.entries(environment)) {
    if (
      entry !== null &&
      !["string", "number", "boolean"].includes(typeof entry)
    ) {
      fail(`environment.${key}`);
    }
    if (typeof entry === "number" && !Number.isFinite(entry))
      fail(`environment.${key}`);
  }
  const observed = nonempty(root.observed_at, "observed_at");
  if (
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(
      observed,
    ) ||
    Number.isNaN(Date.parse(observed))
  ) {
    fail("observed_at");
  }
  if (
    !["passed", "failed", "unavailable", "not_computed"].includes(
      root.result as string,
    )
  ) {
    fail("result");
  }

  const output = object(root.retained_output, "retained_output");
  exact(output, ["media_type", "digest", "size_bytes"]);
  nonempty(output.media_type, "retained_output.media_type");
  digest(output.digest, "retained_output.digest");
  if (
    !Number.isInteger(output.size_bytes) ||
    (output.size_bytes as number) < 0
  ) {
    fail("retained_output.size_bytes");
  }

  // Preserve this explicit array touch as a guard against accepting arrays as
  // environment objects if the shared object validator ever changes.
  if (Array.isArray(root.environment))
    array(root.environment, "environment", true);
}

function fail(message: string): never {
  throw new Error(`invalid proof attestation: ${message}`);
}
