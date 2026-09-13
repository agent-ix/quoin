// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The TypeScript half of the W9 governed-graph-portfolio gate
// (quoin#476, FR-067, FR-100-AC-4).
//
//   cd <repo root>
//   node --loader ts-node/esm \
//     rust/crates/quoin-measurement-graph/oracle/capture-graph-portfolio-oracle.mjs \
//     --out rust/crates/quoin-measurement-graph/tests/goldens/graph-portfolio-oracle.json
//
// Runs `parseGraphPortfolioMappings`, `buildGovernedGraphPortfolioFrom`,
// `compareGraphQualityCollections`, `canonicalGraphPortfolioJson` and
// `renderGovernedGraphPortfolio` over the committed fixture tree under
// `tests/fixtures/graph-portfolio-tree/` and writes what THEY produce. Nothing
// is asserted here; `tc_476_graph_portfolio.rs` compares.
//
// # Why this script does not call `graph-portfolio-load.ts`
//
// The loader reaches into `graph-analysis/` (FR-062), which is a different
// wave's port. `graph-portfolio.ts` is pure and treats the structural graph as
// opaque, so the graphs — and the collection refusals a loader would
// synthesise — are stated as data in `graph-portfolio-tree/graph-inputs.json`
// and read identically by this script and by the Rust test. Nothing about the
// projection under test is supplied by the loader.
//
// The output is committed and frozen. FR-101-AC-5 forbids a live TypeScript
// runtime at test time and `tc_476_boundary.rs` would catch one; FR-101-AC-11
// is why the capture records the producing modules, the quoin revision and the
// node that ran it. This script is deleted by the W12 cutover commit, along
// with the TypeScript it captures.
//
// # The one normalisation
//
// `resolve()` makes every root, mapping and collection path absolute, so the
// bytes would otherwise record whoever's checkout produced them. Every
// occurrence of the fixture tree's own absolute root is replaced by the token
// below, on this side and on the Rust side, before anything is compared. It is
// the only edit made to what the TypeScript produced.

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

const { canonicalJson } = await load("store/canonical.js");
const { readMeasurementCollectionResults } = await load("measurement/store.js");
const { loadMeasurementPlans } = await load("measurement/plans.js");
const { buildPortfolioReportFromCollections } = await load(
  "measurement/portfolio.js",
);
const {
  buildGovernedGraphPortfolioFrom,
  canonicalGraphPortfolioJson,
  compareGraphQualityCollections,
  parseGraphPortfolioMappings,
  renderGovernedGraphPortfolio,
} = await load("measurement/graph-portfolio.js");

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
const tree = resolve(join(crateRoot, "tests", "fixtures", "graph-portfolio-tree"));
const normalise = (text) => text.split(tree).join(TREE_TOKEN);
const repo = (name) => join(tree, name);

const injected = JSON.parse(
  readFileSync(join(tree, "graph-inputs.json"), "utf8"),
);

// ------------------------------------------------------------ the mappings
//
// Every branch `parseGraphPortfolioMappings` has: a repository with all three
// mappings and two changed seeds (`ready`), one with none (`missing`), one
// with a single mapping (`incompatible`), the de-duplicating spelling of a
// root that is already named, and each of the four refusal codes.

