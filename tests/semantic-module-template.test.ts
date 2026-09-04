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
  it("TC-1463, TC-1464 every external command has a declared floor, and the renderer's absence names it", () => {
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
    "TC-1400, TC-1401, TC-1402 renders the %s variant with the right manifest sections",
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
  it("TC-1466 maps each imported module to an exact version, and empty imports to an empty mapping", () => {
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
  it("TC-1403 carries each variant-shared file exactly once in the template source", () => {
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
  it("TC-1467 renders one variant twice to byte-identical trees", () => {
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
  it("TC-1452 renders every variant unattended, from arguments alone", () => {
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
  it("TC-1404 refuses a non-AGPL licence and renders the AGPL text by default", () => {
    cookiecutterVersion();
    expect(expectRefused("object", { license: "MIT" })).toContain("MIT");
    withRendered({ kind: "object" }, (rendered) => {
      const licence = readFileSync(join(rendered.root, "LICENSE"), "utf8");
      expect(licence).toContain("GNU AFFERO GENERAL PUBLIC LICENSE");
    });
  });

  // TC-1405
  it("TC-1405 refuses an unknown module kind naming it", () => {
    cookiecutterVersion();
    expect(expectRefused("object", { module_kind: "hybrid" })).toContain(
      "hybrid",
    );
  });

  // TC-1406
  it("TC-1406 refuses an import without an exact version and accepts one with it", () => {
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
  it("TC-1407 refuses a target outside the filament-core-data registry naming it", () => {
    cookiecutterVersion();
    expect(
      expectRefused("object", { generated_targets: "json-schema,go" }),
    ).toContain("go");
  });

  // TC-1453
  it("TC-1453 refuses a mixed module that imports nothing", () => {
    cookiecutterVersion();
    expect(expectRefused("mixed", { imported_modules: "" })).toContain(
      "import",
    );
  });

  // TC-1459
  it("TC-1459 refuses two imports naming the same package identity", () => {
    cookiecutterVersion();
    const message = expectRefused("mixed", {
      imported_modules:
        "agent-ix/spec-objects-business@0.3.0,agent-ix/spec-objects-business@0.2.0",
    });
    expect(message).toContain("agent-ix/spec-objects-business");
  });

  // TC-1454
  it("TC-1454 leaves no directory behind when it refuses", () => {
    cookiecutterVersion();
    // `expectRefused` asserts the output directory is gone for every case above;
    // this row states it as its own obligation.
    expectRefused("object", { license: "Apache-2.0" });
  });
});

describe("the rendered tree conforms and carries no residue", () => {
  // TC-1444, TC-1447
  it.each(KINDS)(
    "TC-1444 carries every surface the contract names (%s)",
    (kind) => {
      cookiecutterVersion();
      withRendered({ kind }, (rendered) => {
        expect(missingSurfaces(conformance, rendered)).toEqual([]);
      });
    },
  );

  // TC-1447 negative: removing a required surface is detected.
  it("TC-1447 reports a missing required surface naming it", () => {
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
  it.each(KINDS)("TC-1446 carries no generation residue (%s)", (kind) => {
    cookiecutterVersion();
    withRendered({ kind }, (rendered) => {
      expect(residueHits(conformance, rendered)).toEqual([]);
    });
  });

  // TC-1446 negative: the scan actually fires.
  it("TC-1446 reports an injected residue instance naming the file", () => {
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
    "TC-1436 names a private registry only where the contract allows (%s)",
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
  it.each(KINDS)("TC-1412 ships no .npmrc at any depth (%s)", (kind) => {
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
  it.each(KINDS)(
    "TC-1434 uses one licence identifier everywhere (%s)",
    (kind) => {
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
    },
  );

  // TC-1433
  it.each(KINDS)(
    "TC-1433 distributes the same payload through both surfaces (%s)",
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
    "TC-1432 declares no local path reference and no upper bound (%s)",
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
    "TC-1437 ships manually triggered, delegating workflows (%s)",
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
  it.each(KINDS)(
    "TC-1435 ships the ownership and guidance files (%s)",
    (kind) => {
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
    },
  );

  // TC-1438
  it("TC-1438 names the catalog file and the tracking project in the catalog document", () => {
    // TC-1438
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
  it("TC-1451 records a declared target with no emitter as declared and not emitted", () => {
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
  it.each(KINDS)(
    "TC-1439, TC-1443 passes quire validate over spec/ (%s)",
    (kind) => {
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
    },
  );

  // TC-1440, TC-1441, TC-1442
  it.each(KINDS)(
    "TC-1440, TC-1441, TC-1442, TC-1470 carries an honest, in-vocabulary Test Matrix (%s)",
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
  it("TC-1461 says the module's domain types are the maintainer's, and copies no requirement", () => {
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
  it("TC-1455 carries no copy of the emitter, the runtime or the grammar", () => {
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
        // `[^}]` would leave this file with one unmatched closing brace, which
        // `quire coverage`'s source scanner reads as an unbalanced block and
        // answers by dropping the file — taking every tracking tag in it. The
        // lazy any-character form matches the same text and stays balanced.
        .replace(/\{\{[\s\S]*?\}\}/g, "x")
        .replace(/\{%-?[\s\S]*?-?%\}/g, ""),
    ) as { devDependencies: Record<string, string> };
    expect(Object.keys(pkg.devDependencies).sort()).toEqual([
      "@agent-ix/semantic-core",
      "@typespec/compiler",
      "@typespec/json-schema",
    ]);
  });

  // TC-1456
  it("TC-1456 renders one emit driver, byte-identical across every variant", () => {
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
  it("TC-1418 names no surface both maintained modules carry that it neither requires nor exempts", () => {
    requireTool({
      command: "git",
      args: ["--version"],
      install: "your platform's git",
    });
    expect(driftedSurfaces(conformance)).toEqual([]);
  });

  // TC-1462
  it("TC-1462 fails naming the repository and the revision when one cannot be read", () => {
    // The repository IS present — `driftedSurfaces` above read it — so this
    // exercises the unreachable-revision path rather than the absent-repository
    // one, and the assertion names the message that path produces.
    expect(() =>
      maintainedSurfaces({
        ...conformance.maintained_modules[0],
        revision: "0".repeat(40),
      }),
    ).toThrow(/cannot be read at its pinned revision 0{40}/);
  });

  // TC-1445, FR-083-CON-2
  it("TC-1445 is a declared file, and every exemption carries a reason", () => {
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
  it("TC-1445 writes every rendered tree under a temporary directory and removes it", () => {
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

/**
 * The rows above that `make template-gate` executes are executed inside a
 * rendered repository, in a temporary directory that exists only for the length
 * of that run. `quire coverage` cannot bind a test it can never see, so this
 * block binds them here: it names, for each of those rows, the rendered test
 * function that carries it, and asserts the function still exists in the
 * template's suite carrying the rendered acceptance criterion it claims.
 *
 * That is not a restatement of the execution. It is the check that catches the
 * other half of the failure: a rendered test deleted or renamed, which
 * `make template-gate` would report as a smaller green run rather than as a
 * loss.
 */
const RENDERED_ROWS: {
  tc: string;
  file: string;
  test: string;
  criterion: string;
}[] = [
  // TC-1408
  {
    tc: "TC-1408",
    file: "test_schema_emission.py",
    test: "test_one_schema_is_emitted_for_every_exported_type",
    criterion: "FR-002-AC-1",
  },
  // TC-1409
  {
    tc: "TC-1409",
    file: "test_schema_emission.py",
    test: "test_no_emitted_schema_is_the_placeholder_contract",
    criterion: "FR-002-AC-2",
  },
  // TC-1410
  {
    tc: "TC-1410",
    file: "test_manifest_semantic.py",
    test: "test_every_digest_equals_the_bytes_of_the_file_it_names",
    criterion: "FR-001-AC-7",
  },
  // TC-1411
  {
    tc: "TC-1411",
    file: "test_skeletons_semantic.py",
    test: "test_no_skeleton_carries_a_placeholder_body",
    criterion: "FR-003-AC-4",
  },
  // TC-1413
  {
    tc: "TC-1413",
    file: "test_schema_emission.py",
    test: "test_check_mode_is_green_against_the_committed_output",
    criterion: "FR-002-AC-5",
  },
  // TC-1417
  {
    tc: "TC-1417",
    file: "test_manifest_semantic.py",
    test: "test_semantic_block_carries_exactly_the_admitted_keys",
    criterion: "FR-001-AC-1",
  },
  // TC-1419
  {
    tc: "TC-1419",
    file: "test_manifest_semantic.py",
    test: "test_the_manifest_keeps_its_comments_and_is_not_reserialized",
    criterion: "FR-001-AC-9",
  },
  // TC-1420
  {
    tc: "TC-1420",
    file: "test_skeletons_semantic.py",
    test: "test_every_export_has_a_skeleton_in_the_typed_table_form",
    criterion: "FR-003-AC-1",
  },
  // TC-1421
  {
    tc: "TC-1421",
    file: "test_skeletons_semantic.py",
    test: "test_every_skeleton_has_a_sysml_alternate_declaring_the_same_fields",
    criterion: "FR-003-AC-2",
  },
  // TC-1422
  {
    tc: "TC-1422",
    file: "test_skeletons_semantic.py",
    test: "test_the_both_forms_fixture_carries_both_forms",
    criterion: "FR-003-AC-6",
  },
  // TC-1423
  {
    tc: "TC-1423",
    file: "test_skeletons_semantic.py",
    test: "test_every_skeleton_carries_an_ocl_clause_under_its_own_heading",
    criterion: "FR-003-AC-3",
  },
  // TC-1424
  {
    tc: "TC-1424",
    file: "test_skeletons_semantic.py",
    test: "test_every_negative_fixture_is_actually_refused",
    criterion: "FR-003-AC-15",
  },
  // TC-1425
  {
    tc: "TC-1425",
    file: "test_skeletons_semantic.py",
    test: "test_every_legacy_fixture_yields_no_error_under_warning",
    criterion: "FR-003-AC-11",
  },
  // TC-1426
  {
    tc: "TC-1426",
    file: "test_skeletons_semantic.py",
    test: "test_every_golden_record_matches_what_its_skeleton_extracts_to",
    criterion: "FR-003-AC-13",
  },
  // TC-1427
  {
    tc: "TC-1427",
    file: "test_skeletons_semantic.py",
    test: "test_every_skeleton_extracts_and_validates_against_its_emitted_schema",
    criterion: "FR-003-AC-9",
  },
  // TC-1430
  {
    tc: "TC-1430",
    file: "test_schema_emission.py",
    test: "test_the_package_metadata_declares_no_engine_dependency",
    criterion: "FR-002-AC-7",
  },
  // TC-1458
  {
    tc: "TC-1458",
    file: "test_manifest_semantic.py",
    test: "test_no_type_name_is_declared_twice",
    criterion: "FR-001-AC-4",
  },
  // TC-1468
  {
    tc: "TC-1468",
    file: "test_manifest_semantic.py",
    test: "test_exports_and_declared_type_names_are_the_same_set",
    criterion: "FR-001-AC-3",
  },
  // TC-1469
  {
    tc: "TC-1469",
    file: "test_skeletons_semantic.py",
    test: "test_each_negative_fixture_declares_its_own_distinct_expectation",
    criterion: "FR-003-AC-5",
  },
];

describe("the rendered suite carries the rows the template gate executes", () => {
  const suiteDir = join(TEMPLATE_DIR, "{{cookiecutter.repo_name}}", "tests");

  /**
   * One case per row, spelled out.
   *
   * `it.each` would be shorter and would bind nothing: the engine reads the
   * SOURCE, and an interpolated title carries no literal id for it to attach.
   * A single case listing nineteen ids is worse still — it binds one and leaves
   * eighteen rows claiming a backing they do not have, which is the status lie
   * this matrix exists to make impossible.
   */
  function stillCarries(tc: string): void {
    const row = RENDERED_ROWS.find((candidate) => candidate.tc === tc);
    expect(row, `${tc} is not a rendered row`).toBeDefined();
    const source = readFileSync(join(suiteDir, row!.file), "utf8");
    expect(source, `${row!.file} has no ${row!.test}`).toContain(
      `def ${row!.test}(`,
    );
    expect(source, `${row!.test} does not carry ${row!.criterion}`).toContain(
      `"${row!.criterion}"`,
    );
  }

  it("TC-1408 the rendered suite still carries its test", () => {
    // TC-1408
    stillCarries("TC-1408");
  });

  it("TC-1409 the rendered suite still carries its test", () => {
    // TC-1409
    stillCarries("TC-1409");
  });

  it("TC-1410 the rendered suite still carries its test", () => {
    // TC-1410
    stillCarries("TC-1410");
  });

  it("TC-1411 the rendered suite still carries its test", () => {
    // TC-1411
    stillCarries("TC-1411");
  });

  it("TC-1413 the rendered suite still carries its test", () => {
    // TC-1413
    stillCarries("TC-1413");
  });

  it("TC-1417 the rendered suite still carries its test", () => {
    // TC-1417
    stillCarries("TC-1417");
  });

  it("TC-1419 the rendered suite still carries its test", () => {
    // TC-1419
    stillCarries("TC-1419");
  });

  it("TC-1420 the rendered suite still carries its test", () => {
    // TC-1420
    stillCarries("TC-1420");
  });

  it("TC-1421 the rendered suite still carries its test", () => {
    // TC-1421
    stillCarries("TC-1421");
  });

  it("TC-1422 the rendered suite still carries its test", () => {
    // TC-1422
    stillCarries("TC-1422");
  });

  it("TC-1423 the rendered suite still carries its test", () => {
    // TC-1423
    stillCarries("TC-1423");
  });

  it("TC-1424 the rendered suite still carries its test", () => {
    // TC-1424
    stillCarries("TC-1424");
  });

  it("TC-1425 the rendered suite still carries its test", () => {
    // TC-1425
    stillCarries("TC-1425");
  });

  it("TC-1426 the rendered suite still carries its test", () => {
    // TC-1426
    stillCarries("TC-1426");
  });

  it("TC-1427 the rendered suite still carries its test", () => {
    // TC-1427
    stillCarries("TC-1427");
  });

  it("TC-1430 the rendered suite still carries its test", () => {
    // TC-1430
    stillCarries("TC-1430");
  });

  it("TC-1458 the rendered suite still carries its test", () => {
    // TC-1458
    stillCarries("TC-1458");
  });

  it("TC-1468 the rendered suite still carries its test", () => {
    // TC-1468
    stillCarries("TC-1468");
  });

  it("TC-1469 the rendered suite still carries its test", () => {
    // TC-1469
    stillCarries("TC-1469");
  });

  it("binds every rendered row exactly once and names no test twice", () => {
    const ids = RENDERED_ROWS.map((row) => row.tc);
    expect(new Set(ids).size).toBe(ids.length);
    const tests = RENDERED_ROWS.map((row) => `${row.file}::${row.test}`);
    expect(new Set(tests).size).toBe(tests.length);
  });
});
