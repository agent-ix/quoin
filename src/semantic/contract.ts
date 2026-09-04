/**
 * The semantic-module contract quoin was written against (FR-070, FR-073,
 * FR-075; issue #293).
 *
 * Three families of schema are vendored here, each with recorded provenance,
 * for the same reason the Quire output schemas are (`src/quire/contract.ts`):
 * there is no dependency edge along which the files could travel, and quoin
 * performs no network read on a command path.
 *
 * - The module-manifest schema is owned by filament-core-service (FR-035;
 *   CR-003 added the `semantic` block, agent-ix/filament-core-service#21).
 * - The semantic-core JSON Schema bundle is the official TypeSpec projection
 *   of `@agent-ix/semantic-core` (filament-core-data FR-033), private until
 *   filament-core-data#11 publishes it.
 * - The filament-core-data package-manifest and common schemas (FR-021) are
 *   what the derived package manifest (FR-075) validates against.
 *
 * Every hash below is asserted on each test run; the refresh scripts re-derive
 * them from the exact pinned git objects, so an edit to a vendored file without
 * a matching refresh fails immediately.
 */

import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
export const SCHEMA_DIR = join(here, "schemas");

/** A vendored file's origin: repository, exact commit, path there, and bytes. */
export interface VendoredSource {
  readonly repository: string;
  readonly sourceRevision: string;
  readonly sourcePath: string;
  readonly sha256: string;
}

export const SEMANTIC_CONTRACT = {
  /** The `semantic.contract_version` this quoin understands (FR-070). */
  contractVersion: "1.0.0",
  /** The `semantic.semantic_core` versions this quoin ships a bundle for. */
  semanticCoreVersions: ["0.1.0"] as const,
  /** The ten admitted `semantic` keys (FR-070). */
  semanticKeys: [
    "contract_version",
    "semantic_core",
    "package",
    "exports",
    "imports",
    "targets",
    "mappings",
    "compatibility_posture",
    "legacy_forms",
    "sweep_report",
  ] as const,
  moduleManifestSchema: {
    repository: "agent-ix/filament-core-service",
    sourceRevision: "a77f31efc757f3578ad80d8c7e619897aa3b2513",
    sourcePath: "filament_core_service/schemas/module-manifest.schema.json",
    sha256:
      "sha256:69cf9738600e7d8daa45ed5cd7231b17ca8dc58d068bd36af9b0d2c9b69dcbbc",
  } satisfies VendoredSource,
  /**
   * The semantic-core bundle: one directory per version under
   * `schemas/semantic-core/`. `bundleDigest` equals the `digest` recorded in
   * filament-core-data's `packages/semantic-core/generated/toolchain.json` at
   * the pinned revision, computed the same way (name + newline + bytes, in
   * sorted file order).
   */
  semanticCore: {
    repository: "agent-ix/filament-core-data",
    sourceRevision: "d48b8da7ae5e40b8b3d465d45b2bd3e24b994dbb",
    sourcePath: "packages/semantic-core/generated/json-schema",
    version: "0.1.0",
    bundleDigest:
      "sha256:dd33c886f70e908b14507c35e078d163b76308c3d170d2b54ddf933d1a4ebb52",
  },
  packageManifestSchema: {
    repository: "agent-ix/filament-core-data",
    sourceRevision: "d48b8da7ae5e40b8b3d465d45b2bd3e24b994dbb",
    sourcePath: "schema/semantic/v1/package-manifest.schema.json",
    sha256:
      "sha256:d6e696577f58abd59c36588803c019ad3a43f9a7078c873ad41a0aec41031ffd",
  } satisfies VendoredSource,
  commonSchema: {
    repository: "agent-ix/filament-core-data",
    sourceRevision: "d48b8da7ae5e40b8b3d465d45b2bd3e24b994dbb",
    sourcePath: "schema/semantic/v1/common.schema.json",
    sha256:
      "sha256:1de370f344b099b511960c32ddc98d512218183c13b03350201627bdcba7710a",
  } satisfies VendoredSource,
} as const;

export function fileSha256(path: string): string {
  return `sha256:${createHash("sha256").update(readFileSync(path)).digest("hex")}`;
}

export function moduleManifestSchemaPath(): string {
  return join(SCHEMA_DIR, "module-manifest.schema.json");
}

export function semanticCoreDir(): string {
  return join(SCHEMA_DIR, "semantic-core");
}

/** The semantic-core bundle digest, computed exactly as filament-core-data does. */
export function semanticCoreBundleDigest(dir = semanticCoreDir()): string {
  const digest = createHash("sha256");
  for (const name of readdirSync(dir)
    .filter((n) => n.endsWith(".json") && n !== "toolchain.json")
    .sort()) {
    digest.update(`${name}\n${readFileSync(join(dir, name), "utf8")}`);
  }
  return `sha256:${digest.digest("hex")}`;
}

export function packageManifestSchemaPath(): string {
  return join(SCHEMA_DIR, "filament-core-data", "package-manifest.schema.json");
}

export function sweepReportSchemaPath(): string {
  return join(here, "sweep-report.schema.json");
}

export function commonSchemaPath(): string {
  return join(SCHEMA_DIR, "filament-core-data", "common.schema.json");
}

/** Read a vendored JSON document. */
export function readJson(path: string): unknown {
  return JSON.parse(readFileSync(path, "utf8"));
}
