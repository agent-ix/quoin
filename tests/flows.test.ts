import { chmodSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { specFlowNames, startSpecFlow } from "../src/flows";

function tmp(prefix: string): string {
  return mkdtempSync(join(tmpdir(), `quoin-${prefix}-`));
}

// Lay out a packaged-style workflow skill:
//   <root>/<packageSkill>/workflow-assets/skills/<runtimeSkill>
function packagedRoot(flow: "review" | "matrix" | "to-plan"): string {
  const packageSkill = {
    review: "spec-review",
    matrix: "spec-matrix",
    "to-plan": "spec-to-plan",
  }[flow];
  const root = tmp("wf-packaged");
  mkdirSync(join(root, packageSkill, "workflow-assets", "skills", flow), {
    recursive: true,
  });
  return root;
}

// Create a fake `ix-flow` executable that exits with the given code, return its dir.
function fakeIxFlowDir(exitCode: number): string {
  const dir = tmp("bin");
  const bin = join(dir, "ix-flow");
  writeFileSync(bin, `#!/bin/sh\nexit ${exitCode}\n`);
  chmodSync(bin, 0o755);
  return dir;
}

describe("specFlowNames", () => {
  // Trace: FR-020-AC-1
  test("lists the bundled spec flows", () => {
    expect(specFlowNames()).toEqual(["review", "matrix", "to-plan"]);
  });
});

describe("startSpecFlow", () => {
  const savedPath = process.env.PATH;
  const savedRoot = process.env.IX_SPEC_WORKFLOWS_ROOT;
  const savedHome = process.env.IX_HOME;
  const savedExitCode = process.exitCode;

  afterEach(() => {
    process.env.PATH = savedPath;
    if (savedRoot === undefined) delete process.env.IX_SPEC_WORKFLOWS_ROOT;
    else process.env.IX_SPEC_WORKFLOWS_ROOT = savedRoot;
    if (savedHome === undefined) delete process.env.IX_HOME;
    else process.env.IX_HOME = savedHome;
    process.exitCode = savedExitCode;
  });

  // Trace: FR-021-AC-1
  // Trace: FR-023-AC-3
  test("resolves when ix-flow exits 0; builds id/json/target args", async () => {
    process.env.IX_SPEC_WORKFLOWS_ROOT = packagedRoot("review");
    process.env.IX_HOME = tmp("home");
    process.env.PATH = `${fakeIxFlowDir(0)}:${savedPath}`;
    process.exitCode = undefined;
    await expect(
      startSpecFlow("review", {
        json: true,
        id: "run-123",
        target: ["spec/", "spec/two/"],
      }),
    ).resolves.toBeUndefined();
    expect(process.exitCode).toBeUndefined();
  });

  test("resolves with no optional opts (default empty target)", async () => {
    process.env.IX_SPEC_WORKFLOWS_ROOT = packagedRoot("matrix");
    process.env.IX_HOME = tmp("home");
    process.env.PATH = `${fakeIxFlowDir(0)}:${savedPath}`;
    process.exitCode = undefined;
    await expect(startSpecFlow("matrix")).resolves.toBeUndefined();
    expect(process.exitCode).toBeUndefined();
  });

  // Trace: FR-021-AC-2
  test("sets process.exitCode when ix-flow exits non-zero", async () => {
    process.env.IX_SPEC_WORKFLOWS_ROOT = packagedRoot("to-plan");
    process.env.IX_HOME = tmp("home");
    process.env.PATH = `${fakeIxFlowDir(3)}:${savedPath}`;
    process.exitCode = undefined;
    await startSpecFlow("to-plan");
    expect(process.exitCode).toBe(3);
  });

  // Trace: FR-021-AC-3
  test("defaults exit code to 1 when ix-flow is killed by a signal (null code)", async () => {
    // A child terminated by a signal reports close code null -> `code ?? 1`.
    const dir = tmp("bin-signal");
    const bin = join(dir, "ix-flow");
    writeFileSync(bin, "#!/bin/sh\nkill -TERM $$\n");
    chmodSync(bin, 0o755);
    process.env.IX_SPEC_WORKFLOWS_ROOT = packagedRoot("review");
    process.env.IX_HOME = tmp("home");
    process.env.PATH = `${dir}:${savedPath}`;
    process.exitCode = undefined;
    await startSpecFlow("review");
    expect(process.exitCode).toBe(1);
  });

  // Trace: FR-021-AC-4
  test("rejects when ix-flow cannot be spawned (PATH has no ix-flow)", async () => {
    process.env.IX_SPEC_WORKFLOWS_ROOT = packagedRoot("review");
    process.env.IX_HOME = tmp("home");
    process.env.PATH = tmp("empty-bin"); // no ix-flow anywhere
    await expect(startSpecFlow("review")).rejects.toThrow();
  });

  // Trace: FR-020-AC-2
  test("throws for an unknown flow name", async () => {
    await expect(startSpecFlow("nope")).rejects.toThrow(
      "unknown spec flow nope",
    );
  });
});
