#!/usr/bin/env node

import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { findingEnvelopeDigest } from "../evals/lib/finding-envelope.mjs";
import { loadCorpus, formatTier1Json } from "./bench-tier1.mjs";
import {
  ADVISORY_ADJUDICATION_SCHEMA,
  ADVISORY_METRIC_VERSION,
  ADVISORY_RUBRIC_VERSION,
  digestIds,
  verifyAdvisoryAdjudicationSource,
} from "./lib/advisory-adjudication.mjs";
import { standingAdjudications } from "./lib/tier1-corpus.mjs";
import { adjudicationOf } from "./lib/tier1-scoring.mjs";

const command = process.argv[2];

if (command === "freeze") {
  await freeze();
} else if (command === "verify") {
  verify();
} else {
  throw new Error(
    "usage: freeze-advisory-adjudication.mjs freeze --source-report <json> --corpus <qa-corpus> --source-revision <sha> --output <json> | verify --source-report <json> --adjudication <json>",
  );
}

async function freeze() {
  const sourcePath = required("--source-report");
  const corpusRoot = resolve(required("--corpus"));
  const sourceRevision = required("--source-revision");
  const output = resolve(required("--output"));
  const disposition = optional("--disposition") ?? "unresolved";
  const defectOwner = optional("--defect-owner") ?? "unresolved";
  const reviewer = optional("--reviewer") ?? "quoin-maintainers";
  const followUp = optional("--follow-up");
  const retainedSource = optional("--retain-source");
  const sourceBytes = readFileSync(sourcePath);
  const report = JSON.parse(sourceBytes);
  const mapping = JSON.parse(
    readFileSync(resolve("bench/tier1-mapping.json"), "utf8"),
  );
  const loaded = loadCorpus(mapping, corpusRoot);
  const adjudication = adjudicationOf(
    loaded.corpora.filter((entry) => !entry.pending),
    mapping,
    standingAdjudications(corpusRoot),
  );
  const advisoryFamilies = new Set(
    (report.families ?? [])
      .filter((family) => family.shape === "advisory")
      .map((family) => family.family),
  );
  const findings = (report.finding_records ?? [])
    .filter(
      (finding) =>
        advisoryFamilies.has(finding.identity?.family) &&
        isUnadjudicated(finding, adjudication[finding.identity.family]),
    )
    .map((finding) => ({ id: findingEnvelopeDigest(finding), finding }))
    .sort((a, b) => a.id.localeCompare(b.id));
  const expected = (report.families ?? [])
    .filter((family) => family.shape === "advisory")
    .reduce((total, family) => total + family.precision_basis.unadjudicated, 0);
  if (findings.length !== expected) {
    throw new Error(
      `advisory adjudication: selected ${findings.length} rows but source report declares ${expected} unadjudicated`,
    );
  }
  const artifact = {
    schema_version: ADVISORY_ADJUDICATION_SCHEMA,
    metric_version: ADVISORY_METRIC_VERSION,
    rubric: {
      version: ADVISORY_RUBRIC_VERSION,
      correct:
        "The finding truthfully reports that an extractable criterion has no specific property shape, or that the named declaration matched no corpus artifact under the pinned module.",
      incorrect:
        "The pinned source contradicts the finding's claim or the producer selected the wrong subject.",
      ambiguous:
        "Reviewers applying this rubric disagree; the row remains outside the precision denominator.",
      unresolved:
        "The retained source evidence cannot settle the claim; the row remains outside the precision denominator.",
    },
    population: {
      source: {
        revision: sourceRevision,
        report_digest: sha256(sourceBytes),
        provenance: report.provenance,
      },
      selection:
        "Every finding in a source-report advisory family not governed by a compatible per-case, scoped, or standing ruling.",
      count: findings.length,
      digest: digestIds(findings.map((entry) => entry.id)),
    },
    findings,
    rulings: findings.map((entry) => ({
      finding_id: entry.id,
      disposition,
      defect_owner: defectOwner,
      rationale:
        disposition === "unresolved"
          ? "Awaiting row-level review against the pinned source."
          : rationaleFor(entry.finding),
      reviewer,
      rubric_version: ADVISORY_RUBRIC_VERSION,
      metric_version: ADVISORY_METRIC_VERSION,
      disagreements: [],
      follow_up: followUp,
    })),
  };
  writeFileSync(output, await formatTier1Json(artifact, output));
  if (retainedSource) {
    const retainedPath = resolve(retainedSource);
    mkdirSync(dirname(retainedPath), { recursive: true });
    writeFileSync(retainedPath, sourceBytes);
  }
  console.log(`${output}: froze ${findings.length} finding(s)`);
}

function verify() {
  const sourcePath = required("--source-report");
  const artifactPath = required("--adjudication");
  const sourceBytes = readFileSync(sourcePath);
  const artifact = JSON.parse(readFileSync(artifactPath, "utf8"));
  const loaded = verifyAdvisoryAdjudicationSource(artifact, sourceBytes);
  console.log(
    `${artifactPath}: verified ${loaded.population.count} finding(s), ${JSON.stringify(loaded.counts)}`,
  );
}

function isUnadjudicated(finding, ruling = {}) {
  const caseName = finding.identity?.case;
  const declaration = finding.identity?.declaration;
  const matches = (rows) =>
    (rows ?? []).some(
      (row) =>
        row.corpus === caseName &&
        (row.scope === null || row.scope === declaration),
    );
  return !matches(ruling.present) && !matches(ruling.absent);
}

function sha256(bytes) {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

function required(flag) {
  const index = process.argv.indexOf(flag);
  const value = index === -1 ? null : process.argv[index + 1];
  if (!value) throw new Error(`missing ${flag}`);
  return value;
}

function optional(flag) {
  const index = process.argv.indexOf(flag);
  return index === -1 ? null : (process.argv[index + 1] ?? null);
}

function rationaleFor(finding) {
  if (finding.identity?.family === "catch-all-universal") {
    return (
      `The pinned ${finding.identity.declaration} declaration bound the ` +
      `extractable criterion at ${finding.raw.path}:${finding.raw.line}, and ` +
      "the source row declares no specific property shape; the reported " +
      "one-of-one, none-shaped claim is true."
    );
  }
  if (finding.identity?.family === "archetype-matches-nothing") {
    return (
      `Pinned case ${finding.identity.case} contains no TestMatrix document ` +
      "while the test-case declaration selects TestMatrix, so that declaration " +
      "scans and mints no rows as reported."
    );
  }
  throw new Error(
    `advisory adjudication: no reviewed rationale for ${finding.identity?.family}`,
  );
}
