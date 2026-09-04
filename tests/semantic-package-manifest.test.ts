/**
 * Derived package manifests and registry pins (FR-075; issue #293, TASK-042).
 */

import { createHash } from "node:crypto";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { parse as parseYaml, stringify as stringifyYaml } from "yaml";

import { loadCatalog } from "../src/catalog.js";
import { installPlugin, semanticPin } from "../src/plugins.js";
import {
  derivePackageManifest,
  exportDigests,
  readModuleSemantic,
  resolveImports,
  typeIdentity,
  validatePackageManifest,
} from "../src/semantic/index.js";

const FIXTURE = join("tests", "fixtures", "semantic-module", "module-ok");
type Json = Record<string, unknown>;

let scratch: string;
let home: string;

function moduleCopy(
  name: string,
  mutate?: (manifest: Json, root: string) => void,
): string {
  const root = join(scratch, name);
  cpSync(FIXTURE, root, { recursive: true });
  const manifestPath = join(root, "manifest.yaml");
  const manifest = parseYaml(readFileSync(manifestPath, "utf8")) as Json;
  manifest.name = name;
  mutate?.(manifest, root);
  writeFileSync(manifestPath, stringifyYaml(manifest));
  return root;
}

/** Re-point the fixture at another package: rewrite the schema `$id` and its digest. */
function retarget(
  manifest: Json,
  moduleRoot: string,
  packageIdentity: string,
  edit?: (schema: Json) => void,
): void {
  const file = join(moduleRoot, "schemas", "Entity.json");
  const schema = JSON.parse(readFileSync(file, "utf8")) as Json;
  schema.$id = `https://schemas.agent-ix.org/${packageIdentity}/0.1.0/Entity.json`;
  edit?.(schema);
  writeFileSync(file, JSON.stringify(schema));
  ((manifest.object_types as Json[])[0].data_schema as Json).digest =
    `sha256:${createHash("sha256").update(readFileSync(file)).digest("hex")}`;
  (manifest.semantic as Json).package = packageIdentity;
}

beforeEach(() => {
  scratch = mkdtempSync(join(tmpdir(), "quoin-pkg-"));
  home = join(scratch, "home");
  mkdirSync(home, { recursive: true });
});

afterEach(() => {
  rmSync(scratch, { recursive: true, force: true });
});