const mappingCases = [
  {
    name: "ready_missing_and_incompatible",
    locations: [
      repo("foxtrot"),
      repo("echo"),
      repo("alpha"),
      join(tree, "alpha", "spec", "..", "..", "alpha"),
      repo("bravo"),
      repo("charlie"),
      repo("delta"),
    ],
    options: {
      graphExports: [
        `${repo("alpha")}=${join(tree, "alpha", "graph", "export.json")}`,
        `${repo("foxtrot")}=${join(tree, "foxtrot", "graph", "export.json")}`,
        `${repo("echo")}=${join(tree, "echo", "graph", "export.json")}`,
      ],
      graphPremises: [
        `${repo("alpha")}=${join(tree, "alpha", "graph", "premises.json")}`,
        `${repo("foxtrot")}=${join(tree, "foxtrot", "graph", "premises.json")}`,
      ],
      graphAudits: [
        `${repo("alpha")}=${join(tree, "alpha", "graph", "audit.json")}`,
        `${repo("foxtrot")}=${join(tree, "foxtrot", "graph", "audit.json")}`,
      ],
      changed: [
        `${repo("alpha")}=FR-067`,
        `${repo("alpha")}=FR-066`,
        `${repo("alpha")}=FR-067`,
        `${repo("foxtrot")}=FR-100`,
      ],
      cwd: tree,
    },
  },
  {
    name: "relative_spellings",
    locations: ["alpha", "./alpha", "bravo"],
    options: {
      graphExports: ["alpha=alpha/graph/export.json"],
      graphPremises: [" alpha = alpha/graph/premises.json "],
      graphAudits: ["alpha=alpha/graph/audit.json"],
      cwd: tree,
    },
  },
  {
    name: "no_mappings_at_all",
    locations: [repo("alpha")],
    options: { cwd: tree },
  },
  {
    name: "refused_unknown_repository",
    locations: [repo("alpha")],
    options: { graphExports: [`${repo("zulu")}=z.json`], cwd: tree },
  },
  {
    name: "refused_duplicate_export",
    locations: [repo("alpha")],
    options: {
      graphExports: [`${repo("alpha")}=one.json`, `${repo("alpha")}=two.json`],
      cwd: tree,
    },
  },
  {
    name: "refused_duplicate_premises",
    locations: [repo("alpha")],
    options: {
      graphPremises: [`${repo("alpha")}=one.json`, `${repo("alpha")}=two.json`],
      cwd: tree,
    },
  },
  {
    name: "refused_duplicate_audit",
    locations: [repo("alpha")],
    options: {
      graphAudits: [`${repo("alpha")}=one.json`, `${repo("alpha")}=two.json`],
      cwd: tree,
    },
  },
  {
    name: "duplicate_agrees",
    locations: [repo("alpha")],
    options: {
      graphExports: [`${repo("alpha")}=one.json`, `${repo("alpha")}=one.json`],
      cwd: tree,
    },
  },
  ...[
    ["no_separator", "alpha"],
    ["separator_first", "=alpha"],
    ["separator_last", "alpha="],
    ["blank_repository", "  =x.json"],
    ["blank_value", "alpha=   "],
    ["empty", ""],
    ["two_separators", "alpha=a=b"],
  ].map(([name, value]) => ({
    name: `malformed_${name}`,
    locations: [repo("alpha")],
    options: { graphExports: [value], cwd: tree },
  })),
].map((entry) => {
  const serialisable = JSON.parse(normalise(JSON.stringify(entry)));
  try {
    const mappings = parseGraphPortfolioMappings(entry.locations, entry.options);
    return {
      ...serialisable,
      verdict: "accepted",
      output: normalise(canonicalJson(mappings)),
    };
  } catch (error) {
    return {
      ...serialisable,
      verdict: "refused",
      code: error.code ?? null,
      message: normalise(error.message),
    };
  }
});

// ----------------------------------------------------------- the portfolio

const readsOf = (name) => {
  const root = repo(name);
  const reads = readMeasurementCollectionResults(root).map((read) =>
    read.collection
      ? { path: read.path, collection: read.collection }
      : { path: read.path, availability: "unreadable", error: read.error },
  );
  for (const refusal of injected.refusals[name] ?? [])
    reads.push({
      path: join(root, refusal.path),
      availability: refusal.availability,
      error: refusal.error,
    });
  return reads;
};

const collectionsOf = (reads) =>
  reads
    .flatMap((read) => (read.collection ? [read.collection] : []))
    .sort(
      (a, b) =>
        Date.parse(a.timestamp) - Date.parse(b.timestamp) ||
        (a.collectionId < b.collectionId ? -1 : a.collectionId > b.collectionId ? 1 : 0),
    );

