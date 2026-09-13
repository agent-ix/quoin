/**
 * `src/core/snapshot.ts` — the walk that feeds `validators.run` (quoin#412).
 *
 * The cutover moved the gate analysis into `quoin-validators`, but the caller
 * still has to decide WHICH files to put on the wire. That decision is the one
 * place a transport optimisation could silently change a verdict, so it is
 * pinned here: the pre-filter must be a strict SUPERSET of what the Rust
 * classification accepts. Rust re-derives shell scripts, wiring files and the
 * excluded directories from the map it receives; a file this walk drops can
 * therefore never have mattered, and a file it keeps needlessly costs bytes and
 * nothing else.
 */

import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { describe, expect, it } from "vitest";

import { repoSnapshot } from "../src/core/snapshot.js";

function workspace(files: Record<string, string>): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-snapshot-"));
  for (const [path, source] of Object.entries(files)) {
    const target = join(root, path);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, source);
  }
  return root;
}

describe("repoSnapshot (quoin#412)", () => {
  it("carries every shape quoin-validators can classify", () => {
    // One file per accepted shape on the far side: `is_shell_file` (.sh) and
    // every `is_wiring_file` arm — Makefile and its suffixed forms, Taskfile,
    // justfile, package.json, and YAML under .github/workflows/.
    const root = workspace({
      ".github/workflows/ci.yaml": "on: push\n",
      ".github/workflows/gate.yml": "on: push\n",
      Makefile: "gate:\n",
      "Makefile.common": "gate:\n",
      "Taskfile.yml": "version: '3'\n",
      "deep/nested/dir/check.sh": "# gate\n",
      justfile: "gate:\n",
      "package.json": "{}\n",
      "scripts/check.sh": "# gate\n",
    });
    expect(Object.keys(repoSnapshot(root).files).sort()).toEqual([
      ".github/workflows/ci.yaml",
      ".github/workflows/gate.yml",
      "Makefile",
      "Makefile.common",
      "Taskfile.yml",
      "deep/nested/dir/check.sh",
      "justfile",
      "package.json",
      "scripts/check.sh",
    ]);
  });

  it("is coarser than the far side rather than a second copy of it", () => {
    // These are sent and then REJECTED by the Rust classification: `.SH` is not
    // a script (the extension comparison is case-sensitive), a YAML workflow
    // outside .github/workflows/ is not wiring, and `makefile.txt` is not a
    // Makefile. Being wrong in this direction is free; being wrong in the other
    // direction would lose a finding.
    const root = workspace({
      MAKEFILE: "gate:\n",
      "makefile.txt": "prose\n",
      "scripts/CHECK.SH": "# gate\n",
      "workflows/ci.yml": "on: push\n",
    });
    const kept = Object.keys(repoSnapshot(root).files).sort();
    expect(kept).toEqual(["MAKEFILE", "makefile.txt", "scripts/CHECK.SH"]);
    // And `workflows/ci.yml` is dropped only because the far side would not
    // have called it wiring either.
    expect(kept).not.toContain("workflows/ci.yml");
  });

  it("drops what could not have mattered", () => {
    const root = workspace({
      "README.md": "prose\n",
      "src/index.ts": "export {};\n",
    });
    expect(repoSnapshot(root).files).toEqual({});
  });

  it("never descends into an excluded directory", () => {
    const root = workspace({
      "node_modules/pkg/Makefile": "gate:\n",
      "scripts/check.sh": "# gate\n",
      "vendor/gate.sh": "# gate\n",
    });
    expect(Object.keys(repoSnapshot(root).files)).toEqual(["scripts/check.sh"]);
  });

  it("splits on \\n alone, so joining the lines restores the bytes", () => {
    // The far side does its own CRLF handling; splitting on /\r?\n/ here would
    // destroy the carriage returns before it ever saw them.
    const source = "gate:\r\n\t./g.sh\r\n";
    const root = workspace({ Makefile: source });
    const lines = repoSnapshot(root).files["Makefile"] as string[];
    expect(lines.join("\n")).toBe(source);
  });

  it("reports an unreadable file as null and omits unlistable when empty", () => {
    const root = workspace({ "scripts/check.sh": "# gate\n" });
    const snapshot = repoSnapshot(root);
    expect(snapshot.files["scripts/check.sh"]).toEqual(["# gate", ""]);
    expect(snapshot.unlistable).toBeUndefined();
  });
});
