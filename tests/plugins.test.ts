import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { stringify as stringifyYaml } from "yaml";

import {
  installPlugin,
  listPlugins,
  parseSourceArg,
  readModuleName,
  registryPath,
  removePlugin,
} from "../src/plugins";
import { filamentModulesDir } from "../src/catalog";

function tmp(prefix: string): string {
  return mkdtempSync(join(tmpdir(), `quoin-${prefix}-`));
}

function writeManifest(dir: string, name: unknown): void {
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "manifest.yaml"), stringifyYaml({ name }));
}

describe("parseSourceArg", () => {
  test("path: prefix", () => {
    expect(parseSourceArg("path:/a/b")).toEqual({ type: "path", path: "/a/b" });
  });

  test("github: with @ref", () => {
    expect(parseSourceArg("github:agent-ix/x@v1")).toEqual({
      type: "github",
      repo: "agent-ix/x",
      ref: "v1",
    });
  });

  test("github: without ref leaves ref undefined", () => {
    expect(parseSourceArg("github:agent-ix/x")).toEqual({
      type: "github",
      repo: "agent-ix/x",
      ref: undefined,
    });
  });

  test("github: with //subdir maps to a git-subdir source (with ref)", () => {
    expect(parseSourceArg("github:agent-ix/repo//pkg@v1.0.0")).toEqual({
      type: "git-subdir",
      url: "agent-ix/repo",
      path: "pkg",
      ref: "v1.0.0",
    });
  });

  test("github: with //subdir and no ref leaves ref undefined", () => {
    expect(parseSourceArg("github:agent-ix/repo//pkg")).toEqual({
      type: "git-subdir",
      url: "agent-ix/repo",
      path: "pkg",
      ref: undefined,
    });
  });

  test("package: npm with @version", () => {
    expect(parseSourceArg("package:foo@1.2.3")).toEqual({
      type: "npm",
      package: "foo",
      version: "1.2.3",
    });
  });

  test("package: npm without version", () => {
    expect(parseSourceArg("package:foo")).toEqual({
      type: "npm",
      package: "foo",
    });
  });

  test("scoped package: npm keeps the scope @, splits on the last @", () => {
    expect(parseSourceArg("package:@scope/foo@2.0.0")).toEqual({
      type: "npm",
      package: "@scope/foo",
      version: "2.0.0",
    });
  });

  test("bare argument falls back to a path source", () => {
    expect(parseSourceArg("./bare")).toEqual({ type: "path", path: "./bare" });
  });
});

describe("registryPath", () => {
  test("derives registry.json under the home filament dir", () => {
    expect(registryPath("/h")).toBe("/h/filament/registry.json");
  });
});

describe("readModuleName", () => {
  // Trace: FR-019-AC-2
  test("reads the name from a top-level manifest.yaml", () => {
    const dir = tmp("mod-top");
    writeManifest(dir, "spec-objects-business");
    expect(readModuleName(dir)).toBe("spec-objects-business");
  });

  // Trace: FR-019-AC-2
  test("reads the name from a nested <root>/<basename>/manifest.yaml", () => {
    const parent = tmp("mod-parent");
    const base = "spec-objects-nested";
    const root = join(parent, base);
    writeManifest(join(root, base), "spec-objects-nested");
    expect(readModuleName(root)).toBe("spec-objects-nested");
  });

  test("throws when no manifest.yaml exists", () => {
    const dir = tmp("mod-none");
    expect(() => readModuleName(dir)).toThrow("no manifest.yaml found under");
  });

  // Trace: FR-019-AC-2
  test("throws when the manifest has no usable name", () => {
    const dir = tmp("mod-noname");
    writeManifest(dir, 123); // non-string name
    expect(() => readModuleName(dir)).toThrow(/has no name/);
  });

  // Trace: FR-019-AC-2
  test("throws when the manifest name is an empty string", () => {
    const dir = tmp("mod-emptyname");
    writeManifest(dir, "");
    expect(() => readModuleName(dir)).toThrow(/has no name/);
  });
});

describe("installPlugin / listPlugins / removePlugin", () => {
  test("installs, lists, then removes a plugin and its target dir + registry entry", () => {
    const home = tmp("plugin-home");
    const src = tmp("plugin-src");
    writeManifest(join(src, "spec-objects-business"), "spec-objects-business");
    const rec = installPlugin(
      `path:${join(src, "spec-objects-business")}`,
      home,
    );
    expect(rec.name).toBe("spec-objects-business");
    expect(listPlugins(home).map((p) => p.name)).toContain(
      "spec-objects-business",
    );
    expect(
      existsSync(
        join(
          filamentModulesDir(home),
          "spec-objects-business",
          "manifest.yaml",
        ),
      ),
    ).toBe(true);
    expect(existsSync(registryPath(home))).toBe(true);

    removePlugin("spec-objects-business", home);
    expect(listPlugins(home)).toHaveLength(0);
    expect(
      existsSync(join(filamentModulesDir(home), "spec-objects-business")),
    ).toBe(false);
  });
});
