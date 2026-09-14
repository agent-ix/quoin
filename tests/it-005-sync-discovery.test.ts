import { execSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { Config, settings } from "@oclif/core";


// ===========================================================================
// FR-026-AC-7 — preinstalled core-plugin discovery
//
// quoin declares `@agent-ix/filament-plan-sync` in its package.json
// `oclif.plugins` array AND as a dependency. oclif's own Config loader
// discovers that package as a *core* plugin and surfaces its `sync` command
// with NO runtime install step. This test deliberately avoids ix-cli-core: it
// remains the successor proof when the Stage 9 shell retirement deletes that
// package (quoin#396).
// ===========================================================================

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const SYNC_PLUGIN = "@agent-ix/filament-plan-sync";

let config: Config;

beforeAll(async () => {
  // Production loads dist/commands directly; pin that behavior so discovery is
  // exercised against the built graph (mirrors cli.test.ts).
  settings.enableAutoTranspile = false;
  if (!existsSync(join(repoRoot, "dist", "commands", "update.js"))) {
    execSync("corepack pnpm run build", { cwd: repoRoot, stdio: "inherit" });
  }
  // A single Config.load() — there is deliberately NO install/link step here:
  // the plugin must already be discoverable because it ships as a dependency.
  config = await Config.load({ root: repoRoot });
});

describe("core-plugin discovery (FR-026-AC-7)", () => {
  // Trace: FR-026-AC-7
  test("filament-plan-sync is discovered as a CORE plugin contributing `sync`", () => {
    const plugin = config.plugins.get(SYNC_PLUGIN);
    expect(plugin).toBeDefined();
    expect(plugin?.type).toBe("core");
    expect(plugin?.commandIDs).toContain("sync");
  });

  // FR-026-AC-7 — fresh install -> `quoin sync` resolves with
  // zero extra steps (preinstalled core plugin, no runtime install).
  // Trace: FR-026-AC-7
  test("the `sync` command resolves with no runtime install step", () => {
    const cmd = config.findCommand("sync");
    expect(cmd).toBeDefined();
    // it is contributed by the core plugin, not by quoin's own command dir
    expect(cmd?.pluginName).toBe(SYNC_PLUGIN);
    expect(cmd?.pluginType).toBe("core");
  });

  // Trace: FR-026-AC-7
  test("an existing host command (catalog) still resolves unchanged", () => {
    const cmd = config.findCommand("catalog");
    expect(cmd).toBeDefined();
    // catalog is contributed by quoin itself (the root plugin), not the plugin
    expect(cmd?.pluginName).not.toBe(SYNC_PLUGIN);
    expect(config.commandIDs).toContain("catalog:list");
  });
});
