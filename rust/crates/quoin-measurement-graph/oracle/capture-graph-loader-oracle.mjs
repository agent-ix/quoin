// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The TypeScript half of the W10 governed-graph-portfolio *loader* gate
// (quoin#477, FR-067, FR-100-AC-4).
//
//   cd <repo root>
//   node rust/crates/quoin-measurement-graph/oracle/capture-graph-loader-oracle.mjs \
//     --out rust/crates/quoin-measurement-graph/tests/goldens/graph-loader-oracle.json
//
// Runs `buildGovernedGraphPortfolio` end to end over the committed fixture
// tree under `tests/fixtures/graph-loader-tree/` and writes what it produced.
// Nothing is asserted here; `tc_477_graph_loader.rs` compares.
//
// # Two case lists, and why
//
// `byteIdentical` holds the repositories whose whole report — canonical JSON
// and rendered text — is compared byte for byte. `verdicts` holds the two
// repositories whose graph refusal carries prose neither side can make the
// other's: an absent file is `ENOENT: no such file or directory, open '...'`
// under node and `No such file or directory (os error 2)` under Rust, and an
// export that is not an assurance export is refused in zod's sentences here
// and in this workspace's sentences there (quoin#403). For those the capture
// records the availability and the path only, and `DIVERGENCE.md` §9 states
// exactly that. Splitting the two lists is what keeps the first list an
// honest byte gate instead of a gate with an exception inside it.
//
// The output is committed and frozen. FR-101-AC-5 forbids a live TypeScript
// runtime at test time and `tc_476_boundary.rs` asserts none can be spawned;
// FR-101-AC-11 is why the capture records the producing module, its digest,
// the quoin revision and the node that ran it. This script is deleted by the
// W12 cutover commit, along with the TypeScript it captures.
//
// # The one normalisation
//
// Every root, mapping and collection path is absolute, so the bytes would
// otherwise record whoever's checkout produced them. Every occurrence of the
// fixture tree's own absolute root is replaced by the token below, on this
// side and on the Rust side, before anything is compared.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const crateRoot = resolve(join(here, ".."));
const repoRoot = resolve(
  process.env.QUOIN_SRC_ROOT ?? join(here, "..", "..", "..", ".."),
);
const load = (path) => import(pathToFileURL(join(repoRoot, "src", path)).href);

const { buildGovernedGraphPortfolio } = await load(
  "measurement/graph-portfolio-load.js",
);
const { canonicalGraphPortfolioJson, renderGovernedGraphPortfolio } =
  await load("measurement/graph-portfolio.js");

const argv = process.argv.slice(2);
let out = null;
for (let index = 0; index < argv.length; index += 1) {
  if (argv[index] === "--out") {
    out = argv[index + 1];
    index += 1;
  }
}
if (out === null) throw new Error("usage: --out <file>");

const TREE_TOKEN = "@@TREE@@";
const tree = resolve(join(crateRoot, "tests", "fixtures", "graph-loader-tree"));
const normalise = (text) => text.split(tree).join(TREE_TOKEN);
const repo = (name) => join(tree, name);
const graph = (name, file) => join(tree, name, "graph", file);

// The repositories that name graph documents at all. `bravo` and `echo` name
// none, which is the mapping refusal `loadStructuralGraph` answers without
// opening a file.
const MAPPED = new Set(["alpha", "charlie", "delta"]);

const mappingOptions = (names) => {
  const mapped = names.filter((name) => MAPPED.has(name));
  return {
    graphExports: mapped.map(
      (name) => `${repo(name)}=${graph(name, "export.json")}`,
    ),
    graphPremises: mapped.map(
      (name) => `${repo(name)}=${graph(name, "premises.json")}`,
    ),
    graphAudits: mapped.map(
      (name) => `${repo(name)}=${graph(name, "audit.json")}`,
    ),
    changed: names.includes("alpha")
      ? [`${repo("alpha")}=FR-001`, `${repo("alpha")}=FR-002`]
      : [],
    cwd: tree,
  };
};

const runOver = (names) => {
  const options = mappingOptions(names);
  const report = buildGovernedGraphPortfolio(names.map(repo), options);
  return { options, report };
};

// ------------------------------------------------- the byte-identical case
//
// `alpha` loads a whole graph and carries two changed seeds; `bravo` is named
// with no mapping at all, so its graph is the mapping's own refusal; `echo`
// carries one collection whose timestamp no instant grammar reads, which is
// the one refusal this loader synthesises itself.

const byteIdenticalNames = ["alpha", "bravo", "echo"];
const byteIdentical = (() => {
  const { options, report } = runOver(byteIdenticalNames);
  return {
    repositories: byteIdenticalNames,
    options: JSON.parse(normalise(JSON.stringify(options))),
    json: normalise(canonicalGraphPortfolioJson(report)),
    rendered: normalise(renderGovernedGraphPortfolio(report)),
  };
})();

// -------------------------------------------------------- the verdict case
//
// `charlie` names an export that is not there and `delta` names one that is
// not an assurance export. Availability and path are compared; the refusal
// sentence is not, and `DIVERGENCE.md` §9 says why.

const verdictNames = ["charlie", "delta"];
const verdicts = (() => {
  const { options, report } = runOver(verdictNames);
  return {
    repositories: verdictNames,
    options: JSON.parse(normalise(JSON.stringify(options))),
    graphs: report.repositories.map((entry) => ({
      root: normalise(entry.root),
      availability: entry.graph.availability,
      path: entry.graph.path === undefined ? null : normalise(entry.graph.path),
    })),
  };
})();

// --------------------------------------------------------------- the capture

const revision = execFileSync("git", ["rev-parse", "HEAD"], {
  cwd: repoRoot,
  encoding: "utf8",
}).trim();
const digestOf = (path) =>
  `sha256:${createHash("sha256")
    .update(readFileSync(join(repoRoot, path)))
    .digest("hex")}`;

const capture = {
  produced_by: "src/measurement/graph-portfolio-load.ts",
  produced_by_digest: digestOf("src/measurement/graph-portfolio-load.ts"),
  produced_from_revision: revision,
  produced_by_node: process.version,
  capture_script:
    "rust/crates/quoin-measurement-graph/oracle/capture-graph-loader-oracle.mjs",
  fixture_tree:
    "rust/crates/quoin-measurement-graph/tests/fixtures/graph-loader-tree",
  tree_token: TREE_TOKEN,
  byte_identical: byteIdentical,
  verdicts,
};

mkdirSync(dirname(resolve(out)), { recursive: true });
writeFileSync(resolve(out), `${JSON.stringify(capture, null, 2)}\n`);
console.log(
  `${byteIdentical.repositories.length} byte-identical repositories, ` +
    `${verdicts.graphs.length} graph verdicts -> ${resolve(out)}`,
);
