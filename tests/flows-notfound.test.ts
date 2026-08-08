import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// Isolate the "no workflow skill found anywhere" path: force existsSync to
// report false for every candidate so resolveSkillPath exhausts all roots and
// throws. This module mocks node:fs only for this file.
vi.mock("node:fs", async () => {
  const actual = await vi.importActual<typeof import("node:fs")>("node:fs");
  return { ...actual, existsSync: () => false };
});

import { startSpecFlow } from "../src/flows";

describe("resolveSkillPath not-found", () => {
  const savedRoot = process.env.IX_SPEC_WORKFLOWS_ROOT;
  const savedHome = process.env.IX_HOME;

  afterEach(() => {
    if (savedRoot === undefined) delete process.env.IX_SPEC_WORKFLOWS_ROOT;
    else process.env.IX_SPEC_WORKFLOWS_ROOT = savedRoot;
    if (savedHome === undefined) delete process.env.IX_HOME;
    else process.env.IX_HOME = savedHome;
  });

  // Trace: FR-020-AC-3
  test("throws when no candidate root contains the skill", async () => {
    process.env.IX_SPEC_WORKFLOWS_ROOT = mkdtempSync(
      join(tmpdir(), "quoin-wf-none-"),
    );
    process.env.IX_HOME = mkdtempSync(join(tmpdir(), "quoin-home-"));
    await expect(startSpecFlow("review")).rejects.toThrow(
      /could not find ix-spec workflow skill review; set IX_SPEC_WORKFLOWS_ROOT/,
    );
  });
});
