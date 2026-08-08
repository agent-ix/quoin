import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { stringify as stringifyYaml } from "yaml";

import {
  defaultModuleRoots,
  filamentModulesDir,
  findCatalogEntry,
  ixHome,
  loadCatalog,
} from "../src/catalog";

function tmp(prefix: string): string {
  return mkdtempSync(join(tmpdir(), `quoin-${prefix}-`));
}

function writeModule(
  dir: string,
  body: Record<string, unknown>,
  files: Record<string, string> = {},
): string {
  mkdirSync(join(dir, "skeletons"), { recursive: true });
  writeFileSync(join(dir, "manifest.yaml"), stringifyYaml(body));
  for (const [file, content] of Object.entries(files)) {
    mkdirSync(join(dir, file, ".."), { recursive: true });
    writeFileSync(join(dir, file), content);
  }
  return dir;
}

describe("ixHome", () => {
  const original = process.env.IX_HOME;
  afterEach(() => {
    if (original === undefined) delete process.env.IX_HOME;
    else process.env.IX_HOME = original;
  });

  test("uses IX_HOME when set and non-empty", () => {
    process.env.IX_HOME = "/custom/home";
    expect(ixHome()).toBe("/custom/home");
    expect(filamentModulesDir()).toBe("/custom/home/filament/modules");
  });

  test("falls back to ~/.ix when IX_HOME is unset", () => {
    delete process.env.IX_HOME;
    expect(ixHome()).toBe(join(homedir(), ".ix"));
  });

  test("falls back to ~/.ix when IX_HOME is empty", () => {
    process.env.IX_HOME = "";
    expect(ixHome()).toBe(join(homedir(), ".ix"));
  });
});

describe("defaultModuleRoots", () => {
  const originalPaths = process.env.QUOIN_MODULE_PATHS;
  afterEach(() => {
    if (originalPaths === undefined) delete process.env.QUOIN_MODULE_PATHS;
    else process.env.QUOIN_MODULE_PATHS = originalPaths;
  });

  // Trace: FR-007-AC-1, FR-023-AC-2
  test("includes QUOIN_MODULE_PATHS entries and installed module dirs", () => {
    const home = tmp("home");
    const installed = filamentModulesDir(home);
    writeModule(join(installed, "spec-objects-business"), {
      name: "spec-objects-business",
      object_types: [{ name: "domain" }],
    });
    const extraRoot = tmp("extra");
    process.env.QUOIN_MODULE_PATHS = `${extraRoot}::`; // trailing empties filtered
    const roots = defaultModuleRoots(home);
    expect(roots).toContain(extraRoot);
    expect(roots).toContain(join(installed, "spec-objects-business"));
  });

  // Trace: FR-007-AC-3
  test("omits installed dirs when none have been installed", () => {
    delete process.env.QUOIN_MODULE_PATHS;
    const home = tmp("empty-home");
    expect(defaultModuleRoots(home)).toEqual([]);
  });
});

