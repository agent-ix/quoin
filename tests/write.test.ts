import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { stringify as stringifyYaml } from "yaml";

import { loadCatalog, type SpecCatalog } from "../src/catalog";
import {
  createAuthoringPack,
  formatAuthoringPack,
  parseTypeList,
  type AuthoringPack,
} from "../src/write";

function tmp(prefix: string): string {
  return mkdtempSync(join(tmpdir(), `quoin-${prefix}-`));
}

// Build a module that declares one artifact with schema+skeleton, one object
// with only a skeleton, and one artifact with neither (manifest only).
function buildCatalog(): { catalog: SpecCatalog; root: string } {
  const root = tmp("write-src");
  const dir = join(root, "spec-module");
  mkdirSync(join(dir, "skeletons"), { recursive: true });
  mkdirSync(join(dir, "schemas"), { recursive: true });
  writeFileSync(
    join(dir, "manifest.yaml"),
    stringifyYaml({
      name: "spec-module",
      version: "0.1.0",
      artifact_types: [
        { name: "FR", frontmatter_schema_ref: "schemas/fr.schema.json" },
        { name: "BARE" },
      ],
      object_types: [{ name: "domain" }],
    }),
  );
  writeFileSync(join(dir, "schemas", "fr.schema.json"), "{}");
  writeFileSync(join(dir, "skeletons", "fr.md"), "# FR\n");
  writeFileSync(join(dir, "skeletons", "domain.md"), "# domain\n");
  const catalog = loadCatalog([dir]);
  return { catalog, root };
}

describe("parseTypeList", () => {
  test("undefined yields an empty list", () => {
    expect(parseTypeList(undefined)).toEqual([]);
  });

  test("a single string is split on commas and trimmed", () => {
    expect(parseTypeList("FR, domain ,entity")).toEqual([
      "FR",
      "domain",
      "entity",
    ]);
  });

  test("an array is flattened, comma-split, trimmed, and empties filtered", () => {
    expect(parseTypeList(["FR, NFR", " domain ", "", " , "])).toEqual([
      "FR",
      "NFR",
      "domain",
    ]);
  });
});

describe("createAuthoringPack", () => {
  // Trace: FR-013-AC-1
  test("throws when no type names are given", () => {
    const { catalog } = buildCatalog();
    expect(() => createAuthoringPack(catalog, tmp("repo"), [])).toThrow(
      "write requires --types",
    );
  });

  test("throws when repo_dir is not a directory", () => {
    const { catalog } = buildCatalog();
    const missing = join(tmp("repo"), "does-not-exist");
    expect(() => createAuthoringPack(catalog, missing, ["FR"])).toThrow(
      "write repo_dir is not a directory",
    );
  });

  test("throws when repo_dir is a file (exists but not a directory)", () => {
    const { catalog } = buildCatalog();
    const repoParent = tmp("repo-file");
    const filePath = join(repoParent, "afile.txt");
    writeFileSync(filePath, "hi");
    expect(() => createAuthoringPack(catalog, filePath, ["FR"])).toThrow(
      "write repo_dir is not a directory",
    );
  });

  test("throws with available type list when a type is not found", () => {
    const { catalog } = buildCatalog();
    expect(() => createAuthoringPack(catalog, tmp("repo"), ["nope"])).toThrow(
      /catalog type not found: nope[\s\S]*Available types: BARE, domain, FR/,
    );
  });

  // Trace: FR-009-AC-4, FR-015-AC-1
  test("builds a pack with contracts and a non-quoted clean repo path", () => {
    const { catalog } = buildCatalog();
    const repo = tmp("repo");
    const pack = createAuthoringPack(catalog, repo, ["fr", "domain"]);
    expect(pack.repoRoot).toBe(repo);
    expect(pack.types.map((t) => t.name)).toEqual(["FR", "domain"]);
    expect(pack.types[0]?.schemaPath).toContain("fr.schema.json");
    expect(pack.types[0]?.skeletonPath).toContain("skeletons/fr.md");
    expect(pack.validation.globs).toEqual(["spec/**/*.md"]);
    // Clean path: no surrounding quotes in the command.
    expect(pack.validation.command).toContain(
      `quire validate --scope ${repo} "spec/**/*.md"`,
    );
    expect(pack.validation.command).not.toContain(`'${repo}'`);
  });

  test("quotes a repo path containing a space (shellQuote quoting branch)", () => {
    const { catalog } = buildCatalog();
    const parent = mkdtempSync(join(tmpdir(), "quoin repo dir "));
    const pack = createAuthoringPack(catalog, parent, ["FR"]);
    expect(pack.validation.scope).toBe(parent);
    expect(pack.validation.command).toContain(`'${parent}'`);
    expect(pack.validation.command).not.toContain(`--scope ${parent} `);
  });
});

