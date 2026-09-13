// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The TypeScript half of the W6 reporting gate (quoin#473, FR-100-AC-4).
//
//   node --loader ts-node/esm oracle/capture-portfolio-oracle.mjs \
//     --out tests/fixtures/portfolio-oracle.json
//
// Runs the six functions the wave is gated on — `renderMeasurementReport`,
// `renderMeasurementReportJson`, `renderPortfolioReport`,
// `renderPortfolioReportJson`, `comparisonFor` and `seriesFor` — over the
// committed fixture tree under `tests/fixtures/portfolio-tree/` and writes what
// THEY produce. Nothing is asserted here; `tc_473_reporting.rs` compares.
//
// The output is committed and frozen. Per FR-101-AC-11 the capture records the
// producing implementation, the quoin revision it was taken at and the node
// that ran it, so the fixture is evidence rather than a file someone once
// generated. FR-101-AC-5 forbids a live TypeScript runtime at test time, and
// `tc_468_boundary` would catch one.
//
// # The one normalisation
//
// `resolve()` makes every `root` and `path` in the output absolute, so the
// bytes would otherwise record whoever's checkout produced them. Every
// occurrence of the fixture tree's own absolute root is replaced by the token
// below, on this side and on the Rust side, before anything is compared. It is
// the only edit made to what the TypeScript produced.
//
// `QUOIN_SRC_ROOT` overrides where the TypeScript is read from. It must be a
// checkout with `node_modules` installed.

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const crateRoot = resolve(join(here, ".."));
const repoRoot = resolve(
  process.env.QUOIN_SRC_ROOT ?? join(here, "..", "..", "..", ".."),
);
const load = (path) => import(pathToFileURL(join(repoRoot, "src", path)).href);

const { canonicalJson } = await load("store/canonical.js");
const {
  renderMeasurementReport,
  renderMeasurementReportJson,
  buildMeasurementReport,
  comparisonFor,
  seriesFor,
} = await load("measurement/report.js");
const {
  buildPortfolioReport,
  renderPortfolioReport,
  renderPortfolioReportJson,
} = await load("measurement/portfolio.js");

const argv = process.argv.slice(2);
let out = null;
for (let index = 0; index < argv.length; index += 1) {
  if (argv[index] === "--out") {
    out = argv[index + 1];
    index += 1;
  }
}
if (out === null) {
  throw new Error("usage: capture-portfolio-oracle.mjs --out <file>");
}

const TREE_TOKEN = "@@TREE@@";
const tree = resolve(join(crateRoot, "tests", "fixtures", "portfolio-tree"));
const normalise = (text) => text.split(tree).join(TREE_TOKEN);
const repo = (name) => join(tree, name);

const repositories = ["alpha", "bravo", "charlie", "delta", "golf"];

const reportCases = repositories.map((name) => {
  const report = buildMeasurementReport(repo(name));
  return {
    name,
    rendered: normalise(renderMeasurementReport(report)),
    json: normalise(renderMeasurementReportJson(report)),
  };
});

const comparisonCases = [
  { name: "alpha_baseline_found", repository: "alpha", before: "revision-one" },
  { name: "alpha_baseline_prefix", repository: "alpha", before: "revision" },
  { name: "alpha_no_baseline", repository: "alpha", before: "revision-absent" },
  { name: "bravo_no_current", repository: "bravo", before: "revision-one" },
].map(({ name, repository, before }) => {
  const report = comparisonFor(repo(repository), before);
  return { name, repository, before, json: normalise(canonicalJson(report)) };
});

const seriesCases = [
  { name: "alpha_recall", repository: "alpha", metric: "finding_recall" },
  {
    name: "alpha_actionability",
    repository: "alpha",
    metric: "actionability_rate",
  },
  { name: "alpha_unknown", repository: "alpha", metric: "no_such_metric" },
  { name: "bravo_recall", repository: "bravo", metric: "finding_recall" },
].map(({ name, repository, metric }) => ({
  name,
  repository,
  metric,
  json: normalise(canonicalJson(seriesFor(repo(repository), metric))),
}));

// Deliberately out of order, with one repeated spelling and two locations that
// are not readable repositories: the walk resolves, de-duplicates and orders
// them itself (`portfolio.ts:79-82`).
const portfolioLocations = [
  repo("golf"),
  repo("delta"),
  repo("alpha"),
  join(tree, "alpha", "spec", "..", "..", "alpha"),
  repo("bravo"),
  repo("charlie"),
  repo("echo-not-a-directory"),
  repo("zulu-does-not-exist"),
];

const portfolio = buildPortfolioReport(portfolioLocations);
const portfolioCase = {
  locations: portfolioLocations.map(normalise),
  rendered: normalise(renderPortfolioReport(portfolio)),
  json: normalise(renderPortfolioReportJson(portfolio)),
};

const revision = execFileSync("git", ["rev-parse", "HEAD"], {
  cwd: repoRoot,
  encoding: "utf8",
}).trim();

const capture = {
  produced_by: "src/measurement/report.ts, src/measurement/portfolio.ts",
  produced_from_revision: revision,
  produced_by_node: process.version,
  capture_script:
    "rust/crates/quoin-measurement/oracle/capture-portfolio-oracle.mjs",
  fixture_tree: "rust/crates/quoin-measurement/tests/fixtures/portfolio-tree",
  tree_token: TREE_TOKEN,
  reports: reportCases,
  comparisons: comparisonCases,
  series: seriesCases,
  portfolio: portfolioCase,
};

mkdirSync(dirname(resolve(out)), { recursive: true });
writeFileSync(resolve(out), `${JSON.stringify(capture, null, 2)}\n`);
