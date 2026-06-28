import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { beforeAll, describe, expect, it, vi } from "vitest";

// `quoin update` delegates to ix-cli-core's runSelfUpdate. Partial-mock the
// module so runSelfUpdate is a spy (no network / npm) while BaseCommand,
// loadConfig, and the rest of the runtime remain real — the Update command is a
// BaseCommand subclass and must run against the genuine oclif lifecycle.
const { runSelfUpdate } = vi.hoisted(() => ({
  runSelfUpdate: vi.fn(async () => ({ updated: false, latest: "9.9.9" })),
}));
vi.mock("@agent-ix/ix-cli-core", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@agent-ix/ix-cli-core")>();
  return { ...actual, runSelfUpdate };
});

import { loadConfig } from "@agent-ix/ix-cli-core";

import Update from "../src/commands/update";
import { packageVersion } from "../src/cli";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

function tmpHome(): string {
  return mkdtempSync(join(tmpdir(), "quoin-update-"));
}

let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

describe("quoin update", () => {
  it("delegates to runSelfUpdate with quoin's package coordinates", async () => {
    runSelfUpdate.mockClear();
    await Update.run(["--config-root", tmpHome()], config);
    expect(runSelfUpdate).toHaveBeenCalledTimes(1);
    expect(runSelfUpdate).toHaveBeenCalledWith({
      packageName: "@agent-ix/quoin",
      currentVersion: packageVersion(),
      header: "quoin update",
      registry: "https://registry.npmjs.org/",
      check: false,
    });
  });

  it("defaults the registry to public npm when no --registry is given", async () => {
    runSelfUpdate.mockClear();
    await Update.run(["--config-root", tmpHome()], config);
    expect(runSelfUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ registry: "https://registry.npmjs.org/" }),
    );
  });

  it("passes --check through", async () => {
    runSelfUpdate.mockClear();
    await Update.run(["--check", "--config-root", tmpHome()], config);
    expect(runSelfUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ check: true }),
    );
  });

  it("passes a custom --registry through", async () => {
    runSelfUpdate.mockClear();
    await Update.run(
      ["--registry", "http://npm.ix/", "--config-root", tmpHome()],
      config,
    );
    expect(runSelfUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ registry: "http://npm.ix/" }),
    );
  });
});
