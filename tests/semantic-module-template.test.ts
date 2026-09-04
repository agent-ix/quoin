/**
 * The semantic-module template gate (quoin FR-076..FR-083, NFR-018..NFR-020).
 *
 * A template that has never been instantiated is unverified: every defect it
 * carries is latent until the first maintainer meets it, and that maintainer has
 * no way to tell a template defect from their own. Every row here therefore
 * RENDERS the template into a temporary directory and reads what came out.
 *
 * Nothing here skips. When a tool the gate needs is absent, the row fails naming
 * the command that installs it.
 */
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";
import { parse } from "yaml";

import {
  CONFORMANCE_PATH,
  REPO_ROOT,
  RenderRefused,
  TEMPLATE_DIR,
  driftedSurfaces,
  loadConformance,
  maintainedSurfaces,
  missingSurfaces,
  privateRegistryHits,
  render,
  requireTool,
  requiredPathsFor,
  residueHits,
  walkFiles,
  withRendered,
  type ModuleKind,
} from "./support/semantic-module-template.js";

const KINDS: ModuleKind[] = ["object", "artifact", "mixed"];
const conformance = loadConformance();

/** The renderer is a hard requirement of every row below. */
function cookiecutterVersion(): string {
  return requireTool({
    command: "cookiecutter",
    args: ["--version"],
    install: "pipx install cookiecutter",
  });
}

function expectRefused(
  kind: ModuleKind,
  extra: Record<string, string>,
): string {
  let refusal: RenderRefused | undefined;
  try {
    const rendered = render({ kind, extra });
    rendered.dispose();
  } catch (error) {
    if (error instanceof RenderRefused) refusal = error;
    else throw error;
  }
  expect(
    refusal,
    "the rendering was accepted when it should have been refused",
  ).toBeDefined();
  expect(existsSync(refusal!.outputDir)).toBe(false);
  return refusal!.message;
}

describe("the renderer is present", () => {
  // TC-1464
  it("names cookiecutter, its floor and the install command when it is absent", () => {
    expect(cookiecutterVersion()).toMatch(/cookiecutter/i);
    const floors = parse(
      readFileSync(
        join(TEMPLATE_DIR, "{{cookiecutter.repo_name}}", "toolchain.yaml"),
        "utf8",
      ).replace(/\{\{ cookiecutter\.[a-z_]+ \}\}/g, "0.0.0"),
    ) as { commands: { name: string; minimum: string; install: string }[] };
    // TC-1463: every external command carries a floor and an install step.
    expect(floors.commands.length).toBeGreaterThan(0);
    for (const command of floors.commands) {
      expect(command.minimum, `${command.name} declares no floor`).toBeTruthy();
      expect(
        command.install,
        `${command.name} names no install step`,
      ).toBeTruthy();
    }
  });
});

