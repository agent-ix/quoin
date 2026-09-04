/**
 * Semantic-module contract vendoring (issue #293, TASK-039).
 *
 * The vendored schemas are copies; what keeps them honest is that every hash
 * in `SEMANTIC_CONTRACT` is asserted here and re-derived by the refresh
 * scripts from the exact pinned git objects.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import {
  SEMANTIC_CONTRACT,
  commonSchemaPath,
  fileSha256,
  moduleManifestSchemaPath,
  packageManifestSchemaPath,
  readJson,
  semanticCoreBundleDigest,
  semanticCoreDir,
} from "../src/semantic/contract.js";

type Json = Record<string, unknown>;

function requiredArrays(node: unknown, path = ""): Map<string, string[]> {
  const found = new Map<string, string[]>();
  if (Array.isArray(node)) {
    node.forEach((child, index) => {
      for (const [k, v] of requiredArrays(child, `${path}/${index}`))
        found.set(k, v);
    });
  } else if (node && typeof node === "object") {
    const record = node as Json;
    if (
      Array.isArray(record.required) &&
      record.required.every((item) => typeof item === "string")
    ) {
      found.set(path, [...(record.required as string[])]);
    }
    for (const [key, child] of Object.entries(record)) {
      for (const [k, v] of requiredArrays(child, `${path}/${key}`))
        found.set(k, v);
    }
  }
  return found;
}

describe("semantic-module contract vendoring", () => {
  // Trace: FR-070-CON-2
  // Trace: TC-1343
  it("records filament-core-service provenance for the module-manifest schema and the bytes match", () => {
    const record = SEMANTIC_CONTRACT.moduleManifestSchema;
    expect(record.repository).toBe("agent-ix/filament-core-service");
    expect(record.sourceRevision).toMatch(/^[0-9a-f]{40}$/);
    expect(record.sourcePath).toBe(
      "filament_core_service/schemas/module-manifest.schema.json",
    );
    expect(fileSha256(moduleManifestSchemaPath())).toBe(record.sha256);
    const schema = readJson(moduleManifestSchemaPath()) as Json;
    const semantic = (schema.properties as Json).semantic as Json;
    expect(semantic.additionalProperties).toBe(false);
    expect(Object.keys(semantic.properties as Json).sort()).toEqual(
      [...SEMANTIC_CONTRACT.semanticKeys].sort(),
    );
  });

  // Trace: FR-070-CON-1
  // Trace: TC-1342
  // Trace: NFR-017-AC-4
  // Trace: TC-1382
  it("adds no required key anywhere versus the pre-CR-003 schema", () => {
    const before = requiredArrays(
      JSON.parse(
        readFileSync(
          join(
            "tests",
            "fixtures",
            "semantic-module",
            "vendored",
            "module-manifest.schema.pre-cr003.json",
          ),
          "utf8",
        ),
      ),
    );
    const after = requiredArrays(readJson(moduleManifestSchemaPath()));
    for (const [path, required] of before) {
      expect(after.get(path), path).toEqual(required);
    }
    const added = [...after.keys()].filter((path) => !before.has(path));
    for (const path of added) {
      expect(
        path.includes("/properties/semantic") || path.includes("/data_schema/"),
        `new required array outside the optional nodes: ${path}`,
      ).toBe(true);
    }
    expect(after.get("")).toEqual(["manifest_version", "name", "version"]);
  });

  // Trace: FR-073-AC-6
  // Trace: TC-1385
  it("vendors the semantic-core bundle whose digest equals filament-core-data toolchain.json", () => {
    const record = SEMANTIC_CONTRACT.semanticCore;
    expect(semanticCoreBundleDigest()).toBe(record.bundleDigest);
    const toolchain = readJson(
      join(semanticCoreDir(), "toolchain.json"),
    ) as Json;
    expect(toolchain.digest).toBe(record.bundleDigest);
    expect(toolchain.base).toBe(
      `https://schemas.agent-ix.org/semantic-core/${record.version}/`,
    );
    const names = readdirSync(semanticCoreDir())
      .filter((n) => n.endsWith(".json") && n !== "toolchain.json")
      .sort();
    expect(names).toEqual([...(toolchain.files as string[])].sort());
    for (const name of [
      "FieldDecl.json",
      "TypeRef.json",
      "ClauseRef.json",
      "OperationDecl.json",
    ]) {
      expect(names).toContain(name);
    }
    expect(SEMANTIC_CONTRACT.semanticCoreVersions).toContain(record.version);
  });

  // Trace: FR-075-AC-1
  it("vendors the filament-core-data package-manifest and common schemas with matching hashes", () => {
    expect(fileSha256(packageManifestSchemaPath())).toBe(
      SEMANTIC_CONTRACT.packageManifestSchema.sha256,
    );
    expect(fileSha256(commonSchemaPath())).toBe(
      SEMANTIC_CONTRACT.commonSchema.sha256,
    );
    const common = readJson(commonSchemaPath()) as Json;
    const target = ((common.$defs as Json).target as Json).enum as string[];
    expect(target).toEqual([
      "json-schema",
      "rust",
      "typescript",
      "python-pydantic-v2",
      "python-dataclass",
    ]);
  });
});