describe("loadCatalog", () => {
  // Trace: FR-006-AC-2
  test("discovers a manifest one level deep under a candidate dir", () => {
    const parent = tmp("nested");
    writeModule(join(parent, "inner"), {
      name: "spec-objects-business",
      object_types: [{ name: "domain" }],
    });
    const catalog = loadCatalog([parent]);
    expect(catalog.modules.map((m) => m.name)).toEqual([
      "spec-objects-business",
    ]);
    expect(findCatalogEntry(catalog, "domain")?.kind).toBe("object");
  });

  // Trace: FR-006-AC-3
  test("skips a non-manifest child while scanning one level deep, then finds the manifest sibling", () => {
    const parent = tmp("nested-mixed");
    // A child with no manifest (loop must skip it) plus a sibling that has one.
    mkdirSync(join(parent, "00-empty-child"), { recursive: true });
    writeModule(join(parent, "01-real"), {
      name: "spec-objects-business",
      object_types: [{ name: "domain" }],
    });
    const catalog = loadCatalog([parent]);
    expect(catalog.modules.map((m) => m.name)).toEqual([
      "spec-objects-business",
    ]);
  });

  test("skips candidates that do not resolve to a module root", () => {
    const missing = join(tmp("none"), "missing");
    const empty = tmp("empty-dir"); // exists, no manifest, no nested manifest
    const catalog = loadCatalog([missing, empty]);
    expect(catalog.modules).toEqual([]);
  });

  test("skips a candidate that is a file, not a directory", () => {
    const dir = tmp("file-candidate");
    const file = join(dir, "afile.txt");
    writeFileSync(file, "hi");
    const catalog = loadCatalog([file]);
    expect(catalog.modules).toEqual([]);
  });

  test("aborts (strict) on a present but unparseable manifest.yaml", () => {
    // NFR-008: a missing manifest is skipped, but a corrupt one surfaces rather
    // than being silently dropped — loadCatalog throws instead of returning a
    // partial catalog.
    const dir = tmp("bad-manifest");
    writeFileSync(join(dir, "manifest.yaml"), "name: [unterminated\n");
    expect(() => loadCatalog([dir])).toThrow();
  });

  // Trace: FR-006-AC-1, FR-009-AC-2
  test("handles modules with and without a version and artifacts with/without schemaRef", () => {
    const root = tmp("versions");
    writeModule(
      join(root, "with-version"),
      {
        name: "with-version",
        version: "9.9.9",
        artifact_types: [
          { name: "FR", frontmatter_schema_ref: "schemas/fr.json" },
        ],
      },
      { "schemas/fr.json": "{}" },
    );
    writeModule(join(root, "no-version"), {
      name: "no-version",
      // no version field, artifact with no schema ref
      artifact_types: [{ name: "AC" }],
    });
    const catalog = loadCatalog([
      join(root, "with-version"),
      join(root, "no-version"),
    ]);
    const versioned = catalog.modules.find((m) => m.name === "with-version");
    const unversioned = catalog.modules.find((m) => m.name === "no-version");
    expect(versioned?.version).toBe("9.9.9");
    expect(unversioned?.version).toBeUndefined();
    expect(findCatalogEntry(catalog, "FR")?.schemaPath).toContain(
      "schemas/fr.json",
    );
    expect(findCatalogEntry(catalog, "AC")?.schemaPath).toBeUndefined();
  });

  test("deduplicates repeated module roots and module names", () => {
    const root = tmp("dedup");
    const dir = writeModule(join(root, "mod"), {
      name: "dup-name",
      object_types: [{ name: "domain" }],
    });
    // Same root listed twice -> seenRoots dedup; a second dir with the same
    // declared name -> seenModuleNames dedup.
    const otherDir = writeModule(join(root, "mod2"), {
      name: "dup-name",
      object_types: [{ name: "entity" }],
    });
    const catalog = loadCatalog([dir, dir, otherDir]);
    expect(catalog.modules).toHaveLength(1);
    expect(catalog.modules[0]?.name).toBe("dup-name");
    // entity from the duplicate-named module is not added.
    expect(findCatalogEntry(catalog, "entity")).toBeUndefined();
  });

  // Trace: FR-009-AC-1
  test("falls back to the directory basename when manifest has no name", () => {
    const root = tmp("noname");
    writeModule(join(root, "anon-module"), {
      object_types: [{ name: "domain" }],
    });
    const catalog = loadCatalog([join(root, "anon-module")]);
    expect(catalog.modules[0]?.name).toBe("anon-module");
  });

  // Trace: FR-009-AC-3
  test("ignores non-array and malformed type entries", () => {
    const root = tmp("malformed");
    writeModule(join(root, "mod"), {
      name: "mod",
      artifact_types: "not-an-array",
      object_types: [{ name: "domain" }, "string-entry", { noName: true }],
    });
    const catalog = loadCatalog([join(root, "mod")]);
    expect(catalog.modules[0]?.artifactTypes).toEqual([]);
    expect(catalog.modules[0]?.objectTypes).toEqual(["domain"]);
  });
});

