import { execSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { settings, type Config } from "@oclif/core";

import { loadConfig, listCorePlugins } from "@agent-ix/ix-cli-core";

// ===========================================================================
// IT-005 / Gate G3 — preinstalled core-plugin discovery (FR-018, NFR-008)
//
// quoin declares `@agent-ix/filament-plan-sync` in its package.json
// `oclif.plugins` array AND as a dependency. ix-cli-core's runner (via
// @oclif/core's Config loader) discovers that package as a *core* plugin and
// surfaces its `sync` command with NO runtime install step. This test asserts
// the discovery wiring end-to-end against the built command graph.
// ===========================================================================

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const SYNC_PLUGIN = "@agent-ix/filament-plan-sync";

let config: Config;

beforeAll(async () => {
  // Production loads dist/commands directly; pin that behavior so discovery is
  // exercised against the built graph (mirrors cli.test.ts).
  settings.enableAutoTranspile = false;
  if (!existsSync(join(repoRoot, "dist", "commands", "update.js"))) {
    execSync("pnpm run build", { cwd: repoRoot, stdio: "inherit" });
  }
  // A single loadConfig() — there is deliberately NO install/link step here:
  // the plugin must already be discoverable because it ships as a dependency.
  config = await loadConfig({ root: repoRoot });
});

describe("IT-005 sync command discovery (Gate G3)", () => {
  test("filament-plan-sync is discovered as a CORE plugin contributing `sync`", () => {
    const corePlugins = listCorePlugins(config);
    const plugin = corePlugins.find((p) => p.name === SYNC_PLUGIN);
    expect(plugin).toBeDefined();
    expect(plugin?.type).toBe("core");
    expect(plugin?.commandIDs).toContain("sync");
  });

  // @requirement TC-108 NFR-008 — fresh install -> `quoin sync` resolves with
  // zero extra steps (preinstalled core plugin, no runtime install).
  test("TC-108 / NFR-008: the `sync` command resolves with no runtime install step", () => {
    const cmd = config.findCommand("sync");
    expect(cmd).toBeDefined();
    // it is contributed by the core plugin, not by quoin's own command dir
    expect(cmd?.pluginName).toBe(SYNC_PLUGIN);
    expect(cmd?.pluginType).toBe("core");
  });

  test("an existing host command (catalog) still resolves unchanged", () => {
    const cmd = config.findCommand("catalog");
    expect(cmd).toBeDefined();
    // catalog is contributed by quoin itself (the root plugin), not the plugin
    expect(cmd?.pluginName).not.toBe(SYNC_PLUGIN);
    expect(config.commandIDs).toContain("catalog:list");
  });
});
