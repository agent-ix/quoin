#!/usr/bin/env node
// The tier-1 benchmark runner (agent-ix/quoin#199, FR-043-AC-2/AC-7).
//
// `buildBenchCorpora` had no production caller. The corpora were built only by
// a unit test, into a temp directory, and scored by nobody — so every metric in
// `bench/metrics.json` that depends on them carried `baseline: null` with the
// note "No tier-1 run has been scored against a toolchain yet."
//
// This is that runner. It reads STATIC cases from the `agent-ix/qa-corpus`
// submodule at `corpus/`, runs the real tools over each in place, maps their
// payloads to findings through the committed table in
// `bench/tier1-mapping.json`, and scores them against the labels in
// `corpus/labels/`.
//
// It used to GENERATE the corpora into a tmpdir from 550 lines of JavaScript
// (agent-ix/quoin#227). Nothing was on disk to inspect, diff or `cd` into, and
// the generator declared its own synthetic module — `section: Test Cases` where
// the ecosystem declares `Test Case Summary`. A corpus whose manifest heading
// always matches CANNOT EXHIBIT the defect that accounts for 3,514 unminted TC
// ids across 88 repositories, which is why tier 1 never caught the dominant
// ecosystem failure mode. That was a correctness defect in the benchmark.
//
//   node scripts/bench-tier1.mjs                  # score and diff
//   node scripts/bench-tier1.mjs --update         # deliberate re-baseline
//   node scripts/bench-tier1.mjs --json           # the score, machine-readable
//   node scripts/bench-tier1.mjs --modules <dir>  # score against ANOTHER declaration
//
// THE DECLARATION IS AN AXIS, NOT A FIXED INPUT (agent-ix/quoin#240). This
// runner used to vary exactly one thing — the binary passed to `--quire` — and
// the traceability declaration every case binds was whatever the corpus
// vendored at its pinned SHA. Two of EPIC quire-rs#264's Wave 3 fixes are
// declaration-side (`spec-artifacts-process` #68 and #69), so an engine-only
// before/after reports them `held` BY CONSTRUCTION, in the same word it prints
// for a family that genuinely did not move. `--modules` holds the engine fixed
// and moves the declaration; `provenance.declaration` records which one every
// number was scored against.
//
// Ratchet semantics are quire-rs `scripts/bench.py`'s, deliberately: a closed
// metric dictionary that refuses an undeclared name, a one-way compare where a
// regression keeps the OLD baseline so a bad run can never lower the bar, and
// an unreadable metric OMITTED rather than reported as 0.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { parse as parseYaml } from "yaml";
import { crossCheckFamilies, loadMetrics } from "../evals/lib/dictionary.mjs";
import { scoreActionability, scoreFindings } from "../evals/lib/quality.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const MAPPING = join(ROOT, "bench", "tier1-mapping.json");
const METRICS = join(ROOT, "bench", "metrics.json");
const BASELINE = join(ROOT, "bench", "tier1-baseline.json");

/**
 * The engine metric quire-rs#270 added, declared in `bench/metrics.json`.
 *
 * Named here rather than inlined because its ABSENCE is the signal that
 * matters: an engine that predates #270 emits no such metric, and the report
 * must say `null` for it rather than 0 — not-measured and zero are different
 * claims about the same run.
 */
const SECTION_HIT_RATE = "minting.section_hit_rate";

/**
 * The bracketed reason a `validate` finding ends with, and the path and line it
 * opens with.
 *
 * Parsed rather than read from a field because `validate` has no JSON payload —
 * `--diagnostics-format json` wraps the whole human string in `message`. The
 * fragility is declared in `bench/tier1-mapping.json` and tracked as
 * agent-ix/quire-cli#65; a message this cannot parse is an ERROR here, never a
 * skip, because a family that silently stops scoring looks identical to a
 * family with nothing to report.
 */
const VALIDATE_LINE =
  /^(?<path>.+?): line (?<line>\d+): (?<rest>.*) \[(?<reason>[a-z-]+)\]$/;

/** Run a command, returning stdout, stderr and whether it exited zero. */
function run(bin, args, extraEnv) {
  try {
    const stdout = execFileSync(bin, args, {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
      stdio: ["ignore", "pipe", "pipe"],
      env: extraEnv ? { ...process.env, ...extraEnv } : process.env,
    });
    return { ok: true, stdout, stderr: "" };
  } catch (error) {
    // A non-zero exit is normal here: `validate` exits 1 when a document fails,
    // which is exactly the case being measured. The output is what matters.
    return {
      ok: false,
      stdout: String(error.stdout ?? ""),
      stderr: String(error.stderr ?? error.message ?? ""),
    };
  }
}

/**
 * Every finding one corpus produced, already mapped to a family — and the raw
 * `metrics[]` block the same `quire coverage` run emitted.
 *
 * The metrics come back UNFILTERED alongside the findings because a metric can
 * matter to the benchmark without belonging to a family. `minting.section_hit_rate`
 * is the measured case: quire-rs#270 added it to every case's payload and the
 * runner had nowhere to put it, so the one Wave 3 signal that reached this
 * corpus was read, dropped, and reported as "nothing changed"
 * (agent-ix/quoin#236).
 */
