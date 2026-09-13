// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// One-time oracle capture: ajv's VERDICT on a named corpus, under the exact
// two ajv configurations the retained code uses.
//
// The corpus is `retained records × a fixed mutation ladder`:
//
//   * every JSON record retained under `spec/evidence/interventions/` and
//     `spec/evidence/operational/pairs/` is a base document;
//   * each base is then mutated once per entry in the ladder below — a named
//     (pointer, operation) pair. The ladder walks every required key of the
//     base, plus the field-level refusals the two recorded deltas move.
//
// Mutation is chosen over hand-written fixtures because a hand-written invalid
// document proves whatever its author believed; a required key removed from a
// real retained record proves the two validators agree about a document that
// actually occurs, minus one thing.
//
// Run from the repository root:
//   node --experimental-strip-types \
//     rust/crates/quoin-jsonschema/oracle/capture-ajv-verdicts.mjs
//
// Deleted at the cutover commit per FR-101-AC-5.

import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";

import { canonicalJson } from "../../../../src/store/canonical.ts";
import { parseRfc3339DateTime } from "../../../../src/measurement/date-time.ts";
import { interventionExperimentSchema } from "../../../../src/measurement/intervention-schema.ts";
import { operationalEvidenceSchema } from "../../../../src/measurement/operational-schema.ts";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..", "..");

// Copied verbatim from `src/measurement/intervention.ts:346-380`, which does
// not export it. This is the retained strict grammar, and capturing ajv's
// behaviour requires the exact check the retained code registered.
function isRfc3339DateTime(value) {
  const match =
    /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(?:Z|([+-])(\d{2}):(\d{2}))$/.exec(
      value,
    );
  if (!match) return false;
  const [, yearText, monthText, dayText, hourText, minuteText, secondText] =
    match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const hour = Number(hourText);
  const minute = Number(minuteText);
  const second = Number(secondText);
  const offsetHour = Number(match[8] ?? 0);
  const offsetMinute = Number(match[9] ?? 0);
  return (
    month >= 1 &&
    month <= 12 &&
    day >= 1 &&
    day <= daysInMonth(year, month) &&
    hour <= 23 &&
    minute <= 59 &&
    second <= 59 &&
    offsetHour <= 23 &&
    offsetMinute <= 59
  );
}

function daysInMonth(year, month) {
  if (month === 2) {
    const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
    return leap ? 29 : 28;
  }
  return [4, 6, 9, 11].includes(month) ? 30 : 31;
}

// `src/measurement/intervention.ts:22-27`.
const interventionAjv = new Ajv2020({
  allErrors: true,
  strict: false,
  formats: { "date-time": isRfc3339DateTime },
});
const validateIntervention = interventionAjv.compile(interventionExperimentSchema);

// `src/measurement/operational.ts:30-35`.
const operationalAjv = new Ajv2020({ allErrors: true, strict: false });
operationalAjv.addFormat("date-time", {
  type: "string",
  validate: (value) => parseRfc3339DateTime(value) !== null,
});
const validateOperational = operationalAjv.compile(operationalEvidenceSchema);

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

