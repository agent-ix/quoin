/**
 * The quire↔quoin JSON contract (FR-029).
 *
 * quire-rs FR-055 publishes a versioned JSON Schema for each machine-readable
 * payload it emits. This module is the consumer side: it pins the contract
 * version quoin was written against, vendors the published schemas, and
 * enforces the version premise before anything reads a payload.
 *
 * ## Why the schemas are vendored
 *
 * quire-rs is a Rust crate consumed by git tag; quoin is an npm package. There
 * is no dependency edge along which a schema file could travel, and fetching
 * one at runtime is out of the question (quoin performs no network reads on a
 * command path). So the artifacts are **copied in with their provenance
 * recorded** — source revision, path and content hash — and refreshed deliberately
 * by `scripts/refresh-quire-schemas.mjs`, whose diff is reviewed.
 *
 * That is a copy, and a copy can drift. What keeps it honest is that the hash
 * is asserted on every test run and the refresh script re-derives it from the
 * exact pinned quire-rs git object: an edit to the vendored file without
 * a matching refresh fails immediately, rather than silently teaching quoin a
 * contract quire does not emit.
 */

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** Where the vendored schemas came from, and exactly which bytes. */
export const QUIRE_CONTRACT = {
  /** Exact quire-rs commit whose schemas were copied. */
  sourceRevision: "ca7362d4dacecb96f01d74d1d971327118c25917",
  /** The contract version, as carried in each schema's `$id` and filename. */
  contractVersion: "v1",
  /**
   * Minimum `quire` CLI that emits this contract.
   *
   * v0.21.0 is the first release carrying the FR-053 `obligation` field on
   * property records and the FR-055 published schemas. An older binary emits a
   * payload this contract does not describe — which is a diagnosable state, not
   * a parse error to discover three frames deep.
   */
  minimumCli: "0.21.0",
  /*
   * NOTE: `minimumCli` is a CONTRACT floor, not a CAPABILITY floor, and the
   * two have already drifted once.
   *
   * 0.21.0 is still correct here: it is the first release emitting the shapes
   * this file describes, and raising it would reject a CLI that satisfies the
   * contract. But a consumer on 0.22.0 silently gets no FR-059 vocabulary
   * coverage and no FR-061 combinatorial obligations — the payload parses,
   * and simply contains less. quoin cannot tell that from a corpus that
   * declares neither.
   *
   * quire-cli sat five engine releases behind for exactly this reason: every
   * gate passed, the CLI kept working, and it answered from an older engine.
   * A feature needing a specific capability should check for it rather than
   * assume this number covers it.
   */
  /**
   * SHA-256 of each vendored file. Asserted on every test run, so an edit
   * without a matching refresh fails loudly.
   *
   * The revision is pinned rather than described as a release because the QA
   * program intentionally runs both projects from their tracking branches.
   * The refresh helper reads the files from that git object, so unrelated
   * working-tree changes cannot alter the recorded contract.
   */
  hashes: {
    "coverage-v1.schema.json":
      "f0cdbb9457ca7ca76b642c11fd5b6f41273909c894458c31f58c9c50fdbcdb38",
    "properties-v1.schema.json":
      "cc687773c35b3e71e82fc336e887ecc688a28b03e970e349b2921b8624e4d11c",
  },
} as const;

export type SchemaName = keyof typeof QUIRE_CONTRACT.hashes;

const here = dirname(fileURLToPath(import.meta.url));

/** Absolute path of a vendored schema. */
export function schemaPath(name: SchemaName): string {
  return join(here, "schemas", name);
}

/** Read a vendored schema as parsed JSON. */
export function readSchema(name: SchemaName): unknown {
  return JSON.parse(readFileSync(schemaPath(name), "utf8"));
}

/** SHA-256 of a vendored schema as it sits on disk. */
export function schemaHash(name: SchemaName): string {
  return createHash("sha256")
    .update(readFileSync(schemaPath(name)))
    .digest("hex");
}

/**
 * A CLI version that does not satisfy the contract's premise.
 *
 * Carried as a distinct shape so a caller can report *which* premise failed
 * rather than "something went wrong reading quire output" — the failure mode
 * agent-ix/quoin#88 was filed about, where a shape drift surfaced as an
 * unexplained mid-skill error.
 */
export interface VersionPremiseFailure {
  readonly kind: "version-premise";
  readonly found: string | null;
  readonly required: string;
  readonly message: string;
}

/** Parse `quire --version` output (`quire 0.21.0`) into a version string. */
export function parseCliVersion(output: string): string | null {
  const match = /(\d+)\.(\d+)\.(\d+)/.exec(output);
  return match ? match[0] : null;
}

/** Numeric semver comparison. Returns <0, 0, >0. */
export function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map((n) => Number.parseInt(n, 10));
  const pb = b.split(".").map((n) => Number.parseInt(n, 10));
  for (let i = 0; i < 3; i += 1) {
    const diff = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

/**
 * Check a `quire --version` output against the contract's minimum.
 *
 * Returns `null` when the premise holds. Returns a named failure otherwise —
 * including when the version could not be parsed at all, because "no quire on
 * PATH" and "a quire too old to emit this shape" are both states a caller must
 * report rather than push into a downstream parse.
 */
export function checkVersionPremise(
  versionOutput: string | null,
): VersionPremiseFailure | null {
  const found = versionOutput ? parseCliVersion(versionOutput) : null;
  const required = QUIRE_CONTRACT.minimumCli;
  if (found === null) {
    return {
      kind: "version-premise",
      found: null,
      required,
      message:
        `could not determine the quire CLI version (expected >= ${required}). ` +
        `Install or update quire-cli: the JSON contract quoin reads is only ` +
        `emitted from ${required} onward.`,
    };
  }
  if (compareVersions(found, required) < 0) {
    return {
      kind: "version-premise",
      found,
      required,
      message:
        `quire ${found} is older than the ${required} this contract requires. ` +
        `Its JSON payloads predate the published schemas (quire-rs FR-055) and ` +
        `the obligation records (FR-053), so quoin would misread them rather ` +
        `than fail. Update quire-cli.`,
    };
  }
  return null;
}