function findingsFor(quire, corpusRoot, module, mapping) {
  const out = [];
  const bySource = (name) =>
    Object.entries(mapping.families).filter(([, m]) => m.source === name);

  // ── quire coverage ──
  //
  // A module id names either ONE module (`manifest.yaml` directly) or a module
  // PATH — a directory of module directories. The ecosystem declaration is the
  // second: `spec-artifacts-process` carries the traceability model and
  // `spec-artifacts-iso` declares FR/NFR/TestMatrix, and loading only the first
  // leaves criteria classification silently producing nothing while the totals
  // look identical (agent-ix/quire-rs#292). `--module` takes one directory, so
  // a path goes through the search-path env var.
  const single = existsSync(join(module, "manifest.yaml"));
  const args = single
    ? ["coverage", "--scope", corpusRoot, "--module", module, "--json"]
    : ["coverage", "--scope", corpusRoot, "--json"];
  const env = single ? undefined : { IX_FILAMENT_MODULES_PATH: module };
  const cov = run(quire, args, env);
  let payload = null;
  try {
    payload = JSON.parse(cov.stdout);
  } catch {
    throw new Error(
      `bench-tier1: \`quire coverage\` produced no JSON for ${corpusRoot}. ` +
        `Refusing to score a corpus whose payload could not be read — an ` +
        `unreadable run and a clean run are the same zero.\n${cov.stderr.trim()}`,
    );
  }

  for (const [family, m] of bySource("coverage.diagnostics")) {
    for (const d of payload.diagnostics ?? []) {
      if (d.reason !== m.key) continue;
      out.push({ family, reason: d.reason, path: d.path ?? null, line: null });
    }
  }
  for (const [family, m] of bySource("coverage.suspicions")) {
    for (const s of payload.suspicions ?? []) {
      if (s.kind !== m.key) continue;
      out.push({
        family,
        reason: s.kind,
        path: s.path ?? null,
        line: typeof s.line === "number" ? s.line : null,
      });
    }
  }
  // A metric is a finding only when it sits at the value a label expects. The
  // metric MOVING is the signal, so the expectation lives on the label and this
  // stage carries the observed value for the caller to compare.
  const metrics = new Map((payload.metrics ?? []).map((m) => [m.name, m]));
  for (const [family, m] of bySource("coverage.metrics")) {
    const metric = metrics.get(m.key);
    if (!metric || metric.state !== "measured") continue;
    out.push({
      family,
      reason: m.key,
      path: null,
      line: null,
      metric: m.key,
      value: Number(metric.value),
    });
  }

  // ── quire validate ──
  const val = run(
    quire,
    single
      ? [
          "validate",
          "--diagnostics-format",
          "json",
          "--scope",
          corpusRoot,
          "--module",
          module,
          "spec/*.md",
        ]
      : [
          "validate",
          "--diagnostics-format",
          "json",
          "--scope",
          corpusRoot,
          "spec/*.md",
        ],
    env,
  );
  const wanted = new Set(bySource("validate.findings").map(([, m]) => m.key));
  const familyOfReason = new Map(
    bySource("validate.findings").map(([family, m]) => [m.key, family]),
  );
  for (const raw of val.stderr.split("\n")) {
    const text = raw.trim();
    if (!text.startsWith("{")) continue;
    let record;
    try {
      record = JSON.parse(text);
    } catch {
      continue;
    }
    if (record.kind !== "ValidationError") continue;
    const parsed = VALIDATE_LINE.exec(record.message);
    if (!parsed) {
      // Not a locatable finding (the run summary is a ValidationError too).
      // Only refuse when it LOOKS like a finding: `<path>: line N:`.
      if (/^.+: line \d+:/.test(record.message)) {
        throw new Error(
          `bench-tier1: could not parse a validate finding's path, line and ` +
            `reason from:\n  ${record.message}\n` +
            `The message format is the only place they exist ` +
            `(agent-ix/quire-cli#65). Refusing to skip it — a family that ` +
            `silently stops scoring reads as a family with nothing to report.`,
        );
      }
      continue;
    }
    const { path, line, reason } = parsed.groups;
    if (!wanted.has(reason)) continue;
    // A family MAY narrow past the reason token with `contains`. `assert` is
    // the validator's CATEGORY and every ecosystem-bound fixture emits three
    // shared structural ones; without this the family absorbs all of them and
    // its precision measures this table rather than the detector.
    const narrow = mapping.families[familyOfReason.get(reason)]?.contains;
    if (narrow && !record.message.includes(narrow)) continue;
    out.push({
      family: familyOfReason.get(reason),
      reason,
      path,
      line: Number(line),
    });
  }

  return { findings: out, metrics: payload.metrics ?? [] };
}

/**
 * Every labelled case in the corpus: its `case.yaml`, its input directory, and
 * the adjudicated defects from `corpus/labels/<id>.yaml`.
 *
 * Read from disk, in place. A case with no label file is a case that seeds no
 * defect — the clean control — and is still SCORED, because a detector that
 * fires on healthy input is exactly what precision is for.
 */
export function loadCorpus(
  mapping = null,
  root = join(ROOT, "corpus"),
  modulesRoot = null,
) {
  // THE DECLARATION AXIS (agent-ix/quoin#240). A case names a module by a
  // relative id (`ecosystem`, `variants/bench-legacy`); which TREE that id
  // resolves in is a variable of the run, so the same 34 cases can be scored
  // against a pre-fix and a post-fix declaration with the engine held fixed.
  // Defaults to the corpus's own `modules/`, so an ordinary run is unchanged.
  modulesRoot ??= join(root, "modules");
  // The mapping resolves a diagnostic reason to the family it belongs to, so a
  // case's own `expect.yaml` can state ground truth without restating the
  // family. Optional so a caller that only wants the case list (a test
  // asserting corpus shape) need not load it.
  mapping ??= existsSync(MAPPING)
    ? JSON.parse(readFileSync(MAPPING, "utf8"))
    : { families: {} };
  const casesRoot = join(root, "cases");
  if (!existsSync(casesRoot)) {
    throw new Error(
      `bench-tier1: the corpus submodule is not checked out at ${root}. ` +
        `Run \`git submodule update --init\`.`,
    );
  }
  const corpora = [];
  for (const mode of readdirSync(casesRoot).sort()) {
    const modeDir = join(casesRoot, mode);
    for (const name of readdirSync(modeDir).sort()) {
      const dir = join(modeDir, name);
      const caseYaml = join(dir, "case.yaml");
      if (!existsSync(caseYaml)) continue;
      const meta = parseYaml(readFileSync(caseYaml, "utf8"));
      const labelPath = join(root, "labels", `${meta.id}.yaml`);
      const label = existsSync(labelPath)
        ? parseYaml(readFileSync(labelPath, "utf8"))
        : null;
      const expectPath = join(dir, "expect.yaml");
      const expect = existsSync(expectPath)
        ? (parseYaml(readFileSync(expectPath, "utf8")) ?? {})
        : {};
      assertReasonsMapped(meta, expect, mapping, join("cases", mode, name));

      corpora.push({
        name: meta.id,
        family: label?.family ?? familyOf(meta, expect, mapping),
        summary: label?.summary ?? meta.comment ?? "",
        defects: label?.defects ?? defectsFrom(meta, expect, mapping),
        input: join(dir, "input"),
        module: assertModule(join(modulesRoot, meta.module), meta, modulesRoot),
        // The corpus's own declaration of what language the case is written
        // in, carried into the report so a `held` verdict cannot be read as
        // "verified in every language". At pin 088771b all 22 cases said
        // `rust`, and two of Wave 3's six fixes were for the other two
        // (agent-ix/quoin#236).
        language: meta.language ?? "unknown",
        pending: meta.pending ?? null,
      });
    }
  }
  return { corpora, modulesRoot };
}