describe("skeleton resolution", () => {
  // The lookup reads real directory entries, so a module that ships no
  // skeletons/ directory at all must resolve to no skeleton rather than throw.
  // Trace: FR-009-AC-5
  test("resolves no skeleton when the module ships no skeletons directory", () => {
    const root = tmp("catalog-noskel");
    const dir = join(root, "bare-module");
    mkdirSync(dir, { recursive: true });
    writeFileSync(
      join(dir, "manifest.yaml"),
      stringifyYaml({
        name: "bare-module",
        version: "0.1.0",
        artifact_types: [{ name: "FR" }],
      }),
    );
    const catalog = loadCatalog([dir]);
    const entry = findCatalogEntry(catalog, "FR");
    expect(entry?.name).toBe("FR");
    expect(entry?.skeletonPath).toBeUndefined();
  });

  // Both naming conventions ship in practice: spec-artifacts-iso uses lowercase
  // (`fr.md`) while spec-artifacts-process uses the type's own casing
  // (`Feedback.md`). Each must resolve to the real on-disk name, never a
  // fabricated one that only "exists" on a case-insensitive filesystem.
  // Trace: FR-009-AC-4, FR-009-AC-5
  test.each([
    ["the type's own casing", "Feedback", "Feedback.md"],
    ["a lowercase filename", "FR", "fr.md"],
  ])("resolves a skeleton named with %s", (_label, typeName, fileName) => {
    const root = tmp("catalog-casing");
    const dir = join(root, "cased-module");
    mkdirSync(join(dir, "skeletons"), { recursive: true });
    writeFileSync(
      join(dir, "manifest.yaml"),
      stringifyYaml({
        name: "cased-module",
        version: "0.1.0",
        artifact_types: [{ name: typeName }],
      }),
    );
    writeFileSync(join(dir, "skeletons", fileName), `# ${typeName}\n`);

    const entry = findCatalogEntry(loadCatalog([dir]), typeName);
    expect(entry?.skeletonPath).toBe(join(dir, "skeletons", fileName));
  });

  // Trace: FR-009-AC-5
  test("resolves no skeleton when only an unrelated casing is present", () => {
    // `Fr.md` matches neither the type's own casing nor its lowercase form.
    // Previously existsSync would have found it on macOS and missed it on
    // Linux; now it misses consistently on both.
    const root = tmp("catalog-oddcase");
    const dir = join(root, "odd-module");
    mkdirSync(join(dir, "skeletons"), { recursive: true });
    writeFileSync(
      join(dir, "manifest.yaml"),
      stringifyYaml({
        name: "odd-module",
        version: "0.1.0",
        artifact_types: [{ name: "FR" }],
      }),
    );
    writeFileSync(join(dir, "skeletons", "Fr.md"), "# FR\n");

    expect(
      findCatalogEntry(loadCatalog([dir]), "FR")?.skeletonPath,
    ).toBeUndefined();
  });
});

describe("findDuplicates", () => {
  test("reports a type declared by two modules", () => {
    const root = tmp("dups");
    writeModule(join(root, "a"), {
      name: "module-a",
      object_types: [{ name: "domain" }],
    });
    writeModule(join(root, "b"), {
      name: "module-b",
      object_types: [{ name: "domain" }],
    });
    const catalog = loadCatalog([join(root, "a"), join(root, "b")]);
    expect(catalog.duplicates).toEqual([
      { kind: "object", name: "domain", modules: ["module-a", "module-b"] },
    ]);
  });

  test("reports no duplicates when type names are unique", () => {
    const root = tmp("nodups");
    writeModule(join(root, "a"), {
      name: "module-a",
      object_types: [{ name: "domain" }],
    });
    writeModule(join(root, "b"), {
      name: "module-b",
      object_types: [{ name: "entity" }],
    });
    const catalog = loadCatalog([join(root, "a"), join(root, "b")]);
    expect(catalog.duplicates).toEqual([]);
  });
});