const names = injected.repositories;
const readsByName = new Map(names.map((name) => [name, readsOf(name)]));
const portfolio = buildPortfolioReportFromCollections(
  names.map((name) => ({
    root: repo(name),
    collections: collectionsOf(readsByName.get(name)),
  })),
);
const byRoot = new Map(
  portfolio.repositories.map((entry) => [entry.root, entry]),
);
const inputs = names.map((name) => ({
  portfolio: byRoot.get(repo(name)),
  plans: loadMeasurementPlans(repo(name), { includeGovernance: true }),
  collections: readsByName.get(name),
  graph: injected.graphs[name],
}));

const report = buildGovernedGraphPortfolioFrom(inputs);
const portfolioCase = {
  repositories: names,
  json: normalise(canonicalGraphPortfolioJson(report)),
  rendered: normalise(renderGovernedGraphPortfolio(report)),
};

// ---------------------------------------------------------- the comparison
//
// `compareGraphQualityCollections` is exported and is the one function whose
// every branch — comparable, incomparable, and each of the seven blocking
// codes — is reachable without a whole portfolio around it.

const collectionOf = (name, id) =>
  collectionsOf(readsByName.get(name)).find((row) => row.collectionId === id);

const comparisonCases = [
  ["alpha_pair", ["alpha", "a-2026-01"], ["alpha", "a-2026-02"]],
  ["alpha_reversed", ["alpha", "a-2026-02"], ["alpha", "a-2026-01"]],
  ["alpha_with_itself", ["alpha", "a-2026-01"], ["alpha", "a-2026-01"]],
  ["echo_pair", ["echo", "e-2026-01"], ["echo", "e-2026-02"]],
  ["cross_repository", ["alpha", "a-2026-01"], ["bravo", "b-2026-01"]],
  ["charlie_against_alpha", ["charlie", "c-2026-01"], ["alpha", "a-2026-01"]],
  ["no_graph_observations", ["alpha", "a-2026-03"], ["alpha", "a-2026-03"]],
].map(([name, [beforeRepo, beforeId], [afterRepo, afterId]]) => ({
  name,
  before: { repository: beforeRepo, collection: beforeId },
  after: { repository: afterRepo, collection: afterId },
  json: normalise(
    canonicalJson(
      compareGraphQualityCollections(
        collectionOf(beforeRepo, beforeId),
        collectionOf(afterRepo, afterId),
      ),
    ),
  ),
}));

// --------------------------------------------------------------- the capture

const revision = execFileSync("git", ["rev-parse", "HEAD"], {
  cwd: repoRoot,
  encoding: "utf8",
}).trim();
const digestOf = (path) =>
  `sha256:${createHash("sha256").update(readFileSync(join(repoRoot, path))).digest("hex")}`;

const capture = {
  produced_by: "src/measurement/graph-portfolio.ts",
  produced_by_digest: digestOf("src/measurement/graph-portfolio.ts"),
  produced_from_revision: revision,
  produced_by_node: process.version,
  capture_script:
    "rust/crates/quoin-measurement-graph/oracle/capture-graph-portfolio-oracle.mjs",
  fixture_tree:
    "rust/crates/quoin-measurement-graph/tests/fixtures/graph-portfolio-tree",
  tree_token: TREE_TOKEN,
  mappings: mappingCases,
  portfolio: portfolioCase,
  comparisons: comparisonCases,
};

mkdirSync(dirname(resolve(out)), { recursive: true });
writeFileSync(resolve(out), `${JSON.stringify(capture, null, 2)}\n`);
console.log(
  `${mappingCases.length} mapping cases, ${comparisonCases.length} comparisons, ` +
    `${portfolioCase.repositories.length} repositories -> ${resolve(out)}`,
);