/**
 * Refuse a module id that resolves to nothing in the declaration root in use.
 *
 * WITHOUT THIS, A MISTYPED `--modules` IS A SILENT ZERO. `findingsFor` decides
 * between `--module <dir>` and `IX_FILAMENT_MODULES_PATH=<dir>` by asking
 * whether the directory holds a `manifest.yaml`; a directory that does not
 * exist answers "no", the env var points at nothing, the engine registers no
 * archetype, and every case reports an empty payload. The run then looks like a
 * total detection collapse rather than a wrong path — the exact confusion
 * agent-ix/quire-rs#292 records, where vendoring one module of two left
 * criteria classification silently producing nothing.
 */
function assertModule(dir, meta, modulesRoot) {
  const usable =
    existsSync(join(dir, "manifest.yaml")) ||
    (existsSync(dir) &&
      readdirSync(dir).some((child) =>
        existsSync(join(dir, child, "manifest.yaml")),
      ));
  if (usable) return dir;
  throw new Error(
    `bench-tier1: case \`${meta.id}\` binds module \`${meta.module}\`, which ` +
      `resolves to ${dir} under the declaration root ${modulesRoot} and holds ` +
      `no \`manifest.yaml\` — neither directly nor in any child. Refusing to ` +
      `score: the engine would register no archetype, every case would report ` +
      `an empty payload, and the run would read as a detection collapse ` +
      `rather than a missing declaration (agent-ix/quoin#240).`,
  );
}

/** Every file under `dir`, as paths relative to it, in a stable order. */
function walkFiles(dir, base = dir) {
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    if (name === ".git" || name === "__pycache__") continue;
    const full = join(dir, name);
    if (statSync(full).isDirectory()) out.push(...walkFiles(full, base));
    else out.push(relative(base, full).split(sep).join("/"));
  }
  return out;
}

/** A content digest over a declaration tree: path and bytes, nothing else. */
function digestOf(dir) {
  const hash = createHash("sha256");
  for (const rel of walkFiles(dir)) {
    hash.update(rel);
    hash.update("\0");
    hash.update(
      createHash("sha256")
        .update(readFileSync(join(dir, rel)))
        .digest(),
    );
    hash.update("\n");
  }
  return `sha256:${hash.digest("hex")}`;
}

/**
 * The upstream SHAs a vendored declaration records for itself.
 *
 * The corpus vendors COPIES of the ecosystem's declaring modules and records
 * where each came from in a `VENDORED.md` table — that file is the corpus's own
 * provenance and the only place the upstream SHA exists, since a copy carries no
 * git identity of its own. Parsed rather than assumed, and a `VENDORED.md` that
 * yields no row is an ERROR: a provenance file that has silently stopped being
 * readable is worse than none, because the report would keep printing a
 * confident `sources: {}` beside numbers nobody could join to a commit.
 */
