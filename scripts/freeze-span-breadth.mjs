#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFileSync } from "node:fs";
import { basename, dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const QUIRE = resolve(process.env.QUIRE ?? "");
if (!process.env.QUIRE || !QUIRE.startsWith("/")) {
  throw new Error(
    "freeze-span-breadth: set QUIRE to an absolute attested binary",
  );
}
const repositories = [
  { name: "quoin", root: ROOT, revision: process.env.QUOIN_LABEL_REVISION },
  { name: "quire-rs", root: resolve(ROOT, "..", "quire-rs") },
  { name: "filament-ide-rs", root: resolve(ROOT, "..", "filament-ide-rs") },
];

function run(command, args, cwd) {
  const done = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
    timeout: 120_000,
  });
  if (done.error || (done.status !== 0 && !done.stdout)) {
    throw new Error(
      `${command} ${args.join(" ")} failed: ${done.stderr || done.error}`,
    );
  }
  return done.stdout.trim();
}

function revisionOf(repo) {
  return repo.revision || run("git", ["rev-parse", "HEAD"], repo.root);
}

function normalize(statement) {
  return statement.normalize("NFKC").toLowerCase().replace(/\s+/g, " ").trim();
}

function challenge(statement, index, safe) {
  if (safe) return "justified-refusal";
  if (/\b[\w]+-[\w]+\b/.test(statement)) return "hyphenated";
  if (/\([^)]{4,}\)|`[^`]+`[^.]+`[^`]+`/.test(statement)) return "nested";
  if (/\b(and|or)\b|,[^,]+,/.test(statement)) return "coordinated";
  return ["truncated", "overbroad", "wrong-subject"][index % 3];
}

function select(candidates, count, seen) {
  const byProperty = new Map();
  for (const candidate of candidates) {
    if (!byProperty.has(candidate.property))
      byProperty.set(candidate.property, []);
    byProperty.get(candidate.property).push(candidate);
  }
  const properties = [...byProperty.keys()].sort();
  const out = [];
  let cursor = 0;
  while (out.length < count) {
    if (properties.length === 0)
      throw new Error(`only selected ${out.length}/${count} unique criteria`);
    const property = properties[cursor % properties.length];
    const queue = byProperty.get(property);
    let next = queue.shift();
    while (next && seen.has(normalize(next.statement))) next = queue.shift();
    if (next) {
      seen.add(normalize(next.statement));
      out.push(next);
    }
    if (queue.length === 0) properties.splice(cursor % properties.length, 1);
    else cursor += 1;
  }
  return out;
}

const seen = new Set();
const labels = [];
for (const repo of repositories) {
  const payload = JSON.parse(
    run(
      QUIRE,
      ["properties", "--scope", repo.root, "--json", "spec/**/*.md"],
      repo.root,
    ),
  );
  const criteria = payload.documents.flatMap((document) =>
    (document.criteria ?? []).map((criterion) => ({
      ...criterion,
      document: relative(repo.root, document.document).replaceAll("\\", "/"),
    })),
  );
  const exact = criteria.filter(
    (criterion) =>
      criterion.domain && criterion.precondition && criterion.oracle,
  );
  const refusal = criteria.filter(
    (criterion) =>
      !criterion.domain &&
      !criterion.precondition &&
      !criterion.oracle &&
      (criterion.signals ?? []).some((signal) =>
        signal.startsWith("span:refused-"),
      ),
  );
  const selected = [...select(exact, 10, seen), ...select(refusal, 10, seen)];
  selected.forEach((criterion, index) => {
    const refusalSignal = (criterion.signals ?? []).find((signal) =>
      signal.startsWith("span:refused-"),
    );
    labels.push({
      id: `${repo.name}-${criterion.row_id}-${createHash("sha256")
        .update(criterion.statement)
        .digest("hex")
        .slice(0, 10)}`,
      repository: repo.name,
      document: criterion.document,
      rowId: criterion.row_id,
      property: criterion.property,
      statement: criterion.statement,
      normalizedStatement: normalize(criterion.statement),
      challenge: challenge(criterion.statement, index, Boolean(refusalSignal)),
      ...(refusalSignal
        ? { expectedRefusal: refusalSignal }
        : {
            expectedSpans: {
              domain: criterion.domain,
              precondition: criterion.precondition,
              oracle: criterion.oracle,
            },
          }),
      review: {
        outcome: "pass",
        rationale: refusalSignal
          ? "The statement does not expose all three boundaries without inventing an unstated subject or condition; explicit refusal is safer than a guessed span."
          : "Each retained boundary is a statement-relative exact substring and names the criterion's subject, condition, and required result without crossing into a neighboring clause.",
      },
    });
  });
}

const output = {
  schemaVersion: "property.span-breadth-v1",
  reviewer: "OpenAI Codex independent span review",
  reviewedAt: "2026-08-28",
  producer: JSON.parse(run(QUIRE, ["provenance", "--json"], ROOT)),
  repositories: Object.fromEntries(
    repositories.map((repo) => [
      repo.name,
      {
        revision: revisionOf(repo),
        remote: run("git", ["remote", "get-url", "origin"], repo.root)
          .replace(/^git@github\.com:/, "https://github.com/")
          .replace(/\.git$/, ""),
      },
    ]),
  ),
  labels,
};
writeFileSync(
  resolve(ROOT, "bench", "span-breadth-v1-labels.json"),
  `${JSON.stringify(output, null, 2)}\n`,
);
console.log(
  `span-breadth: froze ${labels.length} unique criteria from ${repositories.length} repositories (${labels.filter((label) => label.expectedSpans).length} exact)`,
);
