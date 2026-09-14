// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// One-time oracle capture: the VERDICT `src/quire/validate.ts:99` returns for
// every document in the `assurance-v1` parity corpus (quoin#474).
//
// The oracle is `validateAssurance` itself — imported, not reconstructed. The
// two measurement captures had to rebuild their ajv instance because the
// retained modules do not export one; `validate.ts` exports the function, so
// there is nothing to restate and nothing to get subtly wrong.
//
// The corpus is `base documents x a fixed named mutation ladder`:
//
//   * the bases come from `tests/goldens/assurance-bases.json`, produced once
//     by `oracle/capture-assurance-bases.rs` from the linked engine over the
//     committed fixture corpora. There is no retained `assurance-v1` document
//     in this repository to read instead;
//   * each base is mutated once per rung of the ladder below — a named
//     (pointer, operation) pair. The ladder walks every required key of the
//     base and of the first element of every collection, plus the field-level
//     refusals that separate the two validators: the `locator.path` traversal
//     pattern (a lookahead, which the two regex engines need not agree about)
//     and the `uuid` format (annotation-only under this ajv).
//
// Mutation rather than hand-written fixtures, because a hand-written invalid
// document proves whatever its author believed. A required key removed from a
// document the engine actually produced proves the two validators agree about
// a real document, minus one thing.
//
// `--loader ts-node/esm` and not `--experimental-strip-types`: `validate.ts`
// imports `./contract.js`, the extension TypeScript's node16 resolution maps
// back onto `contract.ts`. Type stripping does not do that mapping and cannot
// load the oracle at all; ts-node does, and is already a dev dependency.
//
// Run from the repository root, after `pnpm install`:
//   node --loader ts-node/esm \
//     rust/crates/quoin-quire/oracle/capture-assurance-verdicts.mjs
//
// Deleted at the cutover commit per FR-101-AC-5.

import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { canonicalJson } from "../../../../src/store/canonical.ts";
import { validateAssurance } from "../../../../src/quire/validate.ts";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..", "..");

const captured = JSON.parse(
  readFileSync(
    join(here, "..", "tests", "goldens", "assurance-bases.json"),
    "utf8",
  ),
);

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

/** Resolve `path` to its parent container, or throw if a segment is absent. */
function parentOf(document, path) {
  let node = document;
  for (const key of path.slice(0, -1)) {
    if (node === null || typeof node !== "object" || !(key in node)) {
      throw new Error(`absent: ${path.join("/")}`);
    }
    node = node[key];
  }
  return node;
}

function setAt(document, path, value) {
  parentOf(document, path)[path.at(-1)] = value;
}

function removeAt(document, path) {
  const node = parentOf(document, path);
  if (Array.isArray(node)) node.splice(Number(path.at(-1)), 1);
  else delete node[path.at(-1)];
}

/** The seven collections the export declares, in schema order. */
const COLLECTIONS = [
  "modules",
  "artifacts",
  "obligations",
  "symbols",
  "relation_kinds",
  "relations",
  "relation_observations",
];

const HEX64 = "a".repeat(64);

