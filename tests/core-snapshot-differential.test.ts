/**
 * One tree, two repositories, one verdict (quoin#412, review #448 FND-001).
 *
 * `quoin validate` no longer analyses a directory. It walks the tree in
 * `src/core/snapshot.ts`, puts a file map on `quoin-core`'s stdin, and the
 * analysis runs over a `MemoryRepo` built from that map. The whole cutover
 * rests on one property: **the walk's pre-filter is a superset of what the Rust
 * classification accepts.** A path it drops is a path the far side never sees,
 * and a finding `quoin validate` silently stops reporting.
 *
 * Rust re-classifying every path it receives only protects one direction. It
 * catches a filter that is too COARSE — extra files change nothing. It cannot
 * catch a filter that is too NARROW, because a file that never goes on the wire
 * cannot be re-classified.
 *
 * Nothing measured that before this file. `tc_377_019` runs `DiskRepo` over a
 * materialised corpus, `tc_412_the_boundary_reproduces_every_captured_
 * typescript_verdict` runs `MemoryRepo` over the same corpus, and both bypass
 * `repoSnapshot` entirely. `tests/core-exec-e2e.test.ts` asks the binary which
 * file NAMES it classifies, which closes the filename axis and only that one:
 * the far side applies `is_excluded` too, so a directory the caller stops
 * descending into reads as "the classifier said no" there as well.
 *
 * The reviewer of #448 demonstrated what that leaves open by adding one line —
 * `"third_party"` — to `EXCLUDED` in `src/core/snapshot.ts`. A repository with
 * a wired empty gate under `third_party/` went from 1 finding to 0, `quoin
 * validate` exited 0 either way, and the whole suite plus `make rust-gate`
 * stayed green.
 *
 * So this file compares the two repositories directly, over ONE real tree:
 *
 *   A. `repoSnapshot(root)` -> `validators.run` -> `MemoryRepo`  (what ships)
 *   B. `quoin-disk-findings <root>` -> `DiskRepo`                (the oracle)
 *
 * Same analysis, same tree, different `RepoSource`. Any divergence is the
 * pre-filter deciding something, which it is not allowed to do — and it covers
 * both axes at once, because a directory the walk skips and a name it drops
 * reach this assertion the same way.
 *
 * The oracle is a binary and not a second TypeScript walk on purpose: a walk
 * written here would share the bug of the walk under test.
 */

