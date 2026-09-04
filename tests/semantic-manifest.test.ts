/**
 * The `semantic` manifest block and `data_schema` references at install time
 * (FR-070, FR-073; issue #293, TASK-040).
 */

import { createHash } from "node:crypto";
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { readdirSync } from "node:fs";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { parse as parseYaml, stringify as stringifyYaml } from "yaml";

import { defaultModuleRoots, loadCatalog } from "../src/catalog.js";
import Ajv2020 from "ajv/dist/2020.js";

import {
  installPlugin,
  listPlugins,
  validateInstalledSemantics,
} from "../src/plugins.js";
import { semanticCoreDir } from "../src/semantic/contract.js";
import {
  readModuleSemantic,
  readSemanticBlock,
} from "../src/semantic/index.js";
import { createAuthoringPack, formatAuthoringPack } from "../src/write.js";

const FIXTURE = join("tests", "fixtures", "semantic-module", "module-ok");

type Json = Record<string, unknown>;

let scratch: string;
let home: string;

function moduleCopy(
  name = "spec-objects-fixture",
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

function codes(root: string): string[] {
  return readModuleSemantic(root).diagnostics.map(
    (d) => `${d.severity}:${d.code}@${d.path}`,
  );
}

beforeEach(() => {
  scratch = mkdtempSync(join(tmpdir(), "quoin-semantic-"));
  home = join(scratch, "home");
  mkdirSync(home, { recursive: true });
});

afterEach(() => {
  rmSync(scratch, { recursive: true, force: true });
});

describe("FR-070 semantic manifest block", () => {
  // Trace: FR-070-AC-1
  // Trace: TC-1336
  // Trace: NFR-017-AC-1
  // Trace: TC-1379
  it("loads every default module unchanged when no semantic block is present", () => {
    const roots = defaultModuleRoots().filter((root) => root.length > 0);
    const catalog = loadCatalog([
      ...roots,
      moduleCopy("no-block", (m) => delete m.semantic),
    ]);
    expect(catalog.modules.length).toBeGreaterThan(0);
    for (const module of catalog.modules) {
      expect(module.semanticDiagnostics ?? [], module.name).toEqual([]);
    }
    const fixture = catalog.modules.find((m) => m.name === "no-block");
    expect(fixture?.semantic).toBeUndefined();
  });

  // Trace: FR-070-AC-2
  // Trace: TC-1337
  it("reads a minimal block and reports it in the authoring pack", () => {
    const root = moduleCopy("minimal", (m) => {
      m.semantic = {
        contract_version: "1.0.0",
        semantic_core: "0.1.0",
        package: "agent-ix/spec-objects-fixture",
      };
    });
    const result = readModuleSemantic(root);
    expect(result.diagnostics.filter((d) => d.severity === "error")).toEqual(
      [],
    );
    expect(result.module?.block).toMatchObject({
      contract_version: "1.0.0",
      package: "agent-ix/spec-objects-fixture",
      semantic_core: "0.1.0",
      compatibility_posture: "additive",
      legacy_forms: "warning",
    });
    const catalog = loadCatalog([root]);
    const pack = formatAuthoringPack(
      createAuthoringPack(catalog, scratch, ["entity"]),
    );
    expect(pack).toContain(
      "semantic: agent-ix/spec-objects-fixture (semantic-core 0.1.0)",
    );
    expect(pack).toContain("data_schema: schemas/Entity.json");
  });

  // Trace: FR-070-AC-3
  // Trace: TC-1338
  it("rejects an unknown key naming it and accepts every admitted key", () => {
    const bad = moduleCopy("unknown-key", (m) => {
      (m.semantic as Json).foo = 1;
    });
    expect(codes(bad)).toContain("error:semantic.unknown-key@semantic.foo");
    const good = moduleCopy("all-keys", (m) => {
      m.semantic = {
        contract_version: "1.0.0",
        semantic_core: "0.1.0",
        package: "agent-ix/spec-objects-fixture",
        exports: ["entity"],
        imports: { "agent-ix/spec-artifacts-iso": "0.4.0" },
        targets: ["json-schema", "markdown"],
        mappings: ["typed-table"],
        compatibility_posture: "strict",
        legacy_forms: "warning",
        sweep_report: "semantic/sweep.json",
      };
    });
    expect(codes(good).filter((c) => c.startsWith("error:"))).toEqual([]);
  });

  // Trace: FR-070-AC-4
  // Trace: TC-1339
  it("rejects an export that no object type declares", () => {
    const root = moduleCopy("bad-export", (m) => {
      (m.semantic as Json).exports = ["endpoint"];
    });
    expect(codes(root)).toContain(
      "error:semantic.unknown-export@semantic.exports.endpoint",
    );
  });

  // Trace: FR-070-AC-5
  // Trace: TC-1340
  it("rejects an unsupported contract version before reading any other key", () => {
    const root = moduleCopy("bad-version", (m) => {
      (m.semantic as Json).contract_version = "2.0.0";
      (m.semantic as Json).foo = 1;
    });
    const result = codes(root);
    expect(result).toEqual([
      "error:semantic.unsupported-contract-version@semantic.contract_version",
    ]);
  });

  // Trace: FR-070-AC-6
  // Trace: TC-1341
  it("fails to install a second module declaring the same semantic package, naming both", () => {
    const first = moduleCopy("alpha-module");
    const second = moduleCopy("beta-module");
    installPlugin(`path:${first}`, home);
    expect(() => installPlugin(`path:${second}`, home)).toThrow(
      /semantic\.duplicate-package/,
    );
    const error = (() => {
      try {
        installPlugin(`path:${second}`, home);
      } catch (e) {
        return String(e);
      }
      return "";
    })();
    expect(error).toContain("alpha-module");
    expect(error).toContain("beta-module");
    expect(listPlugins(home).map((p) => p.name)).toEqual(["alpha-module"]);
  });

  // Trace: FR-070-AC-7
  // Trace: TC-1383
  it("rejects a target outside the registry and a package that is not org/repo", () => {
    const target = moduleCopy("bad-target", (m) => {
      (m.semantic as Json).targets = ["go"];
    });
    expect(
      codes(target).some((c) =>
        c.startsWith("error:semantic.unknown-target@semantic.targets"),
      ),
    ).toBe(true);
    expect(
      readModuleSemantic(target).diagnostics.find(
        (d) => d.code === "semantic.unknown-target",
      )?.message,
    ).toContain("go");
    for (const value of ["ix://agent-ix/x", "https://example.org/pkg"]) {
      const root = moduleCopy("bad-package", (m) => {
        (m.semantic as Json).package = value;
      });
      expect(codes(root)).toContain(
        "error:semantic.invalid-package@semantic.package",
      );
    }
  });

  // Trace: FR-070-AC-3
  it("restores the previously installed version when a re-install is rejected", () => {
    const good = moduleCopy("stable");
    installPlugin(`path:${good}`, home);
    const bad = join(scratch, "stable-bad");
    cpSync(good, bad, { recursive: true });
    const manifestPath = join(bad, "manifest.yaml");
    const manifest = parseYaml(readFileSync(manifestPath, "utf8")) as Json;
    (manifest.semantic as Json).foo = 1;
    writeFileSync(manifestPath, stringifyYaml(manifest));
    expect(() => installPlugin(`path:${bad}`, home)).toThrow(
      /semantic\.unknown-key/,
    );
    expect(listPlugins(home).map((p) => p.name)).toEqual(["stable"]);
    const restored = readFileSync(
      join(home, "filament", "modules", "stable", "manifest.yaml"),
      "utf8",
    );
    expect(restored).not.toContain("foo");
    expect(
      readModuleSemantic(
        join(home, "filament", "modules", "stable"),
      ).diagnostics.filter((d) => d.severity === "error"),
    ).toEqual([]);
  });

  it("readSemanticBlock returns no diagnostics and no module for a manifest without the block", () => {
    expect(readSemanticBlock({ name: "m", version: "0.1.0" }, scratch)).toEqual(
      { diagnostics: [] },
    );
  });
});

describe("FR-073 data_schema by path and digest", () => {
  function entity(root: string): Json {
    return (
      parseYaml(readFileSync(join(root, "manifest.yaml"), "utf8")) as Json
    ).object_types as Json;
  }

  // Trace: FR-073-AC-1
  // Trace: TC-1360
  it("installs a reference-form data_schema and resolves it against the vendored semantic-core bundle", () => {
    const root = moduleCopy("ref-ok");
    const installed = installPlugin(`path:${root}`, home);
    expect(installed.name).toBe("ref-ok");
    const result = readModuleSemantic(
      join(home, "filament", "modules", "ref-ok"),
    );
    expect(result.diagnostics.filter((d) => d.severity === "error")).toEqual(
      [],
    );
    const resolved = result.module?.dataSchemas.entity;
    expect(resolved?.kind).toBe("reference");
    expect((resolved?.schema as Json).$id).toContain("/Entity.json");
    expect(entity(root)).toBeDefined();
    // The FR-006 golden declaration set validates against the resolved schema
    // through the vendored semantic-core bundle (FieldDecl, ClauseRef).
    const ajv = new Ajv2020({ allErrors: true, strict: true });
    for (const name of readdirSync(semanticCoreDir()).filter(
      (n) => n.endsWith(".json") && n !== "toolchain.json",
    ))
      ajv.addSchema(
        JSON.parse(readFileSync(join(semanticCoreDir(), name), "utf8")),
      );
    const validate = ajv.compile(resolved!.schema as Json);
    const golden = JSON.parse(
      readFileSync(
        join(
          "tests",
          "fixtures",
          "semantic-module",
          "mapping",
          "config-version.expected.json",
        ),
        "utf8",
      ),
    ) as Json;
    expect(
      validate({ fields: golden.fields, clauses: golden.clauses }),
      JSON.stringify(validate.errors),
    ).toBe(true);
    expect(validate({ fields: [{ name: "x" }] })).toBe(false);
  });

  // Trace: FR-070-AC-4
  // Trace: TC-1339
  it("rejects an export whose data_schema is not a { schema, digest } reference", () => {
    const root = moduleCopy("export-inline", (m) => {
      (m.semantic as Json).exports = ["entity", "enumeration"];
    });
    expect(codes(root)).toContain(
      "error:semantic.export-without-schema@semantic.exports.enumeration",
    );
    expect(
      readModuleSemantic(root).diagnostics.find(
        (d) => d.code === "semantic.export-without-schema",
      )?.message,
    ).toContain("enumeration");
  });

  // Trace: FR-073-AC-3
  // Trace: TC-1362
  it("treats a $ref to the schema's own $id as a fragment, not a cycle", () => {
    const root = moduleCopy("self-ref", (m, moduleRoot) => {
      const file = join(moduleRoot, "schemas", "Entity.json");
      const schema = JSON.parse(readFileSync(file, "utf8")) as Json;
      schema.$defs = { marker: { type: "string" } };
      (schema.properties as Json).marker = {
        $ref: `${String(schema.$id)}#/$defs/marker`,
      };
      writeFileSync(file, JSON.stringify(schema));
      ((m.object_types as Json[])[0].data_schema as Json).digest =
        digestOf(file);
    });
    expect(codes(root).filter((c) => c.startsWith("error:"))).toEqual([]);
  });

  // Trace: FR-070-AC-1
  // Trace: TC-1379
  it("re-validates installed modules on the reconcile path", () => {
    const root = moduleCopy("reconciled");
    installPlugin(`path:${root}`, home);
    expect(() => validateInstalledSemantics(home)).not.toThrow();
    const installedManifest = join(
      home,
      "filament",
      "modules",
      "reconciled",
      "manifest.yaml",
    );
    const manifest = parseYaml(readFileSync(installedManifest, "utf8")) as Json;
    (manifest.semantic as Json).targets = ["go"];
    writeFileSync(installedManifest, stringifyYaml(manifest));
    expect(() => validateInstalledSemantics(home)).toThrow(
      /installed module reconciled violates the semantic contract[\s\S]*semantic\.unknown-target/,
    );
    const source = readFileSync(join("src", "modules.ts"), "utf8");
    expect(source).toContain("validateInstalledSemantics(home)");
  });

  // Trace: FR-073-AC-2
  // Trace: TC-1361
  it("fails on digest mismatch, missing, non-JSON, and $id-less schema files naming the path and reason", () => {
    const mismatch = moduleCopy("mismatch", (_m, root) => {
      writeFileSync(
        join(root, "schemas", "Entity.json"),
        readFileSync(join(root, "schemas", "Entity.json"), "utf8") + "\n",
      );
    });
    expect(codes(mismatch)).toContain(
      "error:semantic.data-schema-digest-mismatch@object_types[entity].data_schema.digest",
    );
    expect(
      readModuleSemantic(mismatch).diagnostics.find(
        (d) => d.code === "semantic.data-schema-digest-mismatch",
      )?.message,
    ).toMatch(/schemas\/Entity\.json.*sha256:/s);
    const missing = moduleCopy("missing", (_m, root) =>
      rmSync(join(root, "schemas", "Entity.json")),
    );
    expect(codes(missing)).toContain(
      "error:semantic.data-schema-missing@object_types[entity].data_schema.schema",
    );
    const notJson = moduleCopy("not-json", (m, root) => {
      writeFileSync(join(root, "schemas", "Entity.json"), "{ nope");
      ((m.object_types as Json[])[0].data_schema as Json).digest = digestOf(
        join(root, "schemas", "Entity.json"),
      );
    });
    expect(codes(notJson)).toContain(
      "error:semantic.data-schema-not-json@object_types[entity].data_schema.schema",
    );
    const noId = moduleCopy("no-id", (m, root) => {
      writeFileSync(
        join(root, "schemas", "Entity.json"),
        JSON.stringify({ type: "object" }),
      );
      ((m.object_types as Json[])[0].data_schema as Json).digest = digestOf(
        join(root, "schemas", "Entity.json"),
      );
    });
    expect(codes(noId)).toContain(
      "error:semantic.data-schema-not-schema@object_types[entity].data_schema.schema",
    );
  });

  // Trace: FR-073-AC-3
  // Trace: TC-1362
  it("fails a $ref to another semantic-core version, an unshipped $ref, and a $ref cycle naming the $ref", () => {
    const rewrite = (
      name: string,
      edit: (schema: Json, root: string) => void,
    ) =>
      moduleCopy(name, (m, root) => {
        const file = join(root, "schemas", "Entity.json");
        const schema = JSON.parse(readFileSync(file, "utf8")) as Json;
        edit(schema, root);
        writeFileSync(file, JSON.stringify(schema));
        ((m.object_types as Json[])[0].data_schema as Json).digest =
          digestOf(file);
      });
    const version = rewrite("core-version", (schema) => {
      ((schema.properties as Json).fields as Json).items = {
        $ref: "https://schemas.agent-ix.org/semantic-core/0.2.0/FieldDecl.json",
      };
    });
    expect(codes(version)).toContain(
      "error:semantic.schema-ref-version@object_types[entity].data_schema.schema",
    );
    const unshipped = rewrite("unshipped", (schema) => {
      ((schema.properties as Json).fields as Json).items = {
        $ref: "https://schemas.agent-ix.org/agent-ix/spec-objects-fixture/0.1.0/Missing.json",
      };
    });
    expect(codes(unshipped)).toContain(
      "error:semantic.schema-ref-unshipped@object_types[entity].data_schema.schema",
    );
    const cycle = rewrite("cycle", (schema, root) => {
      ((schema.properties as Json).fields as Json).items = {
        $ref: "https://schemas.agent-ix.org/agent-ix/spec-objects-fixture/0.1.0/Other.json",
      };
      writeFileSync(
        join(root, "schemas", "Other.json"),
        JSON.stringify({
          $id: "https://schemas.agent-ix.org/agent-ix/spec-objects-fixture/0.1.0/Other.json",
          $ref: "https://schemas.agent-ix.org/agent-ix/spec-objects-fixture/0.1.0/Entity.json",
        }),
      );
    });
    expect(codes(cycle)).toContain(
      "error:semantic.schema-ref-cycle@object_types[entity].data_schema.schema",
    );
  });

  // Trace: FR-073-AC-4
  // Trace: TC-1363
  it("warns on an inline data_schema under a semantic block and stays silent without one", () => {
    const withBlock = moduleCopy("inline-warn");
    const warnings = codes(withBlock).filter((c) => c.startsWith("warning:"));
    expect(warnings).toEqual([
      "warning:semantic.inline-data-schema@object_types[enumeration].data_schema",
    ]);
    const without = moduleCopy("inline-silent", (m) => delete m.semantic);
    expect(codes(without)).toEqual([]);
  });

  // Trace: FR-073-AC-5
  // Trace: TC-1364
  it("rejects .. and symlink escapes and an ambiguous mixed data_schema", () => {
    const dotdot = moduleCopy("dotdot", (m) => {
      ((m.object_types as Json[])[0].data_schema as Json).schema =
        "../Entity.json";
    });
    expect(codes(dotdot)).toContain(
      "error:semantic.data-schema-escape@object_types[entity].data_schema.schema",
    );
    const outside = join(scratch, "outside.json");
    writeFileSync(
      outside,
      readFileSync(join(FIXTURE, "schemas", "Entity.json")),
    );
    const link = moduleCopy("symlink", (m, root) => {
      symlinkSync(outside, join(root, "schemas", "Linked.json"));
      ((m.object_types as Json[])[0].data_schema as Json).schema =
        "schemas/Linked.json";
    });
    expect(codes(link)).toContain(
      "error:semantic.data-schema-escape@object_types[entity].data_schema.schema",
    );
    const ambiguous = moduleCopy("ambiguous", (m) => {
      ((m.object_types as Json[])[0].data_schema as Json).type = "object";
    });
    expect(codes(ambiguous)).toContain(
      "error:semantic.data-schema-ambiguous@object_types[entity].data_schema",
    );
  });

  // Trace: FR-073-CON-1
  // Trace: TC-1365
  it("resolves references with no network read", () => {
    // No fetch/http import exists on the resolution path; assert the module graph statically.
    const source = readFileSync(
      join("src", "semantic", "data-schema.ts"),
      "utf8",
    );
    expect(source).not.toMatch(/node:http|node:https|fetch\(|undici/);
    const root = moduleCopy("offline");
    expect(codes(root).filter((c) => c.startsWith("error:"))).toEqual([]);
  });

  // Trace: FR-073-CON-2
  // Trace: TC-1366
  it("keeps the inline form valid and silent for a module without a semantic block", () => {
    const root = moduleCopy("legacy-inline", (m) => {
      delete m.semantic;
      (m.object_types as Json[])[0].data_schema = { type: "object" };
    });
    expect(codes(root)).toEqual([]);
    const catalog = loadCatalog([root]);
    expect(catalog.modules[0]?.semantic).toBeUndefined();
    expect(
      catalog.entries.find((e) => e.name === "entity")?.dataSchema,
    ).toEqual({ type: "object" });
  });
});

function digestOf(file: string): string {
  return `sha256:${createHash("sha256").update(readFileSync(file)).digest("hex")}`;
}