describe("variants render from one core", () => {
  // TC-1400, TC-1401, TC-1402
  it.each(KINDS)(
    "renders the %s variant with the right manifest sections",
    (kind) => {
      cookiecutterVersion();
      withRendered({ kind }, (rendered) => {
        const manifest = readFileSync(
          join(rendered.root, rendered.packageName, "manifest.yaml"),
          "utf8",
        );
        const parsed = parse(manifest) as Record<string, unknown>;
        const objects = (parsed.object_types ?? []) as unknown[];
        const artifacts = (parsed.artifact_types ?? []) as unknown[];
        if (kind === "object") {
          expect(objects.length).toBeGreaterThan(0);
          expect(artifacts).toHaveLength(0);
        } else if (kind === "artifact") {
          expect(artifacts.length).toBeGreaterThan(0);
          expect(objects).toHaveLength(0);
        } else {
          expect(objects.length).toBeGreaterThan(0);
          expect(artifacts.length).toBeGreaterThan(0);
        }
      });
    },
  );

  // TC-1466
  it("maps each imported module to an exact version, and empty imports to {}", () => {
    cookiecutterVersion();
    withRendered({ kind: "mixed" }, (rendered) => {
      const manifest = parse(
        readFileSync(
          join(rendered.root, rendered.packageName, "manifest.yaml"),
          "utf8",
        ),
      ) as { semantic: { imports: Record<string, string> } };
      const imports = manifest.semantic.imports;
      expect(Object.keys(imports).length).toBeGreaterThan(0);
      for (const version of Object.values(imports)) {
        expect(version).toMatch(/^\d+\.\d+\.\d+/);
      }
    });
    withRendered({ kind: "object" }, (rendered) => {
      const manifest = parse(
        readFileSync(
          join(rendered.root, rendered.packageName, "manifest.yaml"),
          "utf8",
        ),
      ) as { semantic: { imports: Record<string, string> } };
      expect(manifest.semantic.imports).toEqual({});
    });
  });

  // TC-1403
  it("carries each variant-shared file exactly once in the template source", () => {
    cookiecutterVersion();
    const trees = KINDS.map((kind) =>
      withRendered({ kind }, (rendered) => ({
        kind,
        files: new Map(
          walkFiles(rendered.root).map((file) => [
            file.replace(rendered.packageName, "{package}"),
            readFileSync(join(rendered.root, file), "utf8").replace(
              new RegExp(rendered.repoName, "g"),
              "{repo}",
            ),
          ]),
        ),
      })),
    );
    const templateFiles = walkFiles(TEMPLATE_DIR);
    const shared = [...trees[0].files.keys()].filter((file) =>
      trees.every((tree) => tree.files.get(file) === trees[0].files.get(file)),
    );
    expect(shared.length).toBeGreaterThan(10);
    // Each shared surface has exactly ONE source in the template, at the same
    // relative path. A second copy would be the fleet drift this template exists
    // to end, one level up.
    for (const file of shared) {
      const sources = templateFiles.filter(
        (path) =>
          path ===
          `{{cookiecutter.repo_name}}/${file.replace("{package}", "{{cookiecutter.package_name}}")}`,
      );
      expect(
        sources.length,
        `${file} has ${sources.length} sources in the template`,
      ).toBe(1);
    }
  });

  // TC-1467
  it("renders one variant twice to byte-identical trees", () => {
    cookiecutterVersion();
    const snapshot = (kind: ModuleKind) =>
      withRendered({ kind }, (rendered) =>
        walkFiles(rendered.root).map(
          (file) =>
            `${file}\n${readFileSync(join(rendered.root, file), "utf8")}`,
        ),
      );
    expect(snapshot("mixed")).toEqual(snapshot("mixed"));
  });

  // TC-1452
  it("renders every variant unattended, from arguments alone", () => {
    cookiecutterVersion();
    for (const kind of KINDS) {
      withRendered({ kind }, (rendered) => {
        expect(existsSync(join(rendered.root, "Makefile"))).toBe(true);
      });
    }
  });
});

