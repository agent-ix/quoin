// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// One-time oracle capture: the schema object `src/measurement/intervention-schema.ts`
// BUILDS, serialized with the repository's own `canonicalJson`.
//
// The TypeScript is a program, not a document, so the vendored Rust artifact
// cannot be diffed against it by eye. This records what that program produced
// at one revision; `tests/tc_470_vendored_schemas.rs` asserts the committed
// Rust document differs from this capture by exactly the two recorded deltas.
//
// Run from the repository root:
//   node --experimental-strip-types \
//     rust/crates/quoin-jsonschema/oracle/capture-intervention-schema.mjs
//
// Deleted at the cutover commit per FR-101-AC-5; the capture it wrote is the
// record from then on.

import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { canonicalJson } from "../../../../src/store/canonical.ts";
import { interventionExperimentSchema } from "../../../../src/measurement/intervention-schema.ts";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..", "..");
const revision = execFileSync("git", ["-C", repo, "rev-parse", "HEAD"], {
  encoding: "utf8",
}).trim();

const out = join(
  here,
  "..",
  "tests",
  "goldens",
  "intervention-experiment-v1.captured.json",
);
writeFileSync(out, canonicalJson(interventionExperimentSchema), "utf8");
writeFileSync(
  join(
    here,
    "..",
    "tests",
    "goldens",
    "intervention-experiment-v1.captured.provenance.json",
  ),
  canonicalJson({
    captured_from: "src/measurement/intervention-schema.ts",
    exported_binding: "interventionExperimentSchema",
    serializer: "src/store/canonical.ts canonicalJson",
    producer:
      "rust/crates/quoin-jsonschema/oracle/capture-intervention-schema.mjs",
    quoin_revision: revision,
  }),
  "utf8",
);
console.log(`wrote ${out} at ${revision}`);
