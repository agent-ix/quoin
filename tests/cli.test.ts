import { chmodSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { stringify as stringifyYaml } from "yaml";

// The CLI's catalog/write handlers call ensureDefaultModules() internally with
// the committed default-modules.yaml, whose git-subdir sources would hit the
// network. Stub it to a no-op; tests supply the catalog hermetically through
// QUOIN_MODULE_PATHS (read by loadCatalog) instead.
vi.mock("../src/modules", () => ({
  ensureDefaultModules: () => {},
  defaultModulesManifest: () => ({ schemaVersion: 1, entries: [] }),
  filamentModulesDir: (home: string) => join(home, "filament", "modules"),
}));

import { main, packageVersion } from "../src/cli";

function tmp(prefix: string): string {
  return mkdtempSync(join(tmpdir(), `quoin-${prefix}-`));
}

// ---- console capture helpers -------------------------------------------------

function captureLog(): { lines: string[]; restore: () => void } {
  const lines: string[] = [];
  const spy = vi.spyOn(console, "log").mockImplementation((message) => {
    lines.push(String(message));
  });
  return { lines, restore: () => spy.mockRestore() };
}

function captureError(): { lines: string[]; restore: () => void } {
  const lines: string[] = [];
  const spy = vi.spyOn(console, "error").mockImplementation((message) => {
    lines.push(String(message));
  });
  return { lines, restore: () => spy.mockRestore() };
}

// ---- fixture module helpers --------------------------------------------------

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
        { name: "FR", frontmatter_schema_ref: "schemas/fr.schema.json" },
      ],
    },
    { "schemas/fr.schema.json": "{}", "skeletons/fr.md": "# FR\n" },
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

// Two fixture module roots joined into an QUOIN_MODULE_PATHS value.
function modulePaths(...roots: string[]): string {
  return roots.join(":");
}

// A populated catalog: returns the home dir and sets QUOIN_MODULE_PATHS.
function populatedCatalog(): string {
  const src = tmp("src");
  process.env.QUOIN_MODULE_PATHS = modulePaths(
    isoModule(src),
    businessModule(src),
  );
  return tmp("home");
}

// A catalog with a duplicate "domain" object type across two modules.
function duplicateCatalog(): string {
  const src = tmp("src");
  const extra = makeModule(src, "spec-objects-extra", {
    object_types: [{ name: "domain" }],
  });
  process.env.QUOIN_MODULE_PATHS = modulePaths(businessModule(src), extra);
  return tmp("home");
}

// ---- env save/restore --------------------------------------------------------

const savedEnv = {
  IX_HOME: process.env.IX_HOME,
  PATH: process.env.PATH,
  WF_ROOT: process.env.IX_SPEC_WORKFLOWS_ROOT,
  MODULE_PATHS: process.env.QUOIN_MODULE_PATHS,
};
let savedExitCode: number | undefined;

beforeEach(() => {
  savedExitCode = process.exitCode;
});

afterEach(() => {
  if (savedEnv.IX_HOME === undefined) delete process.env.IX_HOME;
  else process.env.IX_HOME = savedEnv.IX_HOME;
  process.env.PATH = savedEnv.PATH;
  if (savedEnv.WF_ROOT === undefined) delete process.env.IX_SPEC_WORKFLOWS_ROOT;
  else process.env.IX_SPEC_WORKFLOWS_ROOT = savedEnv.WF_ROOT;
  if (savedEnv.MODULE_PATHS === undefined)
    delete process.env.QUOIN_MODULE_PATHS;
  else process.env.QUOIN_MODULE_PATHS = savedEnv.MODULE_PATHS;
  process.exitCode = savedExitCode;
});

// ---- main() dispatch ---------------------------------------------------------