describe("an invalid input is refused, naming the value", () => {
  // TC-1404
  it("refuses a non-AGPL licence and renders the AGPL text by default", () => {
    cookiecutterVersion();
    expect(expectRefused("object", { license: "MIT" })).toContain("MIT");
    withRendered({ kind: "object" }, (rendered) => {
      const licence = readFileSync(join(rendered.root, "LICENSE"), "utf8");
      expect(licence).toContain("GNU AFFERO GENERAL PUBLIC LICENSE");
    });
  });

  // TC-1405
  it("refuses an unknown module kind naming it", () => {
    cookiecutterVersion();
    expect(expectRefused("object", { module_kind: "hybrid" })).toContain(
      "hybrid",
    );
  });

  // TC-1406
  it("refuses an import without an exact version and accepts one with it", () => {
    cookiecutterVersion();
    expect(
      expectRefused("mixed", {
        imported_modules: "agent-ix/spec-objects-business",
      }),
    ).toContain("agent-ix/spec-objects-business");
    withRendered(
      {
        kind: "mixed",
        extra: { imported_modules: "agent-ix/spec-objects-business@0.3.0" },
      },
      (rendered) => expect(existsSync(rendered.root)).toBe(true),
    );
  });

  // TC-1407
  it("refuses a target outside the filament-core-data registry naming it", () => {
    cookiecutterVersion();
    expect(
      expectRefused("object", { generated_targets: "json-schema,go" }),
    ).toContain("go");
  });

  // TC-1453
  it("refuses a mixed module that imports nothing", () => {
    cookiecutterVersion();
    expect(expectRefused("mixed", { imported_modules: "" })).toContain(
      "import",
    );
  });

  // TC-1459
  it("refuses two imports naming the same package identity", () => {
    cookiecutterVersion();
    const message = expectRefused("mixed", {
      imported_modules:
        "agent-ix/spec-objects-business@0.3.0,agent-ix/spec-objects-business@0.2.0",
    });
    expect(message).toContain("agent-ix/spec-objects-business");
  });

  // TC-1454
  it("leaves no directory behind when it refuses", () => {
    cookiecutterVersion();
    // `expectRefused` asserts the output directory is gone for every case above;
    // this row states it as its own obligation.
    expectRefused("object", { license: "Apache-2.0" });
  });
});

describe("the rendered tree conforms and carries no residue", () => {
  // TC-1444, TC-1447
  it.each(KINDS)("carries every surface the contract names (%s)", (kind) => {
    cookiecutterVersion();
    withRendered({ kind }, (rendered) => {
      expect(missingSurfaces(conformance, rendered)).toEqual([]);
    });
  });

  // TC-1447 negative: removing a required surface is detected.
  it("reports a missing required surface naming it", () => {
    cookiecutterVersion();
    withRendered({ kind: "object" }, (rendered) => {
      const contract = loadConformance();
      contract.all.required_paths.push("NOTICE-THAT-DOES-NOT-EXIST");
      expect(missingSurfaces(contract, rendered)).toContain(
        "NOTICE-THAT-DOES-NOT-EXIST",
      );
    });
  });

  // TC-1446, NFR-018
  it.each(KINDS)("carries no generation residue (%s)", (kind) => {
    cookiecutterVersion();
    withRendered({ kind }, (rendered) => {
      expect(residueHits(conformance, rendered)).toEqual([]);
    });
  });

  // TC-1446 negative: the scan actually fires.
  it("reports an injected residue instance naming the file", () => {
    cookiecutterVersion();
    withRendered({ kind: "object" }, (rendered) => {
      writeFileSync(
        join(rendered.root, "NOTICE.md"),
        "Maintained by your-org at /home/somebody/dev.\n",
      );
      const hits = residueHits(conformance, rendered);
      expect(hits.map((hit) => hit.klass).sort()).toEqual([
        "absolute_render_path",
        "placeholder_org",
      ]);
      expect(hits.every((hit) => hit.file === "NOTICE.md")).toBe(true);
    });
  });

  // TC-1436
  it.each(KINDS)(
    "names a private registry only where the contract allows (%s)",
    (kind) => {
      cookiecutterVersion();
      withRendered({ kind }, (rendered) => {
        expect(privateRegistryHits(conformance, rendered)).toEqual([]);
        const pkg = JSON.parse(
          readFileSync(join(rendered.root, "package.json"), "utf8"),
        ) as { publishConfig: { registry: string; access: string } };
        expect(pkg.publishConfig.registry).toBe("https://registry.npmjs.org/");
        expect(pkg.publishConfig.access).toBe("public");
      });
    },
  );

  // TC-1412
  it.each(KINDS)("ships no .npmrc at any depth (%s)", (kind) => {
    cookiecutterVersion();
    withRendered({ kind }, (rendered) => {
      expect(
        walkFiles(rendered.root).filter((f) => f.endsWith(".npmrc")),
      ).toEqual([]);
    });
  });
});

