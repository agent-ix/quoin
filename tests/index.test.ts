import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { stringify as stringifyYaml } from "yaml";
import type { MarketplaceManifest } from "@agent-ix/ts-plugin-kit";

import {
  createAuthoringPack,
  defaultModuleRoots,
  defaultModulesManifest,
  ensureDefaultModules,
  filamentModulesDir,
  findCatalogEntry,
  installPlugin,
  listPlugins,
  loadCatalog,
  main,
  parseSourceArg,
  removePlugin,
} from "../src";

function tmp(prefix: string): string {
  return mkdtempSync(join(tmpdir(), `quoin-${prefix}-`));
}

// Build a fixture module dir (manifest.yaml + skeletons + schemas) on disk.
function makeModule(
  root: string,
  name: string,
  body: Record<string, unknown>,
  files: Record<string, string> = {},
): string {
  const dir = join(root, name);
  mkdirSync(join(dir, "skeletons"), { recursive: true });
  mkdirSync(join(dir, "schemas"), { recursive: true });
  writeFileSync(
    join(dir, "manifest.yaml"),
    stringifyYaml({ name, version: "0.1.0", ...body }),
  );
  for (const [file, content] of Object.entries(files)) {
    writeFileSync(join(dir, file), content);
  }
  return dir;
}

function isoModule(root: string): string {
  return makeModule(
    root,
    "spec-artifacts-iso",
    {
      artifact_types: [
        {
          name: "FR",
          frontmatter_schema_ref: "schemas/fr-frontmatter.schema.json",
        },
      ],
    },
    { "schemas/fr-frontmatter.schema.json": "{}", "skeletons/fr.md": "# FR\n" },
  );
}

function businessModule(root: string): string {
  return makeModule(
    root,
    "spec-objects-business",
    { object_types: [{ name: "domain" }] },
    { "skeletons/domain.md": "# domain\n" },
  );
}

function defaultSet(root: string): MarketplaceManifest {
  return {
    schemaVersion: 1,
    entries: [
      {
        name: "spec-artifacts-iso",
        source: { type: "path", path: isoModule(root) },
      },
      {
        name: "spec-objects-business",
        source: { type: "path", path: businessModule(root) },
      },
    ],
  };
}

// Trace: FR-024-AC-1
test("exports the quoin CLI entrypoint", () => {
  expect(typeof main).toBe("function");
});

// Trace: FR-017-AC-1
test("lazily installs the default module set, then loads its artifacts and objects", () => {
  const home = tmp("home");
  ensureDefaultModules(home, defaultSet(tmp("src")));
  expect(
    existsSync(
      join(filamentModulesDir(home), "spec-artifacts-iso", "manifest.yaml"),
    ),
  ).toBe(true);

  const catalog = loadCatalog(defaultModuleRoots(home));
  expect(findCatalogEntry(catalog, "FR")?.kind).toBe("artifact");
  expect(findCatalogEntry(catalog, "fr")?.kind).toBe("artifact");
  expect(findCatalogEntry(catalog, "domain")?.kind).toBe("object");
  expect(catalog.modules.map((module) => module.name)).toContain(
    "spec-artifacts-iso",
  );
  expect(catalog.modules.map((module) => module.name)).toContain(
    "spec-objects-business",
  );
});

// Trace: FR-009-AC-4
// Trace: FR-010-AC-1
test("creates authoring packs for case-insensitive artifact and object types", () => {
  const home = tmp("write-home");
  const cwd = tmp("write-cwd");
  ensureDefaultModules(home, defaultSet(tmp("write-src")));
  const catalog = loadCatalog(defaultModuleRoots(home));
  const pack = createAuthoringPack(catalog, cwd, ["fr", "DOMAIN"]);
  expect(pack.repoRoot).toBe(cwd);
  expect(pack.types.map((type) => type.name)).toEqual(["FR", "domain"]);
  expect(pack.types[0]?.schemaPath).toContain("fr-frontmatter.schema.json");
  expect(pack.types[0]?.skeletonPath).toContain("skeletons/fr.md");
  expect(pack.validation.command).toContain("quire validate --scope");
});