describe("main dispatch", () => {
  test("--version, -v, and the version command all print the package version", async () => {
    const c = captureLog();
    try {
      await main(["--version"]);
      await main(["-v"]);
      await main(["version"]);
    } finally {
      c.restore();
    }
    expect(c.lines).toEqual([
      packageVersion(),
      packageVersion(),
      packageVersion(),
    ]);
  });

  test("no command prints the top-level usage", async () => {
    const c = captureLog();
    try {
      await main([]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("Spec workflow and catalog CLI");
  });

  test("--help and -h print command help", async () => {
    const c = captureLog();
    try {
      await main(["plugin", "--help"]);
      await main(["write", "-h"]);
      await main(["update", "-h"]); // helpFor -> UPDATE_USAGE
      await main(["--help"]); // no command -> USAGE
    } finally {
      c.restore();
    }
    expect(c.lines[0]).toContain(
      "Install and manage user/community spec modules",
    );
    expect(c.lines[1]).toContain("Build an authoring pack");
    expect(c.lines[2]).toContain("quoin update");
    expect(c.lines[3]).toContain("Spec workflow and catalog CLI");
  });

  test("help for a spec-flow command prints the workflow usage", async () => {
    const c = captureLog();
    try {
      await main(["review", "--help"]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("Start bundled spec review/planning");
  });

  test("--help on an unrecognized command falls back to the top-level usage", async () => {
    // command is truthy (so the help branch runs) but matches no known command,
    // exercising helpFor's USAGE fallthrough.
    const c = captureLog();
    try {
      await main(["bogus", "--help", "--config-root", tmp("home")]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("Spec workflow and catalog CLI");
  });

  test("throws on an unknown command", async () => {
    await expect(main(["bogus", "--config-root", tmp("home")])).rejects.toThrow(
      /unknown command bogus/,
    );
  });
});

// ---- catalog -----------------------------------------------------------------

describe("runCatalog", () => {
  test("list (explicit) prints one line per module", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await main(["catalog", "list", "--config-root", home]);
    } finally {
      c.restore();
    }
    const text = c.lines.join("\n");
    expect(text).toContain("spec-artifacts-iso@0.1.0");
    expect(text).toContain("spec-objects-business@0.1.0");
  });

  test("falls back to IX_HOME when --config-root is omitted, and prints 'unknown' for a versionless module", async () => {
    // No --config-root -> main uses ixHome() (IX_HOME). A module without a
    // version field renders the `?? "unknown"` branch.
    const src = tmp("src-noversion");
    const noVersion = makeModule(src, "spec-objects-noversion", {
      object_types: [{ name: "thing" }],
    });
    // Strip the version that makeModule injects.
    writeFileSync(
      join(noVersion, "manifest.yaml"),
      stringifyYaml({
        name: "spec-objects-noversion",
        object_types: [{ name: "thing" }],
      }),
    );
    process.env.QUOIN_MODULE_PATHS = noVersion;
    process.env.IX_HOME = tmp("home");
    const c = captureLog();
    try {
      await main(["catalog", "list"]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("spec-objects-noversion@unknown");
  });

  test("undefined subcommand behaves like list", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await main(["catalog", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("spec-artifacts-iso");
  });

  test("list --json prints the whole catalog as JSON", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await main(["catalog", "list", "--json", "--config-root", home]);
    } finally {
      c.restore();
    }
    const parsed = JSON.parse(c.lines.join("\n"));
    expect(parsed.modules.map((m: { name: string }) => m.name)).toContain(
      "spec-artifacts-iso",
    );
  });

  test("show prints a single entry (text)", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await main(["catalog", "show", "FR", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("artifact FR from spec-artifacts-iso");
  });

  test("show --json prints the entry as JSON", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await main(["catalog", "show", "FR", "--json", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(JSON.parse(c.lines.join("\n")).name).toBe("FR");
  });

  test("show without a type throws", async () => {
    const home = populatedCatalog();
    await expect(
      main(["catalog", "show", "--config-root", home]),
    ).rejects.toThrow("catalog show requires <type>");
  });

  test("show of a missing type throws", async () => {
    const home = populatedCatalog();
    await expect(
      main(["catalog", "show", "NOPE", "--config-root", home]),
    ).rejects.toThrow("catalog type not found: NOPE");
  });

  test("validate succeeds with no duplicates (text)", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await main(["catalog", "validate", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toMatch(/catalog ok \(2 modules\)/);
    expect(process.exitCode).not.toBe(1);
  });

  test("validate --json succeeds with no duplicates", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await main(["catalog", "validate", "--json", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(JSON.parse(c.lines.join("\n"))).toEqual({ ok: true, modules: 2 });
  });

  test("validate reports duplicates on stderr and sets exit code 1", async () => {
    const home = duplicateCatalog();
    const err = captureError();
    try {
      await main(["catalog", "validate", "--config-root", home]);
    } finally {
      err.restore();
    }
    const parsed = JSON.parse(err.lines.join("\n"));
    expect(parsed.ok).toBe(false);
    expect(parsed.duplicates[0].name).toBe("domain");
    expect(process.exitCode).toBe(1);
  });

  test("unknown catalog subcommand throws", async () => {
    const home = populatedCatalog();
    await expect(
      main(["catalog", "bogus", "--config-root", home]),
    ).rejects.toThrow("unknown catalog command bogus");
  });
});

// ---- plugin ------------------------------------------------------------------

describe("runPlugin", () => {
  test("list (explicit and undefined) prints the plugin registry as JSON", async () => {
    const home = tmp("home");
    const c = captureLog();
    try {
      await main(["plugin", "list", "--config-root", home]);
      await main(["plugin", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(JSON.parse(c.lines[0])).toHaveProperty("plugins");
    expect(JSON.parse(c.lines[1])).toHaveProperty("plugins");
  });

  test("install adds a plugin from a path source", async () => {
    const home = tmp("home");
    const mod = businessModule(tmp("plugin-src"));
    const c = captureLog();
    try {
      await main(["plugin", "install", `path:${mod}`, "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(JSON.parse(c.lines.join("\n")).name).toBe("spec-objects-business");
  });

  test("install without a source throws", async () => {
    const home = tmp("home");
    await expect(
      main(["plugin", "install", "--config-root", home]),
    ).rejects.toThrow("plugin install requires <source>");
  });

  test("ensure-defaults runs the installer and reports the registry", async () => {
    const home = tmp("home");
    const c = captureLog();
    try {
      // ensureDefaultModules is mocked to a no-op, so this exercises the
      // command dispatch + output contract without hitting the network.
      await main(["plugin", "ensure-defaults", "--config-root", home]);
    } finally {
      c.restore();
    }
    const out = JSON.parse(c.lines.join("\n"));
    expect(out.ensured).toBe(true);
    expect(Array.isArray(out.plugins)).toBe(true);
  });

  test("ensure-defaults reports installed plugin names from a non-empty registry", async () => {
    const home = tmp("home");
    const mod = businessModule(tmp("plugin-src"));
    const c = captureLog();
    try {
      await main(["plugin", "install", `path:${mod}`, "--config-root", home]);
      // With a plugin already in the registry, ensure-defaults exercises the
      // `listPlugins().map((p) => p.name)` reporting path (cli.ts).
      await main(["plugin", "ensure-defaults", "--config-root", home]);
    } finally {
      c.restore();
    }
    const out = JSON.parse(c.lines[c.lines.length - 1]);
    expect(out.ensured).toBe(true);
    expect(out.plugins).toContain("spec-objects-business");
  });

  test("remove deletes a plugin and prints confirmation", async () => {
    const home = tmp("home");
    const mod = businessModule(tmp("plugin-src"));
    const c = captureLog();
    try {
      await main(["plugin", "install", `path:${mod}`, "--config-root", home]);
      await main([
        "plugin",
        "remove",
        "spec-objects-business",
        "--config-root",
        home,
      ]);
    } finally {
      c.restore();
    }
    expect(c.lines).toContain("removed spec-objects-business");
  });

  test("remove without a name throws", async () => {
    const home = tmp("home");
    await expect(
      main(["plugin", "remove", "--config-root", home]),
    ).rejects.toThrow("plugin remove requires <name>");
  });

  test("unknown plugin subcommand throws", async () => {
    const home = tmp("home");
    await expect(
      main(["plugin", "bogus", "--config-root", home]),
    ).rejects.toThrow("unknown plugin command bogus");
  });
});

// ---- write -------------------------------------------------------------------

describe("runWrite", () => {
  test("missing repo_dir throws", async () => {
    const home = populatedCatalog();
    await expect(
      main(["write", "--types", "FR", "--config-root", home]),
    ).rejects.toThrow(/write requires <repo_dir>/);
  });

  test("renders an authoring pack as text", async () => {
    const home = populatedCatalog();
    const repo = tmp("repo");
    const c = captureLog();
    try {
      await main([
        "write",
        repo,
        "--types",
        "FR,domain",
        "--config-root",
        home,
      ]);
    } finally {
      c.restore();
    }
    const text = c.lines.join("\n");
    expect(text).toContain("Authoring contracts:");
    expect(text).toContain("- FR (artifact)");
    expect(text).toContain("- domain (object)");
  });

  test("renders an authoring pack as JSON with --json", async () => {
    const home = populatedCatalog();
    const repo = tmp("repo");
    const c = captureLog();
    try {
      await main([
        "write",
        repo,
        "--types",
        "FR",
        "--json",
        "--config-root",
        home,
      ]);
    } finally {
      c.restore();
    }
    const parsed = JSON.parse(c.lines.join("\n"));
    expect(parsed.repoRoot).toBe(repo);
    expect(parsed.types[0].name).toBe("FR");
  });
});

// ---- spec-flow commands via main ---------------------------------------------

describe("spec-flow dispatch", () => {
  function packagedRoot(flow: "review" | "matrix" | "to-plan"): string {
    const packageSkill = {
      review: "spec-review",
      matrix: "spec-matrix",
      "to-plan": "spec-to-plan",
    }[flow];
    const root = tmp("wf");
    mkdirSync(join(root, packageSkill, "workflow-assets", "skills", flow), {
      recursive: true,
    });
    return root;
  }

  function fakeIxFlowDir(exitCode: number): string {
    const dir = tmp("bin");
    const bin = join(dir, "ix-flow");
    writeFileSync(bin, `#!/bin/sh\nexit ${exitCode}\n`);
    chmodSync(bin, 0o755);
    return dir;
  }

  test("review with --target/--json/--id runs the flow (ix-flow exit 0)", async () => {
    const home = tmp("home");
    process.env.IX_SPEC_WORKFLOWS_ROOT = packagedRoot("review");
    process.env.PATH = `${fakeIxFlowDir(0)}:${savedEnv.PATH}`;
    process.exitCode = undefined;
    await main([
      "review",
      "--target",
      "spec/",
      "--target",
      "spec/two",
      "--json",
      "--id",
      "run-1",
      "--config-root",
      home,
    ]);
    expect(process.exitCode).toBeUndefined();
  });

  test("matrix runs the flow and propagates a non-zero exit code", async () => {
    const home = tmp("home");
    process.env.IX_SPEC_WORKFLOWS_ROOT = packagedRoot("matrix");
    process.env.PATH = `${fakeIxFlowDir(2)}:${savedEnv.PATH}`;
    process.exitCode = undefined;
    await main(["matrix", "--config-root", home]);
    expect(process.exitCode).toBe(2);
  });
});

// ---- parseArgs edge cases ----------------------------------------------------
// parseArgs is private; exercised through main's observable behavior.

describe("parseArgs via main", () => {
  test("long flag with = and following-value form both work", async () => {
    // --config-root=<home> (eq form) and --types FR (following-value form).
    const home = populatedCatalog();
    const repo = tmp("repo");
    const c = captureLog();
    try {
      await main([`--config-root=${home}`, "write", repo, "--types", "FR"]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("- FR (artifact)");
  });

  test("boolean long flag (no value) is honored", async () => {
    // --json with no following non-flag value is treated as boolean true.
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await main(["catalog", "list", "--config-root", home, "--json"]);
    } finally {
      c.restore();
    }
    expect(() => JSON.parse(c.lines.join("\n"))).not.toThrow();
  });

  test("subcommand is only captured for catalog/plugin; others become positionals", async () => {
    // For `write`, the second bareword is a positional (repo_dir), not a subcommand.
    const home = populatedCatalog();
    const repo = tmp("repo");
    const c = captureLog();
    try {
      await main(["write", repo, "--types", "FR", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain(`Repo: ${repo}`);
  });

  test("--no-project-config short-circuits project config root", async () => {
    // Exercises the parsed.flags['no-project-config'] !== true branch (set true).
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await main([
        "catalog",
        "list",
        "--no-project-config",
        "--config-root",
        home,
      ]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("spec-artifacts-iso");
  });
});

// ---- packageVersion ----------------------------------------------------------

describe("packageVersion", () => {
  test("returns a non-empty version string", () => {
    expect(typeof packageVersion()).toBe("string");
    expect(packageVersion().length).toBeGreaterThan(0);
  });
});
