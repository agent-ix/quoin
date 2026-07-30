import { execFileSync } from "node:child_process";

import { main, packageVersion } from "../src/cli";
import CatalogList from "../src/commands/catalog/list";
import Write from "../src/commands/write";

test("build-tools help exits successfully", () => {
  const output = execFileSync("node", ["scripts/build-tools.js", "--help"], {
    encoding: "utf8",
  });
  expect(output).toContain("Build Tools");
});

test("catalog help explains the installed module dir", () => {
  // Help text is now oclif-generated from the command metadata; assert the
  // canonical description still documents the installed module directory.
  const help = `${CatalogList.summary}\n${CatalogList.description}`;
  expect(help).toContain("~/.ix/filament/modules");
  expect(help).toContain("quoin module install");
});

test("write help explains authoring packs", () => {
  const help = `${Write.summary}\n${Write.description}`;
  expect(help).toContain("authoring pack");
  expect(Write.examples.join("\n")).toContain("quoin write");
});

test("version flag prints package version", async () => {
  const output: string[] = [];
  const spy = vi.spyOn(console, "log").mockImplementation((message) => {
    output.push(String(message));
  });
  try {
    await main(["--version"]);
    await main(["version"]);
  } finally {
    spy.mockRestore();
  }
  expect(output).toEqual([packageVersion(), packageVersion()]);
});