// Trace: FR-024-AC-2
// Trace: FR-024-AC-3
test("installs, lists, and removes a plugin from a local path source", () => {
  const home = tmp("plugin-home");
  const mod = businessModule(tmp("plugin-src"));
  const rec = installPlugin(`path:${mod}`, home);
  expect(rec.name).toBe("spec-objects-business");
  expect(listPlugins(home).map((p) => p.name)).toContain(
    "spec-objects-business",
  );
  expect(
    existsSync(
      join(filamentModulesDir(home), "spec-objects-business", "manifest.yaml"),
    ),
  ).toBe(true);

  removePlugin("spec-objects-business", home);
  expect(listPlugins(home)).toHaveLength(0);
  expect(
    existsSync(join(filamentModulesDir(home), "spec-objects-business")),
  ).toBe(false);
});

test("parseSourceArg maps CLI prefixes to typed sources", () => {
  expect(parseSourceArg("path:/a/b")).toEqual({ type: "path", path: "/a/b" });
  expect(parseSourceArg("github:agent-ix/x@v1")).toEqual({
    type: "github",
    repo: "agent-ix/x",
    ref: "v1",
  });
  expect(parseSourceArg("github:agent-ix/x")).toEqual({
    type: "github",
    repo: "agent-ix/x",
  });
  expect(parseSourceArg("package:foo@1.2.3")).toEqual({
    type: "npm",
    package: "foo",
    version: "1.2.3",
  });
  expect(parseSourceArg("package:foo")).toEqual({
    type: "npm",
    package: "foo",
  });
  expect(parseSourceArg("./bare")).toEqual({ type: "path", path: "./bare" });
});

// Trace: FR-016-AC-1
// Trace: FR-016-AC-2
test("ships the committed default module set", () => {
  const manifest = defaultModulesManifest();
  // Nine since spec-objects-safety (agent-ix/spec-objects-security#7). The
  // count is asserted rather than a lower bound so a module arriving in the
  // default set costs a deliberate line here — the set is installed into every
  // consumer's ~/.ix/filament/modules, so a silent addition is a silent change
  // to everyone's catalog.
  expect(manifest.entries).toHaveLength(9);
  expect(manifest.entries.map((e) => e.name)).toContain(
    "spec-objects-business",
  );
  expect(manifest.entries.map((e) => e.name)).toContain("spec-objects-safety");
});

test("ships claude plugin skills without artifact-specific write skills", () => {
  const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  // The Claude plugin manifest moved to the canonical `.claude-plugin/`
  // location in 2bd928f (alongside marketplace.json); the assertion had
  // lagged at the old repo-root path.
  expect(existsSync(join(repoRoot, ".claude-plugin", "plugin.json"))).toBe(
    true,
  );
  expect(existsSync(join(repoRoot, "skills", "specify", "SKILL.md"))).toBe(
    true,
  );
  expect(existsSync(join(repoRoot, "skills", "spec-review", "SKILL.md"))).toBe(
    true,
  );
  expect(
    existsSync(
      join(repoRoot, "skills", "spec-dependency-analysis", "SKILL.md"),
    ),
  ).toBe(true);
  expect(existsSync(join(repoRoot, "skills", "spec-write-fr"))).toBe(false);
});

test("ships a Codex plugin manifest with its marketplace metadata", () => {
  // .codex-plugin/ is listed in package.json `files`, so it ships to npm and
  // renders in the Codex marketplace. Nothing else parses it in this repo, so
  // a malformed or gutted manifest would otherwise reach users unnoticed.
  const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const manifestPath = join(repoRoot, ".codex-plugin", "plugin.json");
  expect(existsSync(manifestPath)).toBe(true);

  const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as Record<
    string,
    unknown
  >;
  expect(manifest.name).toBe("quoin");
  expect(manifest.skills).toBe("./skills/");
  expect(manifest.license).toBe("MIT");
  for (const key of ["author", "homepage", "repository", "keywords"]) {
    expect(manifest[key]).toBeDefined();
  }

  const ui = manifest.interface as Record<string, unknown>;
  expect(ui.displayName).toBe("Quoin");
  expect(Array.isArray(ui.capabilities)).toBe(true);
  expect(Array.isArray(ui.defaultPrompt)).toBe(true);

  // A packaging run once wrote a "+codex.<timestamp>" build suffix in here;
  // the committed value must stay a plain version.
  expect(manifest.version).toMatch(/^\d+\.\d+\.\d+$/);
});