import { execFileSync } from "node:child_process";
import {
  accessSync,
  constants,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { describe, expect, it } from "vitest";

import { runCore } from "../src/core/index.js";
import { repoSnapshot } from "../src/core/snapshot.js";

/** An executable named by an environment variable, or `null`. */
function executable(variable: string): string | null {
  const path = process.env[variable];
  if (!path) return null;
  try {
    accessSync(path, constants.X_OK);
    return path;
  } catch {
    return null;
  }
}

const diskFindings = executable("QUOIN_DISK_FINDINGS");
const available = executable("QUOIN_CORE") !== null && diskFindings !== null;

/** A shell gate that claims a negative obligation and never asserts its count. */
function gate(obligation: string): string {
  return [
    "#!/bin/sh",
    `# Gate for ${obligation}: no production symbol shall call \`unwrap\`.`,
    'grep -rn "unwrap()" src/ | wc -l',
    "",
  ].join("\n");
}

/**
 * The tree both sides analyse.
 *
 * Every entry is here for a reason a narrowing of the pre-filter would break:
 *
 * - `third_party/` is NOT an excluded directory on either side. It is the
 *   reviewer's demonstration, kept as the fixture.
 * - `vendor/` and `node_modules/` ARE excluded on both sides, so the wired
 *   gates inside them must be invisible to both — a differential that only
 *   held files neither side excludes would not notice an exclusion list that
 *   stopped being applied on the far side either.
 * - `Makefile.ci`, `Taskfile.yaml`, `justfile`, `.github/workflows/nested/` and
 *   `makefile.sh` are the wiring shapes; `makefile.sh` is both wiring and a
 *   shell script, which `gates.rs` now allows and no golden case pinned.
 * - The nesting depths differ so that a walk that stopped descending is a
 *   failure rather than a smaller-but-equal answer.
 */
const TREE: Record<string, string> = {
  Makefile: "gate:\n\t./scripts/check_unwrap.sh\n\t./third_party/gate.sh\n",
  "Makefile.ci": "ci:\n\t./tools/ci_gate.sh\n",
  "Taskfile.yaml":
    "version: '3'\ntasks:\n  gate:\n    cmds:\n      - deep/nested/verify.sh\n",
  justfile: "audit:\n\t./ops/audit.sh\n",
  "makefile.sh": "#!/bin/sh\n./ops/wired_by_a_script.sh\n",
  ".github/workflows/nested/ci.yaml":
    "jobs:\n  gate:\n    steps:\n      - run: ./ci/workflow_gate.sh\n",
  "scripts/check_unwrap.sh": gate("FR-001-AC-1"),
  "third_party/gate.sh": gate("FR-002-AC-1"),
  "tools/ci_gate.sh": gate("FR-003-AC-1"),
  "deep/nested/verify.sh": gate("FR-004-AC-1"),
  "ops/audit.sh": gate("FR-005-AC-1"),
  "ops/wired_by_a_script.sh": gate("FR-006-AC-1"),
  "ci/workflow_gate.sh": gate("FR-007-AC-1"),
  // Excluded on both sides: wired, defective, and invisible to both answers.
  "vendor/Makefile": "gate:\n\t./vendor/gate.sh\n",
  "vendor/gate.sh": gate("FR-900-AC-1"),
  "node_modules/pkg/Makefile": "gate:\n\t./node_modules/pkg/gate.sh\n",
  "node_modules/pkg/gate.sh": gate("FR-901-AC-1"),
  // Noise neither side classifies.
  "README.md": "# fixture\n",
  "package-lock.json": "{}\n",
  "src/index.ts": "export {};\n",
};

function materialise(): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-differential-"));
  for (const [path, source] of Object.entries(TREE)) {
    const target = join(root, path);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, source);
  }
  return root;
}

interface Finding {
  path: string;
  wiredBy: string;
}

describe.skipIf(!available)(
  "repoSnapshot -> MemoryRepo == DiskRepo (quoin#412)",
  () => {
    // Trace: FR-096-AC-8
    it("reaches the same verdict over one tree by both repositories", () => {
      const root = materialise();
      try {
        const snapshot = repoSnapshot(root);
        const throughTheBoundary = runCore("validators.run", snapshot) as {
          findings: Finding[];
        };
        const overTheDisk = JSON.parse(
          execFileSync(diskFindings as string, [root], { encoding: "utf8" }),
        ) as { findings: Finding[] };

        // Population floors. Every assertion below is an equality between two
        // answers, and two empty answers are equal: a probe that stopped
        // reaching either binary, or a fixture that stopped materialising,
        // would agree perfectly about nothing. These say what the comparison
        // has to be measuring for its agreement to mean anything.
        expect(Object.keys(snapshot.files).length).toBeGreaterThanOrEqual(12);
        expect(overTheDisk.findings.length).toBeGreaterThanOrEqual(7);
        expect(
          new Set(overTheDisk.findings.map((f) => f.wiredBy)).size,
        ).toBeGreaterThanOrEqual(4);

        expect(throughTheBoundary.findings).toEqual(overTheDisk.findings);

        // Named explicitly, because it is the one the reviewer broke: a
        // directory nobody excludes is walked by both, and the equality above
        // would also hold if BOTH sides had stopped seeing it.
        expect(overTheDisk.findings.map((f) => f.path)).toContain(
          "third_party/gate.sh",
        );
        // And its opposite: an excluded directory is invisible to both, so the
        // equality is not being satisfied by both sides over-reporting either.
        expect(
          overTheDisk.findings.filter(
            (f) =>
              f.path.startsWith("vendor/") ||
              f.path.startsWith("node_modules/"),
          ),
        ).toEqual([]);
      } finally {
        rmSync(root, { force: true, recursive: true });
      }
    });
  },
);
