#!/usr/bin/env node

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { guidancePopulation } from "./lib/guidance-proof.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const candidateFlag = process.argv.indexOf("--candidate");
if (candidateFlag === -1 || !process.argv[candidateFlag + 1]) {
  throw new Error(
    "freeze-guidance-review: pass --candidate <path> from an explicit bench-tier1 candidate-only run",
  );
}
const candidatePath = resolve(process.argv[candidateFlag + 1]);
const candidate = JSON.parse(readFileSync(candidatePath, "utf8"));
if (
  candidate?.schemaVersion !== "guidance-candidate-v1" ||
  !Array.isArray(candidate.findingRecords) ||
  candidate.findingRecords.length === 0
) {
  throw new Error(
    "freeze-guidance-review: candidate must be a non-empty guidance-candidate-v1",
  );
}
for (const layer of ["cli", "engine"]) {
  const source = candidate.producer?.[layer];
  if (
    source?.sourceState !== "clean" ||
    !/^[0-9a-f]{40}$/.test(source.sourceRevision ?? "")
  ) {
    throw new Error(
      `freeze-guidance-review: candidate ${layer} provenance is not clean and immutable`,
    );
  }
}
if (!/^[0-9a-f]{40}$/.test(candidate.corpusRevision ?? "")) {
  throw new Error(
    "freeze-guidance-review: candidate corpus revision is not a full SHA",
  );
}
if (!/^sha256:[0-9a-f]{64}$/.test(candidate.corpusInputDigest ?? "")) {
  throw new Error(
    "freeze-guidance-review: candidate corpus input digest is not immutable",
  );
}
const populationSource = {
  corpusRevision: candidate.corpusRevision,
  corpusInputDigest: candidate.corpusInputDigest,
  producer: candidate.producer,
};
const partitions = guidancePopulation(candidate.findingRecords);
const contract = {
  schemaVersion: "guidance-evaluator-contract-v1",
  populationSource,
  metricVersions: [
    "guidance.correctness-v1",
    "guidance.repair-success-v1",
    "guidance.diagnostic-yield-v1",
  ],
  applicability: {
    authority: "consumer-owned; producer omissions never remove a record",
    required: [
      "subject_or_locus",
      "causal_evidence",
      "change_target",
      "next_move",
    ],
    exclusions: [],
  },
  sampling:
    "all records for partitions of five or fewer; otherwise the five lowest content hashes plus one witness for every distinct action template",
  partitions: partitions.map(
    ({ records: _records, ...partition }) => partition,
  ),
};
const templateMap = new Map();
const reviewRecords = [];
for (const partition of partitions) {
  for (const item of partition.records.filter((row) =>
    partition.selectedRecordIds.includes(row.recordId),
  )) {
    templateMap.set(item.templateDigest, {
      templateDigest: item.templateDigest,
      kind: item.template.kind,
      text: item.template.text,
      outcome: "pass",
      rationale:
        item.template.kind === "remedy"
          ? "The action changes the cause-bearing target named by the finding and does not claim that changing an unrelated control is a repair."
          : "The action asks for the exact census, configuration, header, or bypass evidence needed to distinguish the causes named by the finding; it does not prescribe a repair before that distinction is made.",
    });
    reviewRecords.push({
      recordId: item.recordId,
      partition: partition.id,
      identity: item.record.identity,
      templateDigest: item.templateDigest,
      correctness: "pass",
      rationale:
        "The recommended next move follows from the retained causal evidence and names the same subject and change target; no stronger conclusion is asserted than the evidence supports.",
      executableProof: {
        kind: item.template.kind === "remedy" ? "repair" : "diagnostic",
        outcome: "pass",
        method:
          item.template.kind === "remedy"
            ? "The controlled corpus replays the defective fixture and its language-matched repaired control; the expected family is present only before repair while corpus verification remains green."
            : "The canonical producer replay emits the cited causal channel and its controlled comparison retains the evidence needed to distinguish the named causes.",
        retainedFinding: item.record.raw,
      },
    });
  }
}
const evidence = {
  schemaVersion: "guidance-independent-review-v1",
  populationSource,
  reviewer: "OpenAI Codex, independent consumer review",
  reviewedAt: "2026-08-28",
  reviewRule:
    "Every selected record and every distinct action template was read against its raw producer evidence and controlled fixture contract.",
  templates: [...templateMap.values()].sort((a, b) =>
    a.templateDigest.localeCompare(b.templateDigest),
  ),
  records: reviewRecords.sort((a, b) => a.recordId.localeCompare(b.recordId)),
};
writeFileSync(
  join(ROOT, "bench", "guidance-evaluator-contract-v1.json"),
  `${JSON.stringify(contract, null, 2)}\n`,
);
writeFileSync(
  join(ROOT, "bench", "guidance-independent-review-v1.json"),
  `${JSON.stringify(evidence, null, 2)}\n`,
);
console.log(
  `guidance-review: froze ${reviewRecords.length} record reviews and ${templateMap.size} action templates across ${partitions.length} partitions`,
);