/** The mutation ladder for one base document. */
function ladder(base) {
  const rungs = [];
  const push = (name, apply) => rungs.push({ name, apply });

  for (const key of Object.keys(base).sort()) {
    push(`remove/${key}`, (d) => removeAt(d, [key]));
  }
  push("add/unknown-top-level-key", (d) => setAt(d, ["quoin_unknown_key"], 1));

  // The two `const` tags the reader keys on.
  push("set/format=wrong", (d) => setAt(d, ["format"], "quire-coverage"));
  push("set/format_version=2", (d) => setAt(d, ["format_version"], 2));
  push("set/format_version=string", (d) => setAt(d, ["format_version"], "1"));

  // The source premise: `revision` is the 40-hex pattern.
  push("remove/source.repository", (d) =>
    removeAt(d, ["source", "repository"]),
  );
  push("remove/source.revision", (d) => removeAt(d, ["source", "revision"]));
  push("add/source.unknown-key", (d) =>
    setAt(d, ["source", "quoin_unknown"], 1),
  );
  push("set/source.repository=empty", (d) =>
    setAt(d, ["source", "repository"], ""),
  );
  push("set/source.revision=39-hex", (d) =>
    setAt(d, ["source", "revision"], "a".repeat(39)),
  );
  push("set/source.revision=41-hex", (d) =>
    setAt(d, ["source", "revision"], "a".repeat(41)),
  );
  push("set/source.revision=uppercase", (d) =>
    setAt(d, ["source", "revision"], "A".repeat(40)),
  );
  push("set/source.revision=sha256", (d) =>
    setAt(d, ["source", "revision"], HEX64),
  );

  for (const collection of COLLECTIONS) {
    push(`set/${collection}=object`, (d) => setAt(d, [collection], {}));
    push(`set/${collection}=empty-array`, (d) => setAt(d, [collection], []));
    push(`set/${collection}=scalar`, (d) => setAt(d, [collection], 0));
    push(`remove/${collection}.0`, (d) => removeAt(d, [collection, "0"]));
    push(`add/${collection}.0.unknown-key`, (d) =>
      setAt(d, [collection, "0", "quoin_unknown"], 1),
    );
    const first = base[collection]?.[0];
    if (first && typeof first === "object") {
      for (const key of Object.keys(first).sort()) {
        push(`remove/${collection}.0.${key}`, (d) =>
          removeAt(d, [collection, "0", key]),
        );
        push(`set/${collection}.0.${key}=null`, (d) =>
          setAt(d, [collection, "0", key], null),
        );
      }
    }
  }

  // `locator.path` carries the only lookahead in the document:
  //   ^(?!/)(?!.*(?:^|/)\.\.(?:/|$)).+$
  // Two regex engines need not agree about a lookahead at all, so every arm of
  // it is a named rung — including the two that must be ACCEPTED, because a
  // pattern that refuses everything would agree with a pattern that was never
  // applied.
  for (const [name, value] of [
    ["absolute", "/etc/passwd"],
    ["dot-dot-leading", "../escape.md"],
    ["dot-dot-interior", "a/../b.md"],
    ["dot-dot-trailing", "a/.."],
    ["dot-dot-alone", ".."],
    ["dot-dot-as-name-prefix", "a/..b.md"],
    ["dot-dot-as-name-suffix", "a/b...md"],
    ["single-dot", "a/./b.md"],
    ["empty", ""],
    ["not-a-string", 1],
  ]) {
    push(`set/artifacts.0.locator.path=${name}`, (d) =>
      setAt(d, ["artifacts", "0", "locator", "path"], value),
    );
  }
  for (const [name, value] of [
    ["zero", 0],
    ["negative", -1],
    ["fractional", 1.5],
    ["string", "1"],
  ]) {
    push(`set/artifacts.0.locator.line=${name}`, (d) =>
      setAt(d, ["artifacts", "0", "locator", "line"], value),
    );
  }
  for (const [name, value] of [
    ["uppercase", "A".repeat(64)],
    ["63-hex", "a".repeat(63)],
    ["65-hex", "a".repeat(65)],
    ["prefixed", `sha256:${HEX64}`],
  ]) {
    push(`set/artifacts.0.locator.digest=${name}`, (d) =>
      setAt(d, ["artifacts", "0", "locator", "digest"], value),
    );
  }

  // `format: "uuid"` on `artifact.uuid`. The retained ajv registers NO format
  // check, so both values below are ACCEPTED today; a Rust validator built with
  // `compile_with_formats` would refuse the first. That is the rung which makes
  // the constructor choice measurable rather than asserted in prose.
  for (const [name, value] of [
    ["not-a-uuid", "not-a-uuid"],
    ["valid", "3f2504e0-4f89-11d3-9a0c-0305e82c3301"],
    ["number", 7],
  ]) {
    push(`set/artifacts.0.uuid=${name}`, (d) =>
      setAt(d, ["artifacts", "0", "uuid"], value),
    );
  }

  push("set/modules.0.schemas=object", (d) =>
    setAt(d, ["modules", "0", "schemas"], {}),
  );
  push("set/modules.0.version=empty", (d) =>
    setAt(d, ["modules", "0", "version"], ""),
  );
  push("set/symbols.0.capabilities=string", (d) =>
    setAt(d, ["symbols", "0", "capabilities"], "verifies"),
  );
  push("set/relations.0.kind=unknown", (d) =>
    setAt(d, ["relations", "0", "kind"], "quoin-made-up"),
  );

  return rungs;
}

const corpus = [];
for (const base of captured.bases) {
  corpus.push({
    id: `${base.name}#base`,
    base: base.name,
    mutation: "none",
    document: base.document,
  });
  for (const rung of ladder(base.document)) {
    const document = clone(base.document);
    try {
      rung.apply(document);
    } catch {
      // A rung that does not apply to this base (an empty collection has no
      // `.0`) is not a corpus entry; the ladder is shared across both bases.
      continue;
    }
    if (canonicalJson(document) === canonicalJson(base.document)) continue;
    corpus.push({
      id: `${base.name}#${rung.name}`,
      base: base.name,
      mutation: rung.name,
      document,
    });
  }
}

const entries = corpus.map((entry) => {
  const result = validateAssurance(entry.document);
  return { ...entry, oracle_valid: result.ok };
});

const revision = execFileSync("git", ["-C", repo, "rev-parse", "HEAD"], {
  encoding: "utf8",
}).trim();

writeFileSync(
  join(here, "..", "tests", "goldens", "assurance-verdicts.json"),
  canonicalJson({
    provenance: {
      producer: "rust/crates/quoin-quire/oracle/capture-assurance-verdicts.mjs",
      oracle: "src/quire/validate.ts:99 validateAssurance",
      ajv_version: JSON.parse(
        readFileSync(
          createRequire(import.meta.url).resolve("ajv/package.json"),
          "utf8",
        ),
      ).version,
      node_version: process.version,
      bases: "rust/crates/quoin-quire/tests/goldens/assurance-bases.json",
      bases_producer: captured.provenance.producer,
      engine: captured.provenance.engine,
      quoin_revision: revision,
    },
    entries,
  }),
  "utf8",
);
const refused = entries.filter((e) => !e.oracle_valid).length;
console.log(
  `wrote ${entries.length} entries (${entries.length - refused} accepted, ${refused} refused) at ${revision}`,
);