/** Every retained base document, with the path it came from. */
function bases() {
  const out = [];
  const interventions = join(repo, "spec", "evidence", "interventions");
  for (const name of readdirSync(interventions).sort()) {
    if (!name.endsWith(".json")) continue;
    out.push({
      schema: "intervention_experiment_v1",
      origin: `spec/evidence/interventions/${name}`,
      document: readJson(join(interventions, name)),
    });
  }
  const pairs = join(repo, "spec", "evidence", "operational", "pairs");
  for (const name of readdirSync(pairs).sort()) {
    if (!name.endsWith(".json")) continue;
    const pair = readJson(join(pairs, name));
    for (const [index, record] of pair.records.entries()) {
      out.push({
        schema: "operational_evidence_v1",
        origin: `spec/evidence/operational/pairs/${name}#/records/${index}`,
        document: record,
      });
    }
  }
  return out;
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function setAt(document, path, value) {
  let node = document;
  for (const key of path.slice(0, -1)) node = node[key];
  node[path.at(-1)] = value;
}

function removeAt(document, path) {
  let node = document;
  for (const key of path.slice(0, -1)) node = node[key];
  if (Array.isArray(node)) node.splice(Number(path.at(-1)), 1);
  else delete node[path.at(-1)];
}

/** The mutation ladder for one base document. */
function ladder(base) {
  const rungs = [];
  for (const key of Object.keys(base).sort()) {
    rungs.push({ name: `remove/${key}`, apply: (d) => removeAt(d, [key]) });
  }
  rungs.push({
    name: "add/unknown-top-level-key",
    apply: (d) => setAt(d, ["quoin_unknown_key"], 1),
  });
  rungs.push({
    name: "set/schema_version=2",
    apply: (d) => setAt(d, ["schema_version"], 2),
  });
  rungs.push({
    name: "set/record_type=wrong",
    apply: (d) => setAt(d, ["record_type"], "not_a_record_type"),
  });
  rungs.push({
    name: "set/record_id=unsafe",
    apply: (d) => setAt(d, ["record_id"], "/leading-slash"),
  });
  rungs.push({
    name: "set/record_id=empty",
    apply: (d) => setAt(d, ["record_id"], ""),
  });
  for (const [name, value] of [
    ["date-only", "2026-01-01"],
    ["lowercase-t-and-z", "2026-01-01t00:00:00z"],
    ["lowercase-t-only", "2026-01-01t00:00:00Z"],
    ["lowercase-z-only", "2026-01-01T00:00:00z"],
    ["impossible-day", "2026-02-30T00:00:00Z"],
    ["offset", "2026-01-01T00:00:00+01:00"],
    ["fractional", "2026-01-01T00:00:00.250Z"],
    ["not-a-string", 0],
  ]) {
    rungs.push({
      name: `set/observed_at=${name}`,
      apply: (d) => setAt(d, ["observed_at"], value),
    });
  }
  for (const [name, value] of [
    ["semver", "1.2.3"],
    ["v-semver", "v1.2.3-rc.1"],
    ["git-sha1", "a".repeat(40)],
    ["sha256-prefixed", `sha256:${"b".repeat(64)}`],
    ["blake3-prefixed", `blake3:${"c".repeat(64)}`],
    ["bare-sha256", "d".repeat(64)],
    ["garbage", "not a version"],
  ]) {
    rungs.push({
      name: `set/producer.tool_version=${name}`,
      apply: (d) => setAt(d, ["producer", "tool_version"], value),
    });
  }
  rungs.push({
    name: "set/producer.environment={}",
    apply: (d) => setAt(d, ["producer", "environment"], {}),
  });
  rungs.push({
    name: "set/producer.configuration_digest=bare-hex",
    apply: (d) => setAt(d, ["producer", "configuration_digest"], "e".repeat(64)),
  });
  rungs.push({
    name: "set/raw_evidence=[]",
    apply: (d) => setAt(d, ["raw_evidence"], []),
  });
  rungs.push({
    name: "set/owner=empty",
    apply: (d) => setAt(d, ["owner"], ""),
  });
  return rungs;
}

const corpus = [];
for (const base of bases()) {
  corpus.push({
    id: `${base.origin}#base`,
    schema: base.schema,
    mutation: "none",
    origin: base.origin,
    document: base.document,
  });
  for (const rung of ladder(base.document)) {
    const document = clone(base.document);
    try {
      rung.apply(document);
    } catch {
      // A rung that does not apply to this base (an absent parent) is not a
      // corpus entry; the ladder is shared across two record shapes.
      continue;
    }
    if (canonicalJson(document) === canonicalJson(base.document)) continue;
    corpus.push({
      id: `${base.origin}#${rung.name}`,
      schema: base.schema,
      mutation: rung.name,
      origin: base.origin,
      document,
    });
  }
}

const entries = corpus.map((entry) => {
  const validate =
    entry.schema === "intervention_experiment_v1"
      ? validateIntervention
      : validateOperational;
  const valid = validate(entry.document);
  const identities = (validate.errors ?? [])
    .map((error) => [error.instancePath, error.keyword])
    .sort((a, b) => (a[0] === b[0] ? a[1].localeCompare(b[1]) : a[0].localeCompare(b[0])));
  return { ...entry, ajv_valid: valid, ajv_error_identities: identities };
});

const revision = execFileSync("git", ["-C", repo, "rev-parse", "HEAD"], {
  encoding: "utf8",
}).trim();

writeFileSync(
  join(here, "..", "tests", "goldens", "ajv-verdicts.json"),
  canonicalJson({
    provenance: {
      producer: "rust/crates/quoin-jsonschema/oracle/capture-ajv-verdicts.mjs",
      ajv_version: JSON.parse(
        readFileSync(
          createRequire(import.meta.url).resolve("ajv/package.json"),
          "utf8",
        ),
      ).version,
      intervention_config: "src/measurement/intervention.ts:22-27",
      operational_config: "src/measurement/operational.ts:30-35",
      quoin_revision: revision,
    },
    entries,
  }),
  "utf8",
);
const invalid = entries.filter((e) => !e.ajv_valid).length;
console.log(
  `wrote ${entries.length} entries (${entries.length - invalid} accepted, ${invalid} refused) at ${revision}`,
);
