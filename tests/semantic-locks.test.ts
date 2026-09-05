import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import {
  RECOGNISED_MAJORS,
  lockCatalog,
  lockModule,
  type ModuleInput,
} from "../src/semantic/locks.js";

function repo(name: string, dirty = false): string {
  const dir = join(mkdtempSync(join(tmpdir(), "lock-")), name);
  mkdirSync(dir, { recursive: true });
  const run = (...args: string[]) =>
    execFileSync("git", args, { cwd: dir, stdio: "ignore" });
  run("init", "-q");
  run("config", "user.email", "t@x");
  run("config", "user.name", "t");
  writeFileSync(join(dir, "manifest.yaml"), "name: fixture\n");
  run("add", "-A");
  run("commit", "-qm", "fixture");
  if (dirty) writeFileSync(join(dir, "manifest.yaml"), "name: edited\n");
  return dir;
}

const semantic = (over: Record<string, unknown> = {}) => ({
  contract_version: "1.0.0",
  semantic_core: "0.1.0",
  targets: ["json-schema", "rust"],
  packages: {
    "json-schema": { coordinates: "pkg:json-schema", fingerprint: "sha256:aa" },
  },
  ...over,
});

describe("TC-1600..1614 semantic catalog locks (FR-101)", () => {
  // TC-1600 — FR-101-AC-1
  it("identifies the source by commit, and carries a tag only alongside it", () => {
    const { record } = lockModule({
      name: "m",
      root: repo("m"),
      semantic: semantic(),
    });
    expect(record.commit).toMatch(/^[0-9a-f]{40}$/);
    // An untagged commit reports no tag rather than the nearest one: a
    // nearest-tag answer reads as a pin and behaves as a guess.
    expect(record.tag).toBeNull();
  });

  // TC-1601 — FR-101-AC-2
  it("produces byte-identical records for an unchanged environment", () => {
    const root = repo("m");
    const input: ModuleInput = { name: "m", root, semantic: semantic() };
    const a = lockModule(input);
    const b = lockModule(input);
    expect(JSON.stringify(a.record)).toBe(JSON.stringify(b.record));
  });

  // TC-1602 — FR-101-AC-2, order independence
  it("orders the catalog by module name, not by discovery", () => {
    const modules: ModuleInput[] = [
      { name: "zeta", root: repo("zeta"), semantic: semantic() },
      { name: "alpha", root: repo("alpha"), semantic: semantic() },
    ];
    const forwards = lockCatalog(modules);
    const backwards = lockCatalog([...modules].reverse());
    expect(forwards.records.map((r) => r.module)).toEqual(["alpha", "zeta"]);
    expect(JSON.stringify(forwards.records)).toBe(
      JSON.stringify(backwards.records),
    );
  });

  // TC-1603 — FR-101-AC-3
  it("records a dirty tree without dropping the module", () => {
    const { record } = lockModule({
      name: "m",
      root: repo("m", true),
      semantic: semantic(),
    });
    expect(record.clean).toBe(false);
    expect(record.module).toBe("m");
  });

  // TC-1604 — FR-101-AC-4
  it("records a target with no package as missing, naming its owner", () => {
    const { record, diagnostics } = lockModule(
      { name: "m", root: repo("m"), semantic: semantic() },
      { owners: { rust: "agent-ix/filament-core-data#80" } },
    );
    const rust = record.targets.find((t) => t.target === "rust");
    expect(rust?.state).toBe("missing");
    expect(rust?.owningIssue).toBe("agent-ix/filament-core-data#80");
    // Present in the list, not omitted: an omitted target is indistinguishable
    // from one that was never declared.
    expect(record.targets.map((t) => t.target)).toEqual([
      "json-schema",
      "rust",
    ]);
    expect(diagnostics.map((d) => d.code)).toContain(
      "missing-generated-target",
    );
  });

  // TC-1605 — FR-101-AC-5, the falsification of "no silent fallback".
  it("reports a mismatched fingerprint and does not substitute it", () => {
    const { record, diagnostics } = lockModule(
      { name: "m", root: repo("m"), semantic: semantic() },
      { expected: { "m/json-schema": "sha256:expected" } },
    );
    const target = record.targets.find((t) => t.target === "json-schema");
    expect(target?.state).toBe("incompatible");
    expect(target?.detail).toContain("sha256:expected");
    expect(diagnostics.map((d) => d.code)).toContain("incompatible-lock");

    // And the matching case still resolves, or the check would be refusing
    // everything rather than refusing a mismatch.
    const matched = lockModule(
      { name: "m", root: repo("m"), semantic: semantic() },
      { expected: { "m/json-schema": "sha256:aa" } },
    );
    expect(
      matched.record.targets.find((t) => t.target === "json-schema")?.state,
    ).toBe("present");
  });

  // TC-1606 — FR-101-AC-6
  it("reports an unrecognised major without falling back to a recognised one", () => {
    const { record, diagnostics } = lockModule({
      name: "m",
      root: repo("m"),
      semantic: semantic({ contract_version: "7.0.0" }),
    });
    expect(record.targets.every((t) => t.state === "unknown-major")).toBe(true);
    expect(diagnostics.map((d) => d.code)).toContain("unknown-schema-major");
    expect(RECOGNISED_MAJORS).not.toContain("7");
  });

  // TC-1607 — FR-101-AC-7
  it("resolves a module with no semantic block as dynamic-only", () => {
    const { record, diagnostics } = lockModule({ name: "m", root: repo("m") });
    expect(record.resolution).toBe("dynamic-only");
    expect(record.artifactDigest).toBeNull();
    // Still in the catalog, and no diagnostic: a module without a semantic
    // block is not a defect, it is the transition state the ticket requires
    // to keep working.
    expect(record.module).toBe("m");
    expect(diagnostics.filter((d) => d.code === "incompatible-lock")).toEqual(
      [],
    );
  });

  // TC-1608 — FR-101-AC-8
  it("carries the semantic-core version the module declares", () => {
    const { record } = lockModule({
      name: "m",
      root: repo("m"),
      semantic: semantic({ semantic_core: "0.2.0" }),
    });
    expect(record.semanticCore).toBe("0.2.0");
  });

  // TC-1609 — FR-101-AC-9
  it("changes no byte of the module repository", () => {
    const root = repo("m");
    const before = execFileSync("git", ["status", "--porcelain"], {
      cwd: root,
      encoding: "utf8",
    });
    lockModule({ name: "m", root, semantic: semantic() });
    const after = execFileSync("git", ["status", "--porcelain"], {
      cwd: root,
      encoding: "utf8",
    });
    expect(after).toBe(before);
  });

  // TC-1610 — FR-101-AC-10
  it("performs no network access", () => {
    // Static: the module imports only `node:child_process` and `node:crypto`,
    // and every git invocation is local. Asserted by inspection of the import
    // list rather than by observing a socket, because an absent socket in one
    // run is not evidence that none is opened.
    const source = execFileSync("cat", ["src/semantic/locks.ts"], {
      encoding: "utf8",
    });
    expect(source).not.toContain("node:http");
    expect(source).not.toContain("node:https");
    expect(source).not.toContain("fetch(");
    expect(source).toContain(
      'import { execFileSync } from "node:child_process"',
    );
  });
});