describe("FR-075 package manifest derivation and registry pins", () => {
  // Trace: FR-075-AC-1
  // Trace: TC-1372
  it("derives a package manifest that validates against the vendored filament-core-data schema", () => {
    const root = moduleCopy("derive");
    const module = readModuleSemantic(root).module;
    expect(module).toBeDefined();
    const manifest = derivePackageManifest(module!);
    const verdict = validatePackageManifest(manifest);
    expect(verdict.valid, JSON.stringify(verdict.errors)).toBe(true);
    expect(manifest).toMatchObject({
      contractVersion: "1.0.0",
      package: { identity: "agent-ix/spec-objects-fixture", version: "0.1.0" },
      schemaDialect: "https://json-schema.org/draft/2020-12/schema",
      sourceRoots: ["schemas/"],
      targets: ["json-schema", "markdown"],
    });
    expect(manifest.imports).toEqual([
      {
        packageIdentity: "agent-ix/semantic-core",
        versionConstraint: "=0.1.0",
        exports: [],
        capabilities: [],
      },
    ]);
    expect(manifest.exports).toEqual([
      {
        name: "entity",
        typeIdentity: "ix://agent-ix/spec-objects-fixture/type/entity",
        visibility: "public",
      },
    ]);
    const profile = (manifest.profiles as Json[])[0] as Json;
    expect(profile).toMatchObject({
      name: "default",
      version: "0.1.0",
      exports: ["entity"],
      compatibilityPosture: "additive",
      options: {},
    });
    // Installing writes it beside the module.
    installPlugin(`path:${root}`, home);
    const written = join(
      home,
      "filament",
      "modules",
      "derive",
      "semantic",
      "package-manifest.json",
    );
    expect(existsSync(written)).toBe(true);
    expect(JSON.parse(readFileSync(written, "utf8"))).toEqual(
      derivePackageManifest(
        readModuleSemantic(join(home, "filament", "modules", "derive")).module!,
      ),
    );
  });

  // Trace: FR-075-AC-2
  // Trace: TC-1373
  it("pins one digest per exported object type in registry.json and changes when the schema changes", () => {
    const root = moduleCopy("pins");
    installPlugin(`path:${root}`, home);
    const pin = semanticPin("pins", home);
    expect(pin?.package).toBe("agent-ix/spec-objects-fixture");
    expect(pin?.semanticCore).toBe("0.1.0");
    expect(Object.keys(pin?.exports ?? {})).toEqual(["entity"]);
    expect(pin?.exports.entity).toMatch(/^sha256:[0-9a-f]{64}$/);
    // A changed emitted schema (with a matching digest in the manifest) yields a different pin.
    const changed = moduleCopy("pins-changed", (manifest, moduleRoot) => {
      retarget(
        manifest,
        moduleRoot,
        "agent-ix/spec-objects-fixture-2",
        (schema) => {
          schema.description = "changed";
        },
      );
    });
    installPlugin(`path:${changed}`, home);
    expect(semanticPin("pins-changed", home)?.exports.entity).not.toBe(
      pin?.exports.entity,
    );
    expect(exportDigests(readModuleSemantic(root).module!)).toEqual({
      entity: pin?.exports.entity,
    });
  });

  // Trace: FR-075-AC-3
  // Trace: TC-1374
  it("fails install on an import no installed module provides, naming versions, and on an import cycle", () => {
    const needy = moduleCopy("needy", (m) => {
      (m.semantic as Json).imports = { "agent-ix/spec-objects-other": "0.2.0" };
    });
    expect(() => installPlugin(`path:${needy}`, home)).toThrow(
      /semantic\.import-unresolved.*installed: none/s,
    );
    const provider = moduleCopy("provider", (m, moduleRoot) => {
      retarget(m, moduleRoot, "agent-ix/spec-objects-other");
      m.version = "0.1.0";
    });
    installPlugin(`path:${provider}`, home);
    expect(() => installPlugin(`path:${needy}`, home)).toThrow(
      /installed: 0\.1\.0/,
    );
    // Cycle: candidate imports provider which imports candidate.
    const a = readModuleSemantic(provider).module!;
    a.block.imports = { "agent-ix/spec-objects-fixture": "0.1.0" };
    const b = readModuleSemantic(
      moduleCopy("cyclic", (m) => {
        (m.semantic as Json).imports = {
          "agent-ix/spec-objects-other": "0.1.0",
        };
      }),
    ).module!;
    const diagnostics = resolveImports(b, [a]);
    expect(diagnostics.map((d) => d.code)).toContain("semantic.import-cycle");
    expect(
      diagnostics.find((d) => d.code === "semantic.import-cycle")?.message,
    ).toContain(
      "agent-ix/spec-objects-fixture -> agent-ix/spec-objects-other -> agent-ix/spec-objects-fixture",
    );
  });

  // Trace: FR-075-AC-4
  // Trace: TC-1375
  it("exposes the same object-type identities in the dynamic load and the derived exports", () => {
    const root = moduleCopy("parity");
    const catalog = loadCatalog([root]);
    const module = catalog.modules[0]!;
    const dynamic = module.objectTypes
      .filter((name) => module.semantic?.exports.includes(name))
      .map((name) => typeIdentity(module.semantic!.package, name));
    const derived = (
      derivePackageManifest(readModuleSemantic(root).module!).exports as Json[]
    ).map((e) => e.typeIdentity);
    expect(derived).toEqual(dynamic);
    expect(dynamic).toEqual(["ix://agent-ix/spec-objects-fixture/type/entity"]);
  });

  // Trace: FR-075-AC-5
  // Trace: TC-1376
  // Trace: FR-075-CON-2
  // Trace: TC-1378
  it("rejects a URL or ix:// package and derives ix:// type identities from the org/repo package", () => {
    for (const value of ["ix://agent-ix/x", "https://example.org/pkg"]) {
      const root = moduleCopy("bad-pkg", (m) => {
        (m.semantic as Json).package = value;
      });
      expect(readModuleSemantic(root).diagnostics.map((d) => d.code)).toContain(
        "semantic.invalid-package",
      );
    }
    expect(typeIdentity("agent-ix/spec-objects-business", "entity")).toBe(
      "ix://agent-ix/spec-objects-business/type/entity",
    );
  });

  // Trace: FR-075-CON-1
  // Trace: TC-1377
  it("compiles, publishes, and fetches nothing", () => {
    const source = readFileSync(
      join("src", "semantic", "package-manifest.ts"),
      "utf8",
    );
    expect(source).not.toMatch(/node:http|fetch\(|npm publish|child_process/);
  });
});
