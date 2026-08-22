#!/usr/bin/env node
// Assert every surface that reports quoin's version reports the SAME version,
// and — on a clean tag — that it is the tag (agent-ix/quoin#196).
//
// The defect this exists to catch shipped once already, one repo over:
// agent-ix/quire-cli#52, where the 0.24.0–0.28.0 tags all shipped binaries
// reporting 0.23.0. Version provenance is load-bearing here — every SpecReview
// records the tool version it measured with, and three reviews in
// agent-ix/filament-ide-rs cite numbers from a binary whose self-reported
// version was wrong.
//
// Runs against the BUILT binary, not the source: the disagreement was between
// a build-time baked value and package.json, so only a built artifact can show
// it. `make build` first.

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const bin = join(root, "bin", "quoin.js");

if (!existsSync(join(root, "dist", "cli.js"))) {
  console.error("check-version: dist/ is absent — run `make build` first");
  process.exit(2);
}

const runBin = (args) =>
  execFileSync(process.execPath, [bin, ...args], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });

const flag = runBin(["--version"]).trim();

// oclif renders `VERSION\n  <name>/<version> <platform> <node>`.
const help = runBin(["--help"]);
const block = help.match(/^VERSION\n\s+\S+\/(\S+)/m);
if (!block) {
  console.error("check-version: no VERSION block in `--help` output");
  process.exit(1);
}
const helpVersion = block[1];

let failed = false;
const check = (ok, message) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${message}`);
  if (!ok) failed = true;
};

check(
  flag === helpVersion,
  `--version (${flag}) and --help (${helpVersion}) agree`,
);

// A clean tag must report itself. A build ahead of its tag reports the drift
// suffix, which is the point of baking `git describe` and is not a failure.
let describe = null;
try {
  describe = execFileSync("git", ["describe", "--tags", "--dirty"], {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  }).trim();
} catch {
  console.log("skip  no git tag reachable — drift check not applicable");
}

if (describe) {
  const clean = /^v?\d+\.\d+\.\d+$/.test(describe);
  if (clean) {
    check(
      flag === describe.replace(/^v/, ""),
      `a clean tag reports itself (${flag} vs ${describe})`,
    );
  } else {
    console.log(
      `skip  build is ahead of its tag (${describe}) — drift expected`,
    );
  }
}

process.exit(failed ? 1 : 0);