describe("formatAuthoringPack", () => {
  // Trace: FR-009-AC-4, FR-014-AC-1, FR-014-AC-2, FR-014-AC-3
  test("renders skeleton+schema, manifest-only, and object contracts", () => {
    const { catalog } = buildCatalog();
    const repo = tmp("repo");
    const pack = createAuthoringPack(catalog, repo, ["FR", "BARE", "domain"]);
    const text = formatAuthoringPack(pack);
    expect(text).toContain("quoin write");
    expect(text).toContain(`Repo: ${repo}`);
    // FR has both skeleton and schema lines.
    expect(text).toContain("- FR (artifact)");
    expect(text).toMatch(/skeleton: .*skeletons\/fr\.md/);
    expect(text).toMatch(/schema: .*fr\.schema\.json/);
    // BARE has neither -> manifest only.
    expect(text).toContain("- BARE (artifact)");
    expect(text).toContain("contract: manifest only");
    // domain (object) has a skeleton, no schema.
    expect(text).toContain("- domain (object)");
    expect(text).toContain(`Validate scope: ${pack.validation.scope}`);
    expect(text).toContain(`Validate command: ${pack.validation.command}`);
  });

  test("does not emit a manifest-only line when a contract has artifacts", () => {
    const { catalog } = buildCatalog();
    const pack: AuthoringPack = createAuthoringPack(catalog, tmp("repo"), [
      "FR",
    ]);
    const text = formatAuthoringPack(pack);
    expect(text).not.toContain("contract: manifest only");
  });
});

// FR-025-AC-6: the pack carries the org and its source, so an agent reads a
// concrete value instead of copying one out of a skeleton.
describe("authoring pack organization", () => {
  const priorOrg = process.env.QUOIN_ORG;

  beforeEach(() => {
    delete process.env.QUOIN_ORG;
  });

  afterEach(() => {
    if (priorOrg === undefined) delete process.env.QUOIN_ORG;
    else process.env.QUOIN_ORG = priorOrg;
  });

  // Trace: FR-025-AC-6
  test("carries an explicit --org value and its source", () => {
    const { catalog } = buildCatalog();
    const pack = createAuthoringPack(catalog, tmp("repo"), ["FR"], {
      org: "acme",
    });
    expect(pack.org).toBe("acme");
    expect(pack.orgSource).toBe("flag");
    expect(formatAuthoringPack(pack)).toContain("Org: acme (from --org)");
  });

  // Trace: FR-025-AC-6
  test("carries QUOIN_ORG when no flag is given", () => {
    process.env.QUOIN_ORG = "from-env";
    const { catalog } = buildCatalog();
    const pack = createAuthoringPack(catalog, tmp("repo"), ["FR"]);
    expect(pack.org).toBe("from-env");
    expect(pack.orgSource).toBe("env");
    expect(formatAuthoringPack(pack)).toContain(
      "Org: from-env (from QUOIN_ORG)",
    );
  });

  // Trace: FR-025-AC-5
  test("reports unresolved with the remedy and no substituted value", () => {
    const { catalog } = buildCatalog();
    // A bare temp dir: no .git/config to read an origin remote from.
    const pack = createAuthoringPack(catalog, tmp("repo"), ["FR"]);
    expect(pack.org).toBeUndefined();
    expect(pack.orgSource).toBe("none");
    const text = formatAuthoringPack(pack);
    expect(text).toContain("Org: unresolved");
    expect(text).toContain("--org");
    expect(text).not.toContain("agent-ix");
  });

  // Trace: FR-014-AC-4, FR-025-AC-6
  test("serializes the org into the --json rendering", () => {
    const { catalog } = buildCatalog();
    const pack = createAuthoringPack(catalog, tmp("repo"), ["FR"], {
      org: "acme",
    });
    expect(JSON.parse(JSON.stringify(pack))).toMatchObject({
      org: "acme",
      orgSource: "flag",
    });
  });
});
