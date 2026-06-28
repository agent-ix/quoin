import { execSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { settings, type Config } from "@oclif/core";
import { stringify as stringifyYaml } from "yaml";

import { loadConfig, run } from "@agent-ix/ix-cli-core";

// The catalog/write handlers call ensureDefaultModules() internally with the
// committed default-modules.yaml, whose git-subdir sources would hit the
// network. Stub it to a no-op; tests supply the catalog hermetically through
// QUOIN_MODULE_PATHS (read by loadCatalog) instead. The command classes import
// the SAME src/modules module, so this mock applies to them.
vi.mock("../src/modules", () => ({
  ensureDefaultModules: () => {},
  defaultModulesManifest: () => ({ schemaVersion: 1, entries: [] }),
}));

import { main, packageVersion } from "../src/cli";
import CatalogIndex from "../src/commands/catalog/index";
import CatalogList from "../src/commands/catalog/list";
import CatalogShow from "../src/commands/catalog/show";
import CatalogValidate from "../src/commands/catalog/validate";
import ModuleIndex from "../src/commands/module/index";
import ModuleList from "../src/commands/module/list";
import ModuleInstall from "../src/commands/module/install";
import ModuleRemove from "../src/commands/module/remove";
import ModuleEnsureDefaults from "../src/commands/module/ensure-defaults";
import PluginList from "../src/commands/plugin/list";
import PluginInstall from "../src/commands/plugin/install";
import PluginRemove from "../src/commands/plugin/remove";
import PluginEnsureDefaults from "../src/commands/plugin/ensure-defaults";
import Write from "../src/commands/write";
import Review from "../src/commands/review";
import Matrix from "../src/commands/matrix";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

// A single oclif Config rooted at the quoin package; reused so every command is
// instantiated against the real plugin/command graph (FR-016). `Command.run`
// uses the passed class directly, so command bodies execute from src (keeping
// the modules mock effective) while the runner config supplies context.
let config: Config;

// oclif Command base type narrowed to its static `run`. The command classes are
// concrete subclasses; this captures the shape we invoke in tests.
type RunnableCommand = {
  run: (argv: string[], opts: Config) => Promise<unknown>;
};

async function runCmd(Cmd: RunnableCommand, argv: string[]): Promise<void> {
  await Cmd.run(argv, config);
}

function tmp(prefix: string): string {
  return mkdtempSync(join(tmpdir(), `quoin-${prefix}-`));
}

// ---- console capture helpers -------------------------------------------------
// oclif's `this.log` routes to console.log and `this.logToStderr` to
// console.error, so the legacy capture helpers still observe command output.

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

function modulePaths(...roots: string[]): string {
  return roots.join(":");
}

function populatedCatalog(): string {
  const src = tmp("src");
  process.env.QUOIN_MODULE_PATHS = modulePaths(
    isoModule(src),
    businessModule(src),
  );
  return tmp("home");
}

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

beforeAll(async () => {
  // Under vitest NODE_ENV=test, oclif would auto-transpile and resolve commands
  // from src/*.ts (whose ".js" import specifiers don't resolve via raw
  // import()). Production loads dist/commands directly; pin that behavior so the
  // discovery/dispatch assertions exercise the built command graph.
  settings.enableAutoTranspile = false;
  if (!existsSync(join(repoRoot, "dist", "commands", "update.js"))) {
    execSync("pnpm run build", { cwd: repoRoot, stdio: "inherit" });
  }
  config = await loadConfig({ root: repoRoot });
});

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

// ---- runner / dispatch parity (TC-016, TC-107) -------------------------------

describe("oclif runner dispatch parity", () => {
  test("the runner discovers every migrated command (no legacy dispatcher)", () => {
    const ids = new Set(config.commandIDs);
    for (const id of [
      "update",
      "write",
      "review",
      "matrix",
      "to-plan",
      "catalog",
      "catalog:list",
      "catalog:show",
      "catalog:validate",
      "module",
      "module:list",
      "module:install",
      "module:remove",
      "module:ensure-defaults",
      "plugin",
      "plugin:list",
      "plugin:install",
      "plugin:remove",
      "plugin:ensure-defaults",
    ]) {
      expect(ids.has(id)).toBe(true);
    }
  });

  test("subcommands are space-separated (topicSeparator)", () => {
    expect(config.topicSeparator).toBe(" ");
  });

  test("the hand-rolled parseArgs dispatcher is gone from cli.ts", () => {
    const source = readFileSync(join(repoRoot, "src", "cli.ts"), "utf8");
    expect(source).not.toContain("function parseArgs");
    expect(source).not.toContain("function runCatalog");
    expect(source).not.toContain("function runPlugin");
  });

  test("an unknown command is rejected by the runner", async () => {
    await expect(run(["bogus"], config)).rejects.toThrow();
  });

  test("an unknown subcommand is rejected by the runner", async () => {
    await expect(run(["catalog", "bogus"], config)).rejects.toThrow();
  });
});

// ---- version -----------------------------------------------------------------

describe("version", () => {
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

  test("packageVersion returns a non-empty string", () => {
    expect(typeof packageVersion()).toBe("string");
    expect(packageVersion().length).toBeGreaterThan(0);
  });
});

// ---- catalog -----------------------------------------------------------------

describe("catalog", () => {
  test("list (explicit) prints one line per module", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await runCmd(CatalogList, ["--config-root", home]);
    } finally {
      c.restore();
    }
    const text = c.lines.join("\n");
    expect(text).toContain("spec-artifacts-iso@0.1.0");
    expect(text).toContain("spec-objects-business@0.1.0");
  });

  test("falls back to IX_HOME when --config-root is omitted, and prints 'unknown' for a versionless module", async () => {
    const src = tmp("src-noversion");
    const noVersion = makeModule(src, "spec-objects-noversion", {
      object_types: [{ name: "thing" }],
    });
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
      await runCmd(CatalogList, []);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("spec-objects-noversion@unknown");
  });

  test("the catalog index command behaves like list", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await runCmd(CatalogIndex, ["--config-root", home]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("spec-artifacts-iso");
  });

  test("list --json prints the whole catalog as JSON", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await runCmd(CatalogList, ["--json", "--config-root", home]);
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
      await runCmd(CatalogShow, ["FR", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("artifact FR from spec-artifacts-iso");
  });

  test("show --json prints the entry as JSON", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await runCmd(CatalogShow, ["FR", "--json", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(JSON.parse(c.lines.join("\n")).name).toBe("FR");
  });

  test("show without a type throws", async () => {
    const home = populatedCatalog();
    await expect(runCmd(CatalogShow, ["--config-root", home])).rejects.toThrow(
      "catalog show requires <type>",
    );
  });

  test("show of a missing type throws", async () => {
    const home = populatedCatalog();
    await expect(
      runCmd(CatalogShow, ["NOPE", "--config-root", home]),
    ).rejects.toThrow("catalog type not found: NOPE");
  });

  test("validate succeeds with no duplicates (text)", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await runCmd(CatalogValidate, ["--config-root", home]);
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
      await runCmd(CatalogValidate, ["--json", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(JSON.parse(c.lines.join("\n"))).toEqual({ ok: true, modules: 2 });
  });

  test("validate reports duplicates on stderr and sets exit code 1", async () => {
    const home = duplicateCatalog();
    const err = captureError();
    try {
      await runCmd(CatalogValidate, ["--config-root", home]);
    } finally {
      err.restore();
    }
    const parsed = JSON.parse(err.lines.join("\n"));
    expect(parsed.ok).toBe(false);
    expect(parsed.duplicates[0].name).toBe("domain");
    expect(process.exitCode).toBe(1);
  });
});

// ---- module (canonical spec-module command, TC-017) --------------------------

describe("module", () => {
  test("list and index both print the registry as JSON", async () => {
    const home = tmp("home");
    const c = captureLog();
    try {
      await runCmd(ModuleList, ["--config-root", home]);
      await runCmd(ModuleIndex, ["--config-root", home]);
    } finally {
      c.restore();
    }
    expect(JSON.parse(c.lines[0])).toHaveProperty("plugins");
    expect(JSON.parse(c.lines[1])).toHaveProperty("plugins");
  });

  test("install adds a module from a path source", async () => {
    const home = tmp("home");
    const mod = businessModule(tmp("module-src"));
    const c = captureLog();
    try {
      await runCmd(ModuleInstall, [`path:${mod}`, "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(JSON.parse(c.lines.join("\n")).name).toBe("spec-objects-business");
  });

  test("install without a source throws", async () => {
    const home = tmp("home");
    await expect(
      runCmd(ModuleInstall, ["--config-root", home]),
    ).rejects.toThrow("module install requires <source>");
  });

  test("ensure-defaults runs the installer and reports the registry", async () => {
    const home = tmp("home");
    const c = captureLog();
    try {
      await runCmd(ModuleEnsureDefaults, ["--config-root", home]);
    } finally {
      c.restore();
    }
    const out = JSON.parse(c.lines.join("\n"));
    expect(out.ensured).toBe(true);
    expect(Array.isArray(out.plugins)).toBe(true);
  });

  test("ensure-defaults reports installed module names from a non-empty registry", async () => {
    const home = tmp("home");
    const mod = businessModule(tmp("module-src"));
    const c = captureLog();
    try {
      await runCmd(ModuleInstall, [`path:${mod}`, "--config-root", home]);
      await runCmd(ModuleEnsureDefaults, ["--config-root", home]);
    } finally {
      c.restore();
    }
    const out = JSON.parse(c.lines[c.lines.length - 1]);
    expect(out.ensured).toBe(true);
    expect(out.plugins).toContain("spec-objects-business");
  });

  test("remove deletes a module and prints confirmation", async () => {
    const home = tmp("home");
    const mod = businessModule(tmp("module-src"));
    const c = captureLog();
    try {
      await runCmd(ModuleInstall, [`path:${mod}`, "--config-root", home]);
      await runCmd(ModuleRemove, [
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
    await expect(runCmd(ModuleRemove, ["--config-root", home])).rejects.toThrow(
      "module remove requires <name>",
    );
  });
});

// ---- plugin (deprecated alias, TC-017) ---------------------------------------

describe("plugin (deprecated alias)", () => {
  test("plugin list warns to stderr and forwards to module list", async () => {
    const home = tmp("home");
    const out = captureLog();
    const err = captureError();
    try {
      await runCmd(PluginList, ["--config-root", home]);
    } finally {
      out.restore();
      err.restore();
    }
    expect(err.lines.join("\n")).toMatch(/deprecated/i);
    expect(JSON.parse(out.lines.join("\n"))).toHaveProperty("plugins");
  });

  test("plugin install warns and forwards to module install", async () => {
    const home = tmp("home");
    const mod = businessModule(tmp("module-src"));
    const out = captureLog();
    const err = captureError();
    try {
      await runCmd(PluginInstall, [`path:${mod}`, "--config-root", home]);
    } finally {
      out.restore();
      err.restore();
    }
    expect(err.lines.join("\n")).toMatch(/deprecated/i);
    expect(JSON.parse(out.lines.join("\n")).name).toBe("spec-objects-business");
  });

  test("plugin remove warns and forwards to module remove", async () => {
    const home = tmp("home");
    const mod = businessModule(tmp("module-src"));
    const out = captureLog();
    const err = captureError();
    try {
      await runCmd(PluginInstall, [`path:${mod}`, "--config-root", home]);
      await runCmd(PluginRemove, [
        "spec-objects-business",
        "--config-root",
        home,
      ]);
    } finally {
      out.restore();
      err.restore();
    }
    expect(err.lines.join("\n")).toMatch(/deprecated/i);
    expect(out.lines).toContain("removed spec-objects-business");
  });

  test("plugin ensure-defaults warns and forwards to module ensure-defaults", async () => {
    const home = tmp("home");
    const out = captureLog();
    const err = captureError();
    try {
      await runCmd(PluginEnsureDefaults, ["--config-root", home]);
    } finally {
      out.restore();
      err.restore();
    }
    expect(err.lines.join("\n")).toMatch(/deprecated/i);
    expect(JSON.parse(out.lines.join("\n")).ensured).toBe(true);
  });
});

// ---- write -------------------------------------------------------------------

describe("write", () => {
  test("missing repo_dir throws", async () => {
    const home = populatedCatalog();
    await expect(
      runCmd(Write, ["--types", "FR", "--config-root", home]),
    ).rejects.toThrow(/write requires <repo_dir>/);
  });

  test("renders an authoring pack as text", async () => {
    const home = populatedCatalog();
    const repo = tmp("repo");
    const c = captureLog();
    try {
      await runCmd(Write, [
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
      await runCmd(Write, [
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

// ---- spec-flow launchers -----------------------------------------------------

describe("spec-flow launchers", () => {
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
    await runCmd(Review, [
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
    await runCmd(Matrix, ["--config-root", home]);
    expect(process.exitCode).toBe(2);
  });
});

// ---- flag parsing (now owned by oclif) ---------------------------------------

describe("flag parsing", () => {
  test("--config-root=<home> (eq form) and repeated/positional flags work", async () => {
    const home = populatedCatalog();
    const repo = tmp("repo");
    const c = captureLog();
    try {
      await runCmd(Write, [repo, "--types", "FR", `--config-root=${home}`]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("- FR (artifact)");
  });

  test("boolean --json flag (no value) is honored", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await runCmd(CatalogList, ["--config-root", home, "--json"]);
    } finally {
      c.restore();
    }
    expect(() => JSON.parse(c.lines.join("\n"))).not.toThrow();
  });

  test("--no-project-config is accepted", async () => {
    const home = populatedCatalog();
    const c = captureLog();
    try {
      await runCmd(CatalogList, ["--no-project-config", "--config-root", home]);
    } finally {
      c.restore();
    }
    expect(c.lines.join("\n")).toContain("spec-artifacts-iso");
  });
});