describe("the rendered repository is public-ready", () => {
  // TC-1434
  it.each(KINDS)("uses one licence identifier everywhere (%s)", (kind) => {
    cookiecutterVersion();
    withRendered({ kind }, (rendered) => {
      const pkg = JSON.parse(
        readFileSync(join(rendered.root, "package.json"), "utf8"),
      ) as { license: string };
      const pyproject = readFileSync(
        join(rendered.root, "pyproject.toml"),
        "utf8",
      );
      const readme = readFileSync(join(rendered.root, "README.md"), "utf8");
      expect(pkg.license).toBe("AGPL-3.0-or-later");
      expect(pyproject).toContain('license = "AGPL-3.0-or-later"');
      expect(readme).toContain("AGPL-3.0-or-later");
      for (const wrong of ["MIT", "Apache-2.0", "BSD-3-Clause"]) {
        expect(pkg.license).not.toBe(wrong);
      }
    });
  });

  // TC-1433
  it.each(KINDS)(
    "distributes the same payload through both surfaces (%s)",
    (kind) => {
      cookiecutterVersion();
      withRendered({ kind }, (rendered) => {
        const pkg = JSON.parse(
          readFileSync(join(rendered.root, "package.json"), "utf8"),
        ) as { files: string[] };
        const pyproject = readFileSync(
          join(rendered.root, "pyproject.toml"),
          "utf8",
        );
        for (const item of ["manifest.yaml", "schemas/", "skeletons/"]) {
          expect(pkg.files).toContain(item);
          expect(pyproject).toContain(item.replace(/\/$/, ""));
        }
        // The toolchain is a build input, not module data.
        expect(pkg.files).not.toContain("typespec/");
        expect(pyproject).toContain("exclude = [");
      });
    },
  );

  // TC-1432
  it.each(KINDS)(
    "declares no local path reference and no upper bound (%s)",
    (kind) => {
      cookiecutterVersion();
      withRendered({ kind }, (rendered) => {
        const pkg = readFileSync(join(rendered.root, "package.json"), "utf8");
        expect(pkg).not.toMatch(/"(file|link|portal):/);
        const deps = JSON.parse(pkg) as {
          devDependencies: Record<string, string>;
        };
        for (const [name, range] of Object.entries(deps.devDependencies)) {
          expect(range, `${name} carries an upper bound`).not.toMatch(/[<]/);
        }
      });
    },
  );

  // TC-1437
  it.each(KINDS)(
    "ships manually triggered, delegating workflows (%s)",
    (kind) => {
      cookiecutterVersion();
      withRendered({ kind }, (rendered) => {
        for (const name of ["ci.yml", "release-npm.yml"]) {
          const workflow = readFileSync(
            join(rendered.root, ".github", "workflows", name),
            "utf8",
          );
          expect(workflow).toContain("workflow_dispatch");
          expect(workflow).not.toMatch(/^on:\s*\n\s*(push|pull_request):/m);
          expect(workflow).toMatch(/uses: [\w-]+\//);
          expect(workflow).not.toMatch(
            /npm publish|twine upload|poetry publish/,
          );
        }
      });
    },
  );

  // TC-1435
  it.each(KINDS)("ships the ownership and guidance files (%s)", (kind) => {
    cookiecutterVersion();
    withRendered({ kind }, (rendered) => {
      for (const file of [
        ".github/CODEOWNERS",
        "AGENTS.md",
        "CLAUDE.md",
        "README.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
        ".gitignore",
        ".gitattributes",
        "Makefile",
      ]) {
        expect(
          existsSync(join(rendered.root, file)),
          `${file} is missing`,
        ).toBe(true);
      }
    });
  });

  // TC-1438
  it("names the catalog file and the tracking project in the catalog document", () => {
    cookiecutterVersion();
    withRendered({ kind: "object" }, (rendered) => {
      const doc = readFileSync(
        join(rendered.root, "docs", "catalog-entry.md"),
        "utf8",
      );
      expect(doc).toContain("default-modules.yaml");
      expect(doc).toMatch(/Project 18/);
    });
  });

  // TC-1451
  it("records a declared target with no emitter as declared and not emitted", () => {
    cookiecutterVersion();
    withRendered({ kind: "object" }, (rendered) => {
      const readme = readFileSync(join(rendered.root, "README.md"), "utf8");
      const matrix = readFileSync(
        join(rendered.root, "spec", "matrix.md"),
        "utf8",
      );
      const manifest = parse(
        readFileSync(
          join(rendered.root, rendered.packageName, "manifest.yaml"),
          "utf8",
        ),
      ) as { semantic: { targets: string[] } };
      expect(manifest.semantic.targets).toContain("markdown");
      expect(readme).toContain("Only `json-schema` is emitted today");
      expect(matrix).toMatch(/🚧[^|\n]*no emitter/);
    });
  });
});

describe("the rendered governance tree validates as rendered", () => {
  // TC-1439, TC-1443
  it.each(KINDS)("passes quire validate over spec/ (%s)", (kind) => {
    cookiecutterVersion();
    withRendered({ kind }, (rendered) => {
      const quire = requireTool({
        command: "quire",
        args: ["--version"],
        install: "npm i -g @agent-ix/quire-cli",
      });
      expect(quire).toMatch(/quire/);
      const result = execFileSync(
        "quire",
        ["validate", "--scope", rendered.root, "spec/**/*.md"],
        { encoding: "utf8", stdio: "pipe" },
      );
      expect(result).not.toMatch(/failed structural validation/);
      for (const folder of [
        "stakeholder",
        "usecase",
        "functional",
        "non-functional",
      ]) {
        expect(
          existsSync(join(rendered.root, "spec", folder, "index.md")),
        ).toBe(true);
      }
      expect(existsSync(join(rendered.root, "spec", "log.md"))).toBe(true);
    });
  });

  // TC-1440, TC-1441, TC-1442
  it.each(KINDS)(
    "carries an honest, in-vocabulary Test Matrix (%s)",
    (kind) => {
      cookiecutterVersion();
      withRendered({ kind }, (rendered) => {
        const matrix = readFileSync(
          join(rendered.root, "spec", "matrix.md"),
          "utf8",
        );
        const rows = matrix
          .split("\n")
          .filter((line) => /^\| TC-\d+ \|/.test(line))
          .map((line) => line.split("|").map((cell) => cell.trim()));
        expect(rows.length).toBeGreaterThan(10);
        const criteria = new Set(
          [...matrix.matchAll(/(?:FR|NFR|StR)-\d+-AC-\d+/g)].map((m) => m[0]),
        );
        for (const row of rows) {
          const status = row[6];
          expect(status, `${row[1]} carries status ${status}`).toMatch(
            /^(✅|❌|🚧|⛔)(\s+.*)?$/,
          );
          if (status.startsWith("🚧")) {
            expect(
              status.length,
              `${row[1]} is 🚧 with no reason`,
            ).toBeGreaterThan(3);
          }
          for (const trace of row[5].split(",").map((t) => t.trim())) {
            expect(criteria.has(trace), `${row[1]} traces to ${trace}`).toBe(
              true,
            );
          }
        }
        // TC-1470: the retired marker is not admitted anywhere.
        expect(matrix).not.toContain("⚠️ ");
        expect(/^\| TC-\d+ \|.*\| ⚠️/m.test(matrix)).toBe(false);
      });
    },
  );

  // TC-1461
  it("says the module's domain types are the maintainer's, and copies no requirement", () => {
    cookiecutterVersion();
    withRendered({ kind: "mixed" }, (rendered) => {
      const spec = readFileSync(join(rendered.root, "spec", "spec.md"), "utf8");
      expect(spec).toMatch(/Out of Scope/);
      expect(spec).toMatch(/domain vocabulary|domain types/i);
      // Nothing from the maintained repositories' own vocabulary leaked in.
      const rendered_text = walkFiles(join(rendered.root, "spec"))
        .map((file) => readFileSync(join(rendered.root, "spec", file), "utf8"))
        .join("\n");
      for (const foreign of ["aggregate_root", "ubiquitous language", "EARS"]) {
        expect(rendered_text).not.toContain(foreign);
      }
    });
  });
});

describe("the template depends on shared tooling rather than copying it", () => {
  // TC-1455
  it("carries no copy of the emitter, the runtime or the grammar", () => {
    const files = walkFiles(TEMPLATE_DIR);
    for (const file of files) {
      const text = readFileSync(join(TEMPLATE_DIR, file), "utf8");
      expect(text, `${file} appears to vendor the emitter`).not.toContain(
        "class JsonSchemaEmitter",
      );
      expect(text, `${file} appears to vendor the grammar`).not.toContain(
        "namespace AgentIx.Semantic.Core;",
      );
    }
    const pkg = JSON.parse(
      readFileSync(
        join(TEMPLATE_DIR, "{{cookiecutter.repo_name}}", "package.json"),
        "utf8",
      )
        .replace(/\{\{[^}]*\}\}/g, "x")
        .replace(/\{%-?[\s\S]*?-?%\}/g, ""),
    ) as { devDependencies: Record<string, string> };
    expect(Object.keys(pkg.devDependencies).sort()).toEqual([
      "@agent-ix/semantic-core",
      "@typespec/compiler",
      "@typespec/json-schema",
    ]);
  });

  // TC-1456
  it("renders one emit driver, byte-identical across every variant", () => {
    cookiecutterVersion();
    const bodies = KINDS.map((kind) =>
      withRendered({ kind }, (rendered) =>
        readFileSync(
          join(rendered.root, "scripts", "generate-schemas.mjs"),
          "utf8",
        )
          .replace(new RegExp(rendered.repoName, "g"), "{repo}")
          .replace(new RegExp(rendered.packageName, "g"), "{package}"),
      ),
    );
    expect(new Set(bodies).size).toBe(1);
  });
});

describe("the conformance contract tracks the maintained repositories", () => {
  // TC-1418
  it("names no surface both maintained modules carry that it neither requires nor exempts", () => {
    requireTool({
      command: "git",
      args: ["--version"],
      install: "your platform's git",
    });
    expect(driftedSurfaces(conformance)).toEqual([]);
  });

  // TC-1462
  it("fails naming the repository and the revision when one cannot be read", () => {
    expect(() =>
      maintainedSurfaces({
        ...conformance.maintained_modules[0],
        revision: "0".repeat(40),
      }),
    ).toThrow(/pinned revision 0{40}/);
  });

  // TC-1445, FR-083-CON-2
  it("is a declared file, and every exemption carries a reason", () => {
    expect(existsSync(CONFORMANCE_PATH)).toBe(true);
    expect(conformance.contract_version).toBe("1.0.0");
    for (const exemption of conformance.drift_exemptions) {
      const named = exemption.path ?? exemption.pattern;
      expect(
        named,
        "an exemption names neither a path nor a pattern",
      ).toBeTruthy();
      expect(
        exemption.reason,
        `${named} is exempt with no reason`,
      ).toBeTruthy();
    }
    for (const module of conformance.maintained_modules) {
      expect(module.revision).toMatch(/^[0-9a-f]{40}$/);
      expect(module.remote).toMatch(/^https:\/\//);
    }
    for (const kind of KINDS) {
      expect(requiredPathsFor(conformance, kind, "pkg").length).toBeGreaterThan(
        30,
      );
    }
  });

  // TC-1445: no rendered tree survives the run.
  it("writes every rendered tree under a temporary directory and removes it", () => {
    cookiecutterVersion();
    const rendered = render({ kind: "object" });
    expect(rendered.dir.startsWith(REPO_ROOT)).toBe(false);
    rendered.dispose();
    expect(existsSync(rendered.dir)).toBe(false);
    expect(
      readdirSync(REPO_ROOT).filter((entry) =>
        entry.startsWith("quoin-semantic-module-"),
      ),
    ).toEqual([]);
  });
});
