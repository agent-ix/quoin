import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  inspectMockInjections,
  mockInspectionInput,
  writeMockInspection,
} from "../src/evidence/index.js";

function workspace(): string {
  return mkdtempSync(join(tmpdir(), "quoin-mock-inspection-"));
}

function write(root: string, path: string, source: string): void {
  const target = join(root, path);
  mkdirSync(join(target, ".."), { recursive: true });
  writeFileSync(target, source);
}

describe("mock injection inspection producer (agent-ix/quoin#204)", () => {
  it("TC-1062 locates the same explicit stand-in shape in Rust, Python and TypeScript", () => {
    const root = workspace();
    write(
      root,
      "rust/src/lib.rs",
      `#[cfg(test)] mod tests {\n#[test]\nfn confirms() {\n  assert!(grant_root(Confirmation::allow()));\n}\n}`,
    );
    write(
      root,
      "python/test_shell.py",
      `class Confirmation:\n    pass\n\ndef test_confirms():\n    assert grant_root(Confirmation.allow())\n`,
    );
    write(
      root,
      "typescript/shell.test.ts",
      `test("confirms", () => { expect(grantRoot(Confirmation.allow())).toBe(true); });\n`,
    );

    const found = inspectMockInjections(root, "SUITE-001");
    expect(found).toHaveLength(3);
    expect(found.map((item) => item.injects[0])).toEqual([
      "Confirmation.allow",
      "Confirmation::allow",
      "Confirmation.allow",
    ]);
    expect(found.map((item) => `${item.path}:${item.line}`)).toEqual([
      "python/test_shell.py:5",
      "rust/src/lib.rs:4",
      "typescript/shell.test.ts:1",
    ]);
  });

  it("TC-1063 ignores production calls and ordinary constructors but retains unrelated mocks for auditor adjudication", () => {
    const root = workspace();
    write(
      root,
      "src/lib.rs",
      `fn production() { let _ = Confirmation::allow(); }\n#[test]\nfn real() { assert!(grant_root(Confirmation::new())); }\nfn production_after_test() { let _ = Confirmation::allow(); }\n#[test]\nfn clocked() { let _ = FakeClock::new(); }\n`,
    );
    const found = inspectMockInjections(root, "SUITE-001");
    expect(found).toEqual([
      expect.objectContaining({
        symbol: "clocked",
        injects: ["FakeClock::new"],
      }),
    ]);
  });

  it("TC-1064 records a completed empty inspection and reads only the exact commit", () => {
    const root = workspace();
    const commit = "a".repeat(40);
    const path = writeMockInspection(root, {
      schemaVersion: 1,
      suite: "SUITE-001",
      commit,
      tool: "quoin mock-inspection",
      timestamp: "2026-08-26T00:00:00Z",
      injections: [],
    });
    expect(JSON.parse(readFileSync(path, "utf8"))).toMatchObject({
      suite: "SUITE-001",
      injections: [],
    });
    expect(mockInspectionInput(root, commit)).toEqual({
      suites: ["SUITE-001"],
      injections: [],
    });
    expect(mockInspectionInput(root, "b".repeat(40))).toEqual({
      suites: [],
      injections: [],
    });
  });
});