function vendoredSources(modulesRoot) {
  const out = {};
  let files = 0;
  const visit = (dir, depth) => {
    if (depth > 2 || !existsSync(dir)) return;
    for (const name of readdirSync(dir).sort()) {
      const full = join(dir, name);
      if (name === "VENDORED.md") {
        files += 1;
        let rows = 0;
        for (const line of readFileSync(full, "utf8").split("\n")) {
          const cells = line
            .split("|")
            .slice(1, -1)
            .map((c) => c.trim().replace(/`/g, ""));
          if (cells.length !== 3) continue;
          if (!/^[0-9a-f]{7,40}$/.test(cells[2])) continue;
          out[cells[0]] = cells[2];
          rows += 1;
        }
        if (!rows) {
          throw new Error(
            `bench-tier1: ${full} records no \`| module | path | sha |\` row ` +
              `this runner can read, so the declaration's upstream SHA cannot ` +
              `be recorded. Refusing to score a declaration whose provenance ` +
              `file is present and unreadable (agent-ix/quoin#240).`,
          );
        }
      } else if (statSync(full).isDirectory()) visit(full, depth + 1);
    }
  };
  visit(modulesRoot, 0);
  return files ? out : null;
}

/**
 * WHAT DECLARATION THIS RUN WAS SCORED AGAINST (agent-ix/quoin#240).
 *
 * The report already said which ENGINE and which CORPUS produced it. The third
 * input was invisible, and it is the one two of Wave 3's six fixes live in — so
 * two reports taken either side of a declaration change were silently
 * comparable, and a declaration-side fix scored `held` in the same word used
 * for a fix that did nothing.
 *
 * `digest` is measured from the bytes and is the comparable key; `sources` is
 * the upstream SHA the corpus records for each vendored module, which is what a
 * reader needs to fetch the diff. Both, because a digest cannot be looked up
 * and a recorded SHA cannot be verified.
 */
export function declarationProvenance(modulesRoot, bound = []) {
  const inRepo = !relative(ROOT, modulesRoot).startsWith("..");
  const paths = {};
  for (const id of [...new Set(bound)].sort()) {
    paths[id] = digestOf(join(modulesRoot, id));
  }
  return {
    root: inRepo
      ? relative(ROOT, modulesRoot).split(sep).join("/")
      : modulesRoot,
    digest: digestOf(modulesRoot),
    modules: paths,
    sources: vendoredSources(modulesRoot),
  };
}

/** The family that claims a diagnostic reason, or `null`. */
function familyForReason(mapping, reason) {
  const entry = Object.entries(mapping?.families ?? {}).find(
    ([, m]) => m.key === reason,
  );
  return entry ? entry[0] : null;
}

/**
 * Refuse a case whose own `expect.yaml` names a reason no family claims.
 *
 * THE DEFECT THIS FIXES (agent-ix/quoin#236): `defectsFrom` used to `continue`
 * past an unrecognised reason, so `cases/minting/section-name-mismatch` —
 * whose only expectation is `section-matches-nothing` — derived ZERO defects.
 * It was also marked `pending: agent-ix/quire-rs#270`, and the FR-065 check
 * that must fail when a pending case starts passing reads those derived
 * defects: with none to read it returned before running the engine. The fix
 * landed in quire-rs `a6a1144`, the engine emitted the diagnostic on every run
 * from then on, the marker went stale, and the benchmark's own guard said
 * nothing. Three silent skips in a row, each individually defensible.
 *
 * Checked for EVERY case, not only unlabelled ones: a `labels/` file supplies
 * `defects` and short-circuits `defectsFrom` entirely, so validating inside it
 * would leave the labelled half of the corpus unguarded.
 *
 * The escape hatch is the one the table already has. A reason no detector
 * produces yet gets a family with `source: none`, the way
 * `gate-that-gates-nothing` does — a DECLARED hole, reviewable in the mapping,
 * rather than an absence a reader has to infer from a row that never appears.
 */
function assertReasonsMapped(meta, expect, mapping, where) {
  const unmapped = (expect.diagnostic_reasons ?? []).filter(
    (reason) => !familyForReason(mapping, reason),
  );
  if (!unmapped.length) return;
  throw new Error(
    `bench-tier1: ${where} (${meta.id}) expects diagnostic reason` +
      `${unmapped.length === 1 ? "" : "s"} ` +
      unmapped.map((r) => `\`${r}\``).join(", ") +
      ` that no family in bench/tier1-mapping.json claims. Refusing to score ` +
      `a case whose own ground truth maps to nothing: the finding would be ` +
      `read out of the payload, matched against no family, and vanish — and ` +
      `a family that silently stops scoring reads exactly like a family with ` +
      `nothing to report. Declare an owning family (use \`source: none\` when ` +
      `no detector exists yet, as \`gate-that-gates-nothing\` does), or drop ` +
      `the reason from expect.yaml (agent-ix/quoin#236).`,
  );
}

/**
 * The ground truth a case states about itself, for a case with no label file.
 *
 * THE DEFECT THIS FIXES: the first version treated "no label file" as "seeds no
 * defect", so every case quoin had not separately adjudicated was scored as
 * healthy input — and every CORRECT detection on it counted as a false
 * positive. Fourteen of twenty-two cases were unlabelled and **eight of those
 * declare `kind: failure`**, with an `expect.yaml` naming the exact diagnostics
 * quire emits. The reported precision collapse (1.0 -> 0.083) was the benchmark
 * penalising the engine for being right, against ground truth sitting unread in
 * the same submodule.
 *
 * `case.yaml`'s `kind`/`findable` and `expect.yaml`'s `diagnostic_reasons` are
 * the corpus's own statement of what should be found. A separate `labels/` file
 * adds quoin's finer adjudication — location, collateral, `expect_metric` — and
 * wins where it exists.
 */
function defectsFrom(meta, expect, mapping) {
  if (meta.kind !== "failure") return [];
  // ONE defect per case, not one per expected reason. A case isolates a single
  // family by construction (a mini-repo mixing three defects cannot tell you
  // which one a finding was about) — and the SECOND reason a seeded defect
  // produces is COLLATERAL, which the runner already models. Deriving both as
  // seeded defects broke the corpus's own isolation property, which is what
  // `every corpus isolates ONE defect family` caught.
  for (const reason of expect.diagnostic_reasons ?? []) {
    // `assertReasonsMapped` has already refused the case if any reason here
    // maps to nothing, so this cannot be null — and it USED to be a silent
    // `continue`, which is the whole of agent-ix/quoin#236.
    const family = familyForReason(mapping, reason);
    return [
      {
        // The corpus's id convention — two initials and an ordinal — made
        // unique across the whole set, which `defect ids are unique across the
        // whole corpus set` caught: `marker-form-mismatch` and
        // `marker-form-declared` both initialise to `MF`.
        id: `${initials(meta.id)}-${ordinal(meta.id)}`,
        family,
        location: expect.diagnostic_paths?.[reason] ?? null,
        findable: meta.findable !== false,
        expect_reason: reason,
        confirmed_at: "derived from the case's own expect.yaml",
        // The REMAINING expected reasons are collateral, not second seeded
        // defects. Making one fire makes the others fire by construction — an
        // unreadable marker makes `coverage.backed` a ratio over an unread
        // population, so `no-symbol-bound` and `hollow-denominator` are one
        // situation seen through two lenses. Dropping them (the first fix for
        // the one-family-per-case violation) left each correct collateral
        // firing counted as a false positive.
        collateral: (expect.diagnostic_reasons ?? [])
          .filter((r) => r !== reason)
          .map((r) => {
            const collateralFamily = familyForReason(mapping, r);
            return collateralFamily
              ? {
                  family: collateralFamily,
                  reason: r,
                  // Required by TC-950: collateral suppresses a finding from
                  // the precision denominator, so a declaration with no stated
                  // reason is a licence to launder false positives. This one is
                  // DERIVED rather than adjudicated, and says so — the case
                  // states both reasons in its own `expect.yaml`, so both are
                  // expected consequences of the one situation it seeds, but
                  // nobody has written down why they co-occur.
                  note:
                    `derived: the case's own expect.yaml expects both ` +
                    `\`${reason}\` and \`${r}\`, so this is a consequence of ` +
                    `the one defect it seeds, not a second seeded defect. Not ` +
                    `separately adjudicated.`,
                }
              : null;
          })
          .filter(Boolean),
        note:
          "Derived, not adjudicated: the corpus states in `expect.yaml` what " +
          "should be found, and an unlabelled failure case is NOT healthy " +
          "input. Treating it as such counted every correct detection on it " +
          "as a false positive (agent-ix/quoin#227 review).",
      },
    ];
  }
  return [];
}

/**
 * A stable ordinal for a case id, so two cases sharing initials do not share a
 * defect id. Derived from the id rather than a counter: a counter depends on
 * walk order, and two runners walking differently would disagree about which
 * defect is which.
 */
function ordinal(id) {
  let hash = 0;
  for (const ch of id) hash = (hash * 31 + ch.charCodeAt(0)) % 997;
  return hash;
}

/** `marker-form-mismatch` -> `MF`. The corpus's defect-id convention. */
function initials(id) {
  const parts = id.split(/[^a-z0-9]+/i).filter(Boolean);
  const letters =
    (parts[0]?.[0] ?? "X") + (parts[1]?.[0] ?? parts[0]?.[1] ?? "X");
  return letters.toUpperCase();
}

/**
 * The family a case belongs to, when quoin has no label file for it.
 *
 * Taken from the first of its own expected reasons the mapping recognises.
 * `none` for a control, and for a failure case whose expectations name nothing
 * any family claims — the honest answer, since a family this benchmark does not
 * govern is one it cannot score.
 */
function familyOf(meta, expect, mapping) {
  if (meta.kind !== "failure") return "none";
  for (const reason of expect.diagnostic_reasons ?? []) {
    const family = familyForReason(mapping, reason);
    if (family) return family;
  }
  return "none";
}

/** Flatten the labels into the flat array scoring takes. */
export function flattenLabels(labels) {
  // The shape mismatch that kept `buildBenchCorpora` and `scoreFindings` from
  // ever meeting: the builder writes a wrapper keyed by corpus, the scorer
  // consumes a flat list. Converted in ONE place so the two cannot drift.
  return labels.corpora.flatMap((c) =>
    c.defects.map((d) => ({ ...d, corpus: c.name })),
  );
}

/**
 * The fraction of true positives whose reported location matches the label's.
 *
 * The metric that encodes the actual requirement: an alert must say WHERE.
 * `scoreFindings` already counts positional pairings; this is their share of
 * the confirmed findings, and `null` — never 0 — when nothing was confirmed,
 * because 0/0 is not 0%.
 */
export function localisationRate(score) {
  const truePositives = score.families.reduce((n, f) => n + f.truePositives, 0);
  if (truePositives === 0) return null;
  return Number((score.positional / truePositives).toFixed(3));
}

/**
 * The same score, cut by the language each case declares (agent-ix/quoin#236).
 *
 * Not a second measurement: the findings and labels are the ones the whole-run
 * score already used, partitioned by the case they came from. So the per-language
 * rows always sum to the corpus the headline was computed over, and a reader can
 * see WHICH language a family's recall came from.
 *
 * This exists because "held" was read as "verified" over a corpus that was 22
 * of 22 Rust. A family at recall 1.00 in Rust and with no case at all in Python
 * is not a family that works; it is a family nobody has asked the question of,
 * and the two look identical in a single table.
 */
export function byLanguage(corpora, findings, labels, shapes) {
  const languages = [...new Set(corpora.map((c) => c.language))].sort();
  return languages.map((language) => {
    const names = new Set(
      corpora.filter((c) => c.language === language).map((c) => c.name),
    );
    const mine = findings.filter((f) => names.has(f.corpus));
    const theirs = labels.filter((l) => names.has(l.corpus));
    return {
      language,
      corpora: names.size,
      families: scoreFindings(mine, theirs, shapes).families,
    };
  });
}

/**
 * quire-rs `bench.py`'s `compare`, in JS and with its semantics intact.
 *
 * A regression keeps the OLD baseline, so a bad run can never implicitly lower
 * the bar; `gate-zero` carries no baseline and no tolerance, so `--update`
 * cannot launder it; a missing baseline is `new`, never a pass by default.
 */
export function compare(direction, observed, baseline) {
  if (direction === "gate-zero") {
    return [observed === 0 ? "held" : "regressed", 0];
  }
  if (baseline === null || baseline === undefined) return ["new", observed];
  const better =
    direction === "higher-is-better"
      ? observed > baseline
      : observed < baseline;
  if (better) return ["improved", observed];
  if (observed === baseline) return ["held", baseline];
  return ["regressed", baseline];
}

/**
 * Whether two reports are over LIKE inputs, and which field says they are not.
 *
 * THE COMPARISON THIS REFUSES ALMOST GOT PUBLISHED. The previous pass scored
 * engine `84740d4` over a 34-case corpus against a baseline written over 21
 * cases; every family read `regressed`, and nothing in the output said the
 * population had grown by 13. That is EPIC exit criterion 6's "refuses deltas
 * across unlike definitions or populations" and agent-ix/quoin#231's
 * unimplemented clause, and quoin#240 adds the third field it has to cover.
 *
 * The ENGINE is deliberately not a reason: varying the engine and comparing is
 * what this benchmark is for. The CORPUS, the DECLARATION and the POPULATION
 * are the inputs a delta is only meaningful when they are held.
 *
 * A baseline that records nothing for a field cannot make a run incomparable —
 * it is UNKNOWN, and returned separately so the run can say so out loud rather
 * than either refusing every legacy baseline or quietly assuming it matched.
 */
export function comparability(report, previous) {
  const reasons = [];
  const unknown = [];
  const check = (field, mine, theirs) => {
    if (theirs === undefined || theirs === null) return unknown.push(field);
    const a = JSON.stringify(mine);
    const b = JSON.stringify(theirs);
    if (a !== b) reasons.push({ field, baseline: theirs, observed: mine });
  };
  const languages = (r) =>
    Object.fromEntries(
      (r.by_language ?? []).map((l) => [l.language, l.corpora]),
    );
  if (!previous) return { comparable: true, reasons, unknown };
  check(
    "provenance.declaration.digest",
    report.provenance?.declaration?.digest ?? null,
    previous.provenance?.declaration?.digest ?? null,
  );
  check(
    "provenance.corpus",
    report.provenance?.corpus ?? null,
    previous.provenance?.corpus ?? null,
  );
  check("corpora", report.corpora ?? null, previous.corpora ?? null);
  check(
    "by_language",
    languages(report),
    previous.by_language ? languages(previous) : null,
  );
  return { comparable: reasons.length === 0, reasons, unknown };
}

/**
 * Refuse a binary that cannot say what it is, or lacks what the mapping reads.
 *
 * The `engine` provenance block arrived with agent-ix/quire-cli#68 precisely so
 * a saved number can name the build that produced it. A binary without one
 * predates it and cannot be scored against.
 */
function assertEngine(quire) {
  const probe = run(quire, ["--version"]);
  const line = (probe.stdout || "").trim();
  if (!line.includes("engine")) {
    throw new Error(
      `bench-tier1: ${quire} reports "${line}" — no engine version, so it ` +
        `predates agent-ix/quire-cli#68 and cannot say which engine it links. ` +
        `Refusing to score.`,
    );
  }
  console.error(`bench-tier1: engine ${line}`);
  return line;
}

/**
 * The corpus revision this run read, or `null`.
 *
 * `null` and never a guess: a baseline that names the wrong corpus is worse
 * than one that names none, because the second cannot be believed and the
 * first can.
 */
function corpusRevision(root = join(ROOT, "corpus")) {
  const probe = run("git", ["-C", root, "rev-parse", "HEAD"]);
  return probe.ok ? probe.stdout.trim() : null;
}

function main() {
  const update = process.argv.includes("--update");
  const asJson = process.argv.includes("--json");
  // NOT a PATH lookup. `quire` on PATH is whatever somebody installed —
  // measured at 0.29.0 here, which pins engine v0.42.0 and predates
  // `binding_census`. Scored with it, EVERY coverage family reported recall 0
  // and the run looked like a corpus regression rather than a stale binary.
  // That is agent-ix/quire-rs#265's defect, a third repository over.
  const quire = argOf("--quire") ?? process.env.QUIRE;
  if (!quire) {
    throw new Error(
      "bench-tier1: pass --quire <path> or set QUIRE. Deliberately not a PATH " +
        "lookup: scoring a benchmark with an unidentified binary is the defect " +
        "this benchmark exists to catch.",
    );
  }
  const engine = assertEngine(quire);
  // The DECLARATION axis. Absolute or relative to the working directory, and
  // absent means the corpus's own vendored `modules/` — so the gate is
  // unchanged and only a run that asks scores a different declaration.
  const modules = argOf("--modules") ?? process.env.MODULES ?? null;

  const mapping = JSON.parse(readFileSync(MAPPING, "utf8"));
  const dictionary = loadMetrics(METRICS);

  const loaded = loadCorpus(
    mapping,
    join(ROOT, "corpus"),
    modules ? resolve(modules) : null,
  );
  let report;
  // Cases the corpus marks `pending` assert behaviour the engine does not
  // have yet. Scoring them counts a known-missing detector as a miss on every
  // run, which turns a deliberate red fixture into permanent noise in the
  // benchmark. They are excluded and NAMED, never silently dropped.
  const pending = loaded.corpora.filter((c) => c.pending);
  const labels = { corpora: loaded.corpora.filter((c) => !c.pending) };
  const flat = flattenLabels(labels);
  if (pending.length) {
    console.error(
      `bench-tier1: ${pending.length} case(s) excluded as pending a fix: ` +
        pending.map((c) => `${c.name} (${c.pending})`).join(", "),
    );
  }

  // FR-065: "The runner SHALL fail the run when a case declaring `pending`
  // passes." Excluding a pending case from SCORING is right — a known-missing
  // detector counted as a miss on every run turns a deliberate red into
  // permanent noise. Excluding it from CHECKING is not: without this, the fix
  // lands, the marker goes stale, and no score ever moves to say so.
  //
  // A pending case that states nothing the check can run is REFUSED, not
  // skipped. `if (!want.length) return false` was the second half of
  // agent-ix/quoin#236: `section-name-mismatch` derived no defects because its
  // one expected reason mapped to no family, so this returned before the
  // engine was ever invoked — the guard written for exactly this situation
  // passed a case whose fix had already shipped. A `pending:` marker with no
  // runnable expectation cannot expire, and a marker that cannot expire is
  // permanent.
  for (const c of pending) {
    if (c.defects.some((d) => d.expect_reason)) continue;
    throw new Error(
      `bench-tier1: pending case ${c.name} (${c.pending}) names no ` +
        `\`expect_reason\`, so the FR-065 stale-pending check has nothing to ` +
        `run and would pass it forever. State the diagnostic reason the ` +
        `ticket will make fire — in the case's own \`expect.yaml\` under ` +
        `\`diagnostic_reasons:\`, or in its \`labels/\` entry — or drop the ` +
        `\`pending:\` marker (agent-ix/quoin#236).`,
    );
  }
  const stale = pending.filter((c) => {
    const want = c.defects.map((d) => d.expect_reason).filter(Boolean);
    const { findings } = findingsFor(quire, c.input, c.module, mapping);
    return want.every((r) => findings.some((f) => f.reason === r));
  });
  if (stale.length) {
    throw new Error(
      `bench-tier1: ${stale.length} pending case(s) now PASS — ` +
        stale.map((c) => `${c.name} (${c.pending})`).join(", ") +
        `. The fix appears to have landed; remove \`pending:\` from case.yaml ` +
        `so the case is scored.`,
    );
  }

  // Both directions, before anything is scored: a declared family with no
  // corpus and a corpus family no metric governs are each a hole in the
  // score, and finding them after a run wastes the run.
  crossCheckFamilies(
    dictionary.families,
    labels.corpora.map((c) => c.family),
    { path: "bench/metrics.json" },
  );

  const found = [];
  // `matched` and `examined` summed across cases, NOT a mean of per-case
  // rates: a mean over cases weights a one-document case the same as a
  // twenty-document one, and this metric's denominator is documents.
  const sectionHit = { matched: 0, examined: 0, cases: 0 };
  for (const corpus of labels.corpora) {
    const { findings, metrics } = findingsFor(
      quire,
      corpus.input,
      corpus.module,
      mapping,
    );
    // The case a finding came from, and the language that case is written in.
    // Carried on the finding so the score can be cut per language without a
    // second run — `scoreFindings` ignores fields it does not read.
    found.push(
      ...findings.map((f) => ({
        ...f,
        corpus: corpus.name,
        language: corpus.language,
      })),
    );
    const hit = metrics.find((m) => m.name === SECTION_HIT_RATE);
    if (hit && hit.state === "measured") {
      sectionHit.cases += 1;
      sectionHit.matched += Number(hit.matched ?? 0);
      sectionHit.examined += Number(hit.examined ?? 0);
    }
  }

  // A metric-sourced finding counts only at the value its label expects; the
  // metric merely EXISTING says nothing. Filtered here rather than inside
  // `findingsFor` so the mapping stage stays a pure payload read.
  const expectedValues = new Map(
    flat
      .filter((l) => l.expect_metric !== undefined)
      .map((l) => [l.expect_metric, Number(l.expect_value)]),
  );
  const scoredFindings = found.filter(
    (f) => f.metric === undefined || expectedValues.get(f.metric) === f.value,
  );

  // The shape a family is scored under is declared in the mapping, beside the
  // key it reads — not inferred here, so a reader of the table can see why a
  // family reports no precision without tracing the scorer.
  const shapes = Object.fromEntries(
    Object.entries(mapping.families).map(([family, m]) => [
      family,
      m.shape ?? "defect",
    ]),
  );
  const score = scoreFindings(scoredFindings, flat, shapes);
  report = {
    // WHAT PRODUCED THIS. `bench/tier1-baseline.json` carried no engine and no
    // corpus revision, so which binary wrote any of its five git revisions had
    // to be reconstructed from `quire-cli`'s pin history by timestamp and then
    // confirmed by rebuilding — for a number the file states as fact. That is
    // the join agent-ix/quoin#229 is about, and it is the whole of it that a
    // report can supply on its own. Deterministic on purpose: no timestamp, so
    // two runs of the same engine over the same corpus are byte-identical.
    provenance: {
      engine,
      corpus: corpusRevision(),
      // THE THIRD INPUT, previously invisible (agent-ix/quoin#240). Without it
      // two reports taken either side of a `spec-artifacts-process` change are
      // silently comparable, and the declaration-side half of Wave 3 scores
      // `held` in the same word used for a fix that changed nothing.
      declaration: declarationProvenance(
        loaded.modulesRoot,
        labels.corpora.map((c) =>
          relative(loaded.modulesRoot, c.module).split(sep).join("/"),
        ),
      ),
    },
    families: score.families,
    excluded: score.excluded,
    collateral: score.collateral,
    positional: score.positional,
    finding_localisation_rate: localisationRate(score),
    // Declared in bench/metrics.json, REPORTED and never ratcheted: its value
    // is a property of the corpus population, so it would move when a fixture
    // was authored. `null` — not 0 — when no case reports it, which is what an
    // engine predating quire-rs#270 looks like.
    "minting.section_hit_rate": sectionHit.examined
      ? {
          rate: Number((sectionHit.matched / sectionHit.examined).toFixed(3)),
          matched: sectionHit.matched,
          examined: sectionHit.examined,
          cases_reporting: sectionHit.cases,
        }
      : null,
    actionability: scoreActionability(scoredFindings),
    corpora: labels.corpora.length,
    // The score, cut by the language each case declares. A single `held` table
    // over a corpus that is 100% one language reads as "verified everywhere"
    // and is not: at qa-corpus 088771b every one of the 22 cases said `rust`,
    // and Wave 3's python and TypeScript fixes could not be exercised at all
    // (agent-ix/quoin#236).
    by_language: byLanguage(labels.corpora, scoredFindings, flat, shapes),
    // In the REPORT, not only on stderr: a consumer piping stdout would
    // otherwise read a corpus count with nothing recording that a case exists
    // and was deliberately dropped.
    pending: pending.map((c) => ({ case: c.name, ticket: c.pending })),
    findings: scoredFindings.length,
  };

  const previous = existsSync(BASELINE)
    ? JSON.parse(readFileSync(BASELINE, "utf8"))
    : null;
  const verdicts = ratchet(report, previous, dictionary);
  // An UNKNOWN field is said out loud rather than assumed to have matched. A
  // baseline written before quoin#240 records no declaration, so a comparison
  // against it is resting on an assumption nobody stated — which is the whole
  // of what this ticket is about, one file over.
  const { unknown } = comparability(report, previous);
  if (unknown.length) {
    console.error(
      `bench-tier1: the baseline records no ${unknown.join(", ")}; this ` +
        `comparison ASSUMES those inputs did not move, and cannot check it.`,
    );
  }

  if (asJson) {
    console.log(JSON.stringify({ ...report, verdicts }, null, 2));
  } else {
    console.log(render(report, verdicts));
  }

  if (update) {
    writeFileSync(BASELINE, JSON.stringify(report, null, 2) + "\n");
    console.error(`bench-tier1: baseline rewritten at ${BASELINE}`);
    return 0;
  }
  // `incomparable` exits non-zero for the same reason `regressed` does: the
  // gate has not been met. It is NOT a claim that anything got worse — nothing
  // was compared — and the only way past it is a deliberate re-baseline, which
  // is what makes bumping the corpus or the declaration a reviewable act rather
  // than a silent change of subject.
  return verdicts.some(
    (v) => v.verdict === "regressed" || v.verdict === "incomparable",
  )
    ? 1
    : 0;
}

/** Per-family precision and recall against the baseline, one-way. */
export function ratchet(report, previous, dictionary) {
  const out = [];
  const before = new Map((previous?.families ?? []).map((f) => [f.family, f]));
  for (const family of report.families) {
    for (const metric of ["precision", "recall"]) {
      const declared = dictionary.metrics[`finding_${metric}`];
      // A metric this run could not read is OMITTED, never reported as 0 —
      // `null` precision means no denominator, and calling that a regression
      // would fail the build for a family nothing fired on.
      if (family[metric] === null) continue;
      const [verdict, kept] = compare(
        declared.direction,
        family[metric],
        before.get(family.family)?.[metric] ?? null,
      );
      out.push({
        metric: `finding_${metric}`,
        family: family.family,
        observed: family[metric],
        baseline: kept,
        verdict,
      });
    }
  }
  // A family the baseline scored and this run does not report AT ALL — its
  // corpus deleted, its mapping dropped, its label removed. Without this the
  // family simply vanishes from the table and the ratchet says nothing, so
  // deleting a corpus reads as a clean run. That is the same shape as a check
  // that cannot fail: the absence of a row is indistinguishable from an
  // absence of news.
  const now = new Set(report.families.map((f) => f.family));
  for (const f of previous?.families ?? []) {
    if (now.has(f.family)) continue;
    out.push({
      metric: "finding_recall",
      family: f.family,
      observed: null,
      baseline: f.recall,
      verdict: "regressed",
      why: "the baseline scored this family and this run did not report it at all",
    });
  }

  if (report.finding_localisation_rate !== null) {
    const [verdict, kept] = compare(
      "higher-is-better",
      report.finding_localisation_rate,
      previous?.finding_localisation_rate ?? null,
    );
    out.push({
      metric: "finding_localisation_rate",
      family: null,
      observed: report.finding_localisation_rate,
      baseline: kept,
      verdict,
    });
  }

  // NO DELTA ACROSS UNLIKE INPUTS. Computed after the verdicts rather than
  // instead of them, so the two numbers stay visible and only the CLAIM that
  // one moved is withdrawn: `improved` and `regressed` are statements about a
  // change, and a run over a different corpus, a different declaration or a
  // different population did not observe one.
  const { comparable, reasons } = comparability(report, previous);
  if (comparable) return out;
  const why =
    "not compared: " +
    reasons
      .map(
        (r) =>
          `${r.field} moved (baseline ${short(r.baseline)}, this run ${short(r.observed)})`,
      )
      .join("; ");
  return out.map((v) => ({ ...v, verdict: "incomparable", why }));
}

/** A digest or a language census, short enough to sit in a verdict line. */
function short(value) {
  if (typeof value === "string" && value.startsWith("sha256:")) {
    return `sha256:${value.slice(7, 19)}…`;
  }
  if (typeof value === "string" && /^[0-9a-f]{40}$/.test(value)) {
    return `${value.slice(0, 12)}…`;
  }
  return typeof value === "object" && value !== null
    ? JSON.stringify(value)
    : String(value);
}

const MARK = {
  improved: "++",
  held: "ok",
  new: "**",
  regressed: "!!",
  incomparable: "??",
};

function render(report, verdicts) {
  const pct = (v) =>
    v === null ? "  n/a" : `${Math.round(v * 100)}%`.padStart(5);
  const declaration = report.provenance.declaration;
  const lines = [
    `tier-1: ${report.corpora} corpora, ${report.findings} findings mapped` +
      ` (${report.by_language
        .map((l) => `${l.language} ${l.corpora}`)
        .join(", ")})`,
    // WHICH DECLARATION, on the same footing as which engine. Two of Wave 3's
    // six fixes changed nothing but this, and a reader of the score had no way
    // to tell which of the three inputs a run had moved (agent-ix/quoin#240).
    `engine      ${report.provenance.engine}`,
    `corpus      ${short(report.provenance.corpus)}`,
    `declaration ${declaration.root} ${short(declaration.digest)}` +
      (declaration.sources
        ? ` (${Object.entries(declaration.sources)
            .map(([name, sha]) => `${name} ${short(sha)}`)
            .join(", ")})`
        : " (no VENDORED.md: no upstream SHA recorded)"),
    "",
    "family                     TP  FP  miss   prec  recall",
  ];
  for (const f of report.families) {
    lines.push(
      `  ${f.family.padEnd(24)} ${String(f.truePositives).padStart(2)}  ` +
        `${String(f.falsePositives).padStart(2)}  ${String(f.misses).padStart(4)}  ` +
        `${pct(f.precision)}  ${pct(f.recall)}`,
    );
  }
  lines.push(
    "",
    `finding_localisation_rate  ${pct(report.finding_localisation_rate)} ` +
      `(${report.positional} of ${report.families.reduce((n, f) => n + f.truePositives, 0)} true positives named where)`,
    `actionability_rate         ${pct(report.actionability.rate)} ` +
      `(${report.actionability.actionable} of ${report.actionability.total} findings name a row or a line)`,
    `minting.section_hit_rate   ${
      report[SECTION_HIT_RATE] === null
        ? "  n/a (no case reports it — this engine predates quire-rs#270)"
        : `${pct(report[SECTION_HIT_RATE].rate)} (${report[SECTION_HIT_RATE].matched}` +
          ` of ${report[SECTION_HIT_RATE].examined} declared minting documents` +
          ` reached their section, over ${report[SECTION_HIT_RATE].cases_reporting} cases)`
    }`,
  );
  // Per language, because one table over a single-language corpus reads as a
  // statement about the toolchain and is a statement about Rust.
  lines.push("", "by language:");
  for (const lang of report.by_language) {
    lines.push(`  ${lang.language} (${lang.corpora} corpora)`);
    for (const f of lang.families) {
      lines.push(
        `    ${f.family.padEnd(24)} ${String(f.truePositives).padStart(2)}  ` +
          `${String(f.falsePositives).padStart(2)}  ${String(f.misses).padStart(4)}  ` +
          `${pct(f.precision)}  ${pct(f.recall)}`,
      );
    }
    if (!lang.families.length) {
      lines.push("    (no family scored a finding or a label here)");
    }
  }
  if (report.collateral.length) {
    lines.push(
      "",
      "declared collateral, set aside from scoring:",
      ...report.collateral.map((c) => `  ${c.family} (${c.reason})`),
    );
  }
  if (report.excluded.length) {
    lines.push(`excluded as not findable: ${report.excluded.join(", ")}`);
  }
  lines.push("", "ratchet:");
  // An `incomparable` run carries ONE reason, and printing it on all fifteen
  // rows buries the numbers it is there to protect. The rows say `not
  // compared`, the banner says why, and the JSON carries `why` per verdict so a
  // row read on its own is still self-describing.
  const wholeRun = verdicts.every((v) => v.verdict === "incomparable");
  for (const v of verdicts) {
    const name = v.family ? `${v.metric}[${v.family}]` : v.metric;
    const why = wholeRun && v.verdict === "incomparable" ? null : v.why;
    lines.push(
      `  ${MARK[v.verdict]} ${name.padEnd(40)} ${v.observed} (baseline ${v.baseline})` +
        (why
          ? ` — ${why}`
          : v.verdict === "incomparable"
            ? " — not compared"
            : ""),
    );
  }
  if (verdicts.some((v) => v.verdict === "incomparable")) {
    lines.push(
      "",
      "INCOMPARABLE — this run and the baseline are over unlike inputs, so NO",
      "DELTA WAS COMPUTED. Both values are printed above; neither `improved`",
      "nor `regressed` is claimed, because a change was not observed — the",
      "subject changed.",
      `  ${verdicts.find((v) => v.verdict === "incomparable").why}`,
      "Re-baseline deliberately with `make bench-tier1-update` once the new",
      "inputs are the ones the gate should track.",
    );
  } else if (verdicts.some((v) => v.verdict === "regressed")) {
    lines.push(
      "",
      "REGRESSED — the baseline is kept, never lowered by a bad run.",
    );
  } else if (verdicts.some((v) => v.verdict === "improved")) {
    lines.push(
      "",
      "improved — run `make bench-tier1-update` to move the baseline.",
    );
  }
  return lines.join("\n");
}

function argOf(flag) {
  const i = process.argv.indexOf(flag);
  return i === -1 ? null : process.argv[i + 1];
}

if (
  resolve(process.argv[1] ?? "") === resolve(fileURLToPath(import.meta.url))
) {
  process.exit(main());
}
