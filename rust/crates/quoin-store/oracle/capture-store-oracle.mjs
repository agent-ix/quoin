// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The TypeScript half of the cutover gate.
//
//   node --loader ts-node/esm oracle/capture-store-oracle.mjs \
//     --out /tmp/store-oracle.json <repo> [<repo> ...]
//
// For every `*.json` file under each repository's `spec/evidence`, records what
// the *oracle* computes: whether its strict reader accepts the file, the digest
// of its canonical serialization in the form that family uses, whether
// re-serializing reproduces the bytes on disk, and the canonical digest of
// every JSON node in the document. Every `output.bin` beside an attestation is
// digested as raw bytes.
//
// Nothing is asserted here. `quoin-store-replay --oracle <out>` compares.
//
// The capture is newline-delimited JSON, one record per line, written as each
// file is read. A single JSON document holding every node digest of every store
// in the ecosystem exhausted a 4 GB V8 heap; streaming keeps the working set to
// one file.
//
// The node walk is pre-order, with object members visited in RFC 8785 name
// order and array items by index. `quoin_store::replay::walk_nodes` walks
// identically; if the two walks ever disagree the replay reports a node-count
// divergence rather than silently comparing different things.
//
// `QUOIN_SRC_ROOT` overrides where the TypeScript is read from. It must be a
// checkout with `node_modules` installed.

import {
  appendFileSync,
  closeSync,
  openSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(
  process.env.QUOIN_SRC_ROOT ?? join(here, "..", "..", ".."),
);
const load = (path) => import(pathToFileURL(join(repoRoot, "src", path)).href);

const { blake3Hex, canonicalBytes, canonicalizeJcs, parseStrictJson } =
  await load("change-assurance/integrity.js");
const { canonicalJson } = await load("evidence/store.js");

const argv = process.argv.slice(2);
let out = null;
const repositories = [];
for (let index = 0; index < argv.length; index += 1) {
  if (argv[index] === "--out") {
    out = argv[index + 1];
    index += 1;
  } else {
    repositories.push(resolve(argv[index]));
  }
}
if (!out || repositories.length === 0) {
  process.stderr.write(
    "usage: capture-store-oracle.mjs --out <file> <repo> [<repo> ...]\n",
  );
  process.exit(2);
}

const encoder = new TextEncoder();

/** Every JSON node, pre-order, members in RFC 8785 name order. */
function walk(value, into) {
  into.push(value);
  if (Array.isArray(value)) {
    for (const item of value) walk(item, into);
  } else if (value !== null && typeof value === "object") {
    for (const name of Object.keys(value).sort()) walk(value[name], into);
  }
  return into;
}

function listFiles(root, into) {
  let entries;
  try {
    entries = readdirSync(root, { withFileTypes: true });
  } catch {
    return into;
  }
  for (const entry of [...entries].sort((a, b) => (a.name < b.name ? -1 : 1))) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) listFiles(path, into);
    else into.push(path);
  }
  return into;
}

writeFileSync(out, "", "utf8");
const handle = openSync(out, "a");
const emit = (record) =>
  appendFileSync(handle, `${JSON.stringify(record)}\n`, "utf8");

let jsonFileCount = 0;
let rawFileCount = 0;
let digestCount = 0;
let repositoriesWithStores = 0;
let serializationFailures = 0;

for (const repository of repositories) {
  const storeRoot = join(repository, "spec", "evidence");
  try {
    if (!statSync(storeRoot).isDirectory()) continue;
  } catch {
    continue;
  }
  repositoriesWithStores += 1;
  for (const path of listFiles(storeRoot, [])) {
    const suffix = relative(storeRoot, path).split("\\").join("/");
    const key = `${repository}/${suffix}`;
    if (path.endsWith(".json")) {
      const bytes = readFileSync(path);
      const record = { kind: "file", key, path: suffix };
      let value;
      try {
        value = parseStrictJson(bytes);
      } catch (error) {
        record.parsed = false;
        record.error = error instanceof Error ? error.message : String(error);
        emit(record);
        jsonFileCount += 1;
        continue;
      }
      record.parsed = true;
      const isChangeAssurance = suffix.split("/").includes("change-assurance");
      record.form = isChangeAssurance ? "jcs" : "pretty";
      // Serialization is guarded for the same reason parsing is. `sortKeys`
      // and both writers recurse, so a document the strict parser ACCEPTS can
      // still overflow the V8 stack on the way out — at roughly half the
      // parser's depth. Leaving this unguarded ends the process mid-stream and
      // leaves a truncated capture that looks, to a consumer, exactly like a
      // complete one for a smaller store. An entry that names its own failure
      // is what the replay needs in order to refuse.
      let serialized;
      try {
        serialized = encoder.encode(
          isChangeAssurance ? canonicalizeJcs(value) : canonicalJson(value),
        );
        record.serialization_digest = blake3Hex(serialized);
        record.round_trip_identical =
          serialized.byteLength === bytes.byteLength &&
          serialized.every((byte, index) => byte === bytes[index]);
        record.node_digests = walk(value, []).map((node) =>
          blake3Hex(canonicalBytes(node)),
        );
        digestCount += record.node_digests.length;
      } catch (error) {
        record.serialization_error =
          error instanceof Error ? error.message : String(error);
        record.node_digests = [];
        serializationFailures += 1;
      }
      emit(record);
      jsonFileCount += 1;
    } else if (path.endsWith("output.bin")) {
      emit({
        kind: "raw",
        key,
        path: suffix,
        digest: blake3Hex(readFileSync(path)),
      });
      rawFileCount += 1;
      digestCount += 1;
    }
  }
}

closeSync(handle);
process.stderr.write(
  `typescript oracle: ${repositoriesWithStores} stores, ${jsonFileCount} json files, ` +
    `${rawFileCount} raw outputs, ${digestCount} digests, ` +
    `${serializationFailures} serialization failures -> ${out}\n`,
);
if (serializationFailures > 0) {
  process.stderr.write(
    `typescript oracle: ${serializationFailures} file(s) parsed but could not be ` +
      `serialized; each carries a serialization_error entry and the replay will ` +
      `report them as unmatched rather than compare nothing\n`,
  );
}
