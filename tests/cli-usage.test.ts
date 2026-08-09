/**
 * Unit coverage for the unknown-command usage helpers added for FR-005-AC-1
 * (SR-003 FND-001). The property test in tests/props/fr-005.prop.test.ts drives
 * these through the real runner; these cases pin the pure edges the runner path
 * cannot reach — a flags-only argv, an alias, a hidden command, and a non-Error
 * rejection.
 */
import { isUnknownCommand, rootUsage, withRootUsage } from "../src/cli";

const config = (
  commands: Array<{ id: string; aliases?: string[]; hidden?: boolean }>,
) => ({ bin: "quoin", commands });

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
});

// Trace: FR-005-AC-1
describe("isUnknownCommand", () => {
  const graph = config([
    { id: "write" },
    { id: "catalog:list" },
    { id: "module:list", aliases: ["plugin:list"] },
  ]);

  test("is false when argv carries no command at all", () => {
    expect(isUnknownCommand([], graph)).toBe(false);
    expect(isUnknownCommand(["--help", "-v"], graph)).toBe(false);
  });

  test("is false for a known command, a known topic, and a known alias", () => {
    expect(isUnknownCommand(["write"], graph)).toBe(false);
    expect(isUnknownCommand(["catalog", "list"], graph)).toBe(false);
    expect(isUnknownCommand(["plugin", "list"], graph)).toBe(false);
  });

  test("is true only for a token no command or alias claims", () => {
    expect(isUnknownCommand(["bogus"], graph)).toBe(true);
    // Flags before the command must not shadow it.
    expect(isUnknownCommand(["--json", "bogus"], graph)).toBe(true);
  });
});

// Trace: FR-005-AC-1
describe("withRootUsage", () => {
  test("appends the usage to an Error, preserving the original message", () => {
    const result = withRootUsage(new Error("command x not found"), "USAGE");
    expect((result as Error).message).toBe("command x not found\n\nUSAGE");
  });

  test("passes a non-Error rejection through untouched", () => {
    const thrown = { not: "an error" };
    expect(withRootUsage(thrown, "USAGE")).toBe(thrown);
  });
});
