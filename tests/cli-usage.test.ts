/**
 * Coverage for the unknown-command usage path added for FR-005-AC-1
 * (SR-003 FND-001).
 *
 * The remedy lives in oclif's own `command_not_found` hook
 * (`src/hooks/command-not-found.ts`), which `Config.runCommand` invokes and
 * whose failure it rethrows in place of the default `command <x> not found`.
 * That keeps `bin/quoin.js` on `execute()` — whose `flush()` the shipped CLI
 * needs so piped output is not truncated — instead of rerouting around it.
 */
import { Errors } from "@oclif/core";

import commandNotFound from "../src/hooks/command-not-found";
import { rootUsage } from "../src/cli";

const config = (
  commands: Array<{ id: string; hidden?: boolean }>,
  bin = "quoin",
) => ({ bin, commands });

// Trace: FR-005-AC-1
describe("rootUsage", () => {
  test("lists visible top-level commands, sorted and de-duplicated", () => {
    const usage = rootUsage(
      config([
        { id: "write" },
        { id: "catalog:list" },
        { id: "catalog:show" },
        { id: "config:get" },
      ]),
    );
    expect(usage).toContain("Usage: quoin <command> [options]");
    expect(usage).toContain("Commands: catalog, config, write");
    expect(usage).toContain("Run `quoin <command> --help` for details.");
  });

  test("omits hidden commands, so usage never advertises a deprecated way in", () => {
    const usage = rootUsage(
      config([{ id: "write" }, { id: "plugin:list", hidden: true }]),
    );
    expect(usage).toContain("Commands: write");
    expect(usage).not.toMatch(/Commands:[^\n]*\bplugin\b/);
  });

  test("names whatever bin the config declares, rather than a baked-in string", () => {
    expect(rootUsage(config([{ id: "write" }], "other"))).toContain(
      "Usage: other <command>",
    );
  });
});

// Trace: FR-005-AC-1
describe("command_not_found hook", () => {
  const invoke = (ctxConfig: ReturnType<typeof config>, id: string) =>
    (
      commandNotFound as unknown as (
        this: { config: unknown },
        options: { id: string; argv: string[] },
      ) => Promise<void>
    ).call({ config: ctxConfig }, { id, argv: [] });

  test("throws a CLIError naming the command and carrying the root usage", async () => {
    const graph = config([{ id: "write" }, { id: "catalog:list" }]);
    const error = await invoke(graph, "bogus").catch((e: Error) => e);

    expect(error).toBeInstanceOf(Errors.CLIError);
    expect(error.message).toContain("command bogus not found");
    expect(error.message).toContain("Usage: quoin <command> [options]");
    expect(error.message).toContain("Commands: catalog, write");
  });

  test("exits 2, the code oclif uses for an unresolved command", async () => {
    const error = (await invoke(config([{ id: "write" }]), "bogus").catch(
      (e: Error) => e,
    )) as Errors.CLIError;
    expect(error.oclif.exit).toBe(2);
  });
});
