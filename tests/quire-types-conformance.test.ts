/**
 * FR-029-AC-13 — the TypeScript interfaces and the vendored coverage schema
 * describe the same shapes (TC-272).
 *
 * The vendored `coverage-v1.schema.json` carried `implements` /
 * `$defs/ImplementsRecord` since v0.39, while `src/quire/types.ts` only
 * gained the interface in PR #173 — a two-release type/schema drift,
 * acknowledged in a code comment, with no gate to stop the next one. The
 * contract tests validate payloads against the schema and never the
 * interfaces against the schema (agent-ix/quoin#179).
 *
 * The gate is the ticket's cheapest workable form, made two-sided:
 *
 * - Each sample below is typed `Required<Interface>`, so it MUST carry every
 *   interface field and CANNOT carry a key the interface lacks — that half is
 *   enforced by `tsc`, which the last test runs over this file, because
 *   vitest transforms types away and the build's dts diagnostics are
 *   non-fatal (quoin#180 is open on exactly that).
 * - At runtime, each sample's keys are compared against its `$defs` entry's
 *   `properties` keys, both directions. Sample keys ARE the interface keys
 *   (Required forces them all), so this is interface-vs-schema by proxy.
 * - The assembled payload is validated against the schema, so the per-key
 *   TYPES stay honest too, not just the key names.
 *
 * Red-verified both ways: deleting `form` from the schema's
 * ImplementsRecord fails the key comparison; deleting it from the interface
 * fails the tsc gate with TS2353 (object literal may only specify known
 * properties) on the sample.
 */

import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { readSchema, validateCoverage } from "../src/quire/index.js";
import type {
  BindingCensus,
  CatchAllCriterion,
  CoverageDiagnostic,
  CoverageReport,
  CoverageTotals,
  CriteriaCounts,
  EngineProvenance,
  GroundingCounts,
  GroupCounts,
  ImplementsRecord,
  MeasuredMetric,
  MintedTargetRecord,
  NoSymbolRow,
  Obligation,
  SharedTraceId,
  SharedTraceSymbol,
  StatusLie,
  Suspicion,
  UnbackedRow,
  UnboundSymbol,
  UndeclaredStatus,
  UntrackedSymbol,
  UnmatchedTag,
  VocabularyValueRecord,
} from "../src/quire/types.js";

const schema = readSchema("coverage-v1.schema.json") as {
  properties: Record<string, unknown>;
  $defs: Record<
    string,
    {
      properties?: Record<string, unknown>;
      oneOf?: { properties: Record<string, unknown> }[];
    }
  >;
};

// ── One sample per $defs entry, typed Required<Interface> ──
//
// `Required<T>` is the load-bearing part: an optional interface field left
// out of a sample would exempt itself from the key comparison, and this way
// the compiler refuses the omission.

const unbackedRow: Required<UnbackedRow> = {
  reference: "traces-to",
  document: "spec/tests.md",
  row_id: "TC-001",
  target_ids: ["TC-001"],
  line: 17,
};

const statusLie: Required<StatusLie> = {
  reference: "traces-to",
  document: "spec/tests.md",
  row_id: "TC-002",
  status: "✅",
  target_ids: ["TC-002"],
  line: 18,
};

const noSymbolRow: Required<NoSymbolRow> = {
  reference: "traces-to",
  document: "spec/tests.md",
  row_id: "TC-003",
  test_type: "Eval",
  target_ids: ["TC-003"],
  line: 19,
};

const undeclaredStatus: Required<UndeclaredStatus> = {
  reference: "traces-to",
  document: "spec/tests.md",
  row_id: "TC-004",
  status: "⚠️ scale evidence deferred",
  line: 20,
};

const implementsRecord: Required<ImplementsRecord> = {
  path: "src/lib.rs",
  symbol: "parse",
  trace_id: "FR-001",
  form: "rust-implements-line",
};

const untrackedSymbol: Required<UntrackedSymbol> = {
  path: "src/lib.rs",
  symbol: "orphan",
  trace_id: "TC-999",
  line: 42,
};

const mintedTargetRecord: Required<MintedTargetRecord> = {
  id: "TC-001",
  target: "test-case",
  document: "spec/tests.md",
  line: 17,
  backed: true,
};

const unmatchedTag: Required<UnmatchedTag> = {
  trace_id: "TC-999",
  language: "rust",
  path: "tests/parse.rs",
  line: 42,
  symbol: "parse_without_declared_form",
};

const sharedTraceSymbol: Required<SharedTraceSymbol> = {
  path: "tests/parse.rs",
  symbol: "tc_001_parses",
};

const sharedTraceId: Required<SharedTraceId> = {
  trace_id: "TC-001",
  symbols: [
    sharedTraceSymbol,
    { path: "tests/parse_more.rs", symbol: "tc_001_parses_again" },
  ],
};

const groupCounts: Required<GroupCounts> = {
  document: "spec/tests.md",
  target: "test-case",
  backed: 1,
  total: 2,
};

const criteriaCounts: Required<CriteriaCounts> = {
  document: "spec/functional/FR-001.md",
  archetype: "FR",
  criteria: 2,
  property_shaped: 1,
  by_property: { universal: 1, example: 1 },
  specific_shaped: 1,
  grounding: {
    invariant: {
      records: 1,
      domain: 1,
      precondition: 1,
      oracle: 1,
      all_three: 1,
    },
  },
  catch_all_example: { row_id: "FR-001-AC-2", line: 18 },
};

const groundingCounts: Required<GroundingCounts> = {
  records: 1,
  domain: 1,
  precondition: 1,
  oracle: 1,
  all_three: 1,
};

const catchAllCriterion: Required<CatchAllCriterion> = {
  row_id: "FR-001-AC-2",
  line: 18,
};

const coverageDiagnosticSurface: Required<CoverageDiagnostic> = {
  declaration: "test-case",
  reason: "uncatalogued-verification-method",
  message:
    "'Deferred' is neither a declared verification_catalog method id nor a declared class",
  path: null,
  line: 21,
  value: "Deferred",
  subject: "verification method `Deferred`",
  change_target: "spec/functional/FR-001.md:21",
  remedy: "add the method to the verification catalog",
  next_diagnostic_step: "inspect the authored method and catalog",
};

const coverageDiagnostic: CoverageDiagnostic = {
  declaration: "test-case",
  reason: "uncatalogued-verification-method",
  message:
    "'Deferred' is neither a declared verification_catalog method id nor a declared class",
  path: null,
  line: 21,
  value: "Deferred",
  subject: "verification method `Deferred`",
  change_target: "spec/functional/FR-001.md:21",
  next_diagnostic_step: "inspect the authored method and catalog",
};

const vocabularyValueRecord: Required<VocabularyValueRecord> = {
  vocabulary: "verification-methods",
  archetype: "FR",
  field: "verification",
  check: "warning",
  value: "Inspection",
  state: "owned",
  documents: ["spec/functional/FR-001.md"],
};

const obligation: Required<Obligation> = {
  source: "acceptance-criterion",
  id: "FR-001-AC-1",
  document: "spec/functional/FR-001.md",
  statement: "The system shall do it.",
  statement_hash: "a".repeat(64),
  method: "Test",
  parameters: { threshold: "< 8ms" },
  criticality: "P1",
  target_ids: ["TC-001"],
};

const coverageTotals: Required<CoverageTotals> = {
  backed: 1,
  total: 2,
  criteria: 2,
  property_shaped: 1,
  specific_shaped: 1,
};

const unboundSymbol: Required<UnboundSymbol> = {
  path: "tests/parse.rs",
  line: 44,
  symbol: "parse_without_marker",
};

const bindingCensus: Required<BindingCensus> = {
  language: "rust",
  candidates: 2,
  tagged: 1,
  bound: 1,
  self_named: 1,
  self_named_bound: 1,
  forms: ["rust-verifies-line"],
  unbound_example: unboundSymbol,
  unmatched_example: unboundSymbol,
  self_named_unbound_example: unboundSymbol,
};

const engineProvenance: Required<EngineProvenance> = {
  cli: "0.30.2",
  engine: "a14dcb2",
  capabilities: ["binding_census"],
};

const metric: Required<MeasuredMetric> = {
  name: "coverage.backed",
  unit: "matrix row",
  method: "backed matrix rows divided by all matrix rows",
  shape: "ratio",
  state: "measured",
  value: 1,
  population: 2,
  examined: 2,
  matched: 1,
};

const suspicionSurface: Required<Suspicion> = {
  kind: "vacuous-under-guard",
  path: "tests/parse.rs",
  symbol: "all_inputs",
  line: 50,
  message: "every assertion is guarded",
  evidence: "1 of 42 samples entered the assertion",
  subject: "test symbol `all_inputs`",
  change_target: "tests/parse.rs:50",
  remedy: "add an unconditional oracle",
  next_diagnostic_step: "inspect inputs that bypass the narrowing guard",
};

const suspicion: Suspicion = {
  kind: "vacuous-under-guard",
  path: "tests/parse.rs",
  symbol: "all_inputs",
  line: 50,
  message: "every assertion is guarded",
  evidence: "1 of 42 samples entered the assertion",
  subject: "test symbol `all_inputs`",
  change_target: "tests/parse.rs:50",
  next_diagnostic_step: "inspect inputs that bypass the narrowing guard",
};

/** Sample per `$defs` entry, keyed by the schema's own def names. */
const samples: Record<string, unknown> = {
  BindingCensus: bindingCensus,
  CatchAllCriterion: catchAllCriterion,
  UnbackedRow: unbackedRow,
  StatusLie: statusLie,
  NoSymbolRow: noSymbolRow,
  UndeclaredStatus: undeclaredStatus,
  ImplementsRecord: implementsRecord,
  UntrackedSymbol: untrackedSymbol,
  MintedTargetRecord: mintedTargetRecord,
  UnmatchedTag: unmatchedTag,
  SharedTraceId: sharedTraceId,
  SharedTraceSymbol: sharedTraceSymbol,
  GroupCounts: groupCounts,
  CriteriaCounts: criteriaCounts,
  CoverageDiagnostic: coverageDiagnosticSurface,
  VocabularyValueRecord: vocabularyValueRecord,
  Obligation: obligation,
  CoverageTotals: coverageTotals,
  EngineProvenance: engineProvenance,
  GroundingCounts: groundingCounts,
  Metric: metric,
  MetricMethod: metric.method,
  MetricName: metric.name,
  MetricShape: metric.shape,
  MetricUnit: metric.unit,
  Suspicion: suspicionSurface,
  UnboundSymbol: unboundSymbol,
};

const report: Required<CoverageReport> = {
  unbacked_rows: [unbackedRow],
  status_lies: [statusLie],
  no_symbol_rows: [noSymbolRow],
  undeclared_statuses: [undeclaredStatus],
  untracked_symbols: [untrackedSymbol],
  minted_targets: [mintedTargetRecord],
  unmatched_tags: [unmatchedTag],
  shared_trace_ids: [sharedTraceId],
  groups: [groupCounts],
  criteria: [criteriaCounts],
  diagnostics: [coverageDiagnostic],
  diagnostic_reason_registry: ["uncatalogued-verification-method"],
  obligations: [obligation],
  implements: [implementsRecord],
  vocabulary_coverage: [vocabularyValueRecord],
  excluded_source_files: 3,
  binding_census: [bindingCensus],
  metrics: [metric],
  suspicions: [suspicion],
  engine: engineProvenance,
  totals: coverageTotals,
};

const sorted = (keys: Iterable<string>) => [...keys].sort();

describe("TC-272 the interfaces and the vendored coverage schema agree", () => {
  // TC-272
  it("covers every $defs entry with a typed sample", () => {
    expect(sorted(Object.keys(samples))).toEqual(
      sorted(Object.keys(schema.$defs)),
    );
  });

  // TC-272
  it("each sample's keys equal its $defs entry's keys, both directions", () => {
    for (const [name, sample] of Object.entries(samples)) {
      if (typeof sample !== "object" || sample === null) continue;
      const definition = schema.$defs[name];
      const properties =
        definition.properties ??
        definition.oneOf?.find((branch) => {
          const state = branch.properties.state as { const?: unknown };
          return state?.const === (sample as { state?: unknown }).state;
        })?.properties;
      expect(properties, `${name}: no matching object schema`).toBeDefined();
      expect(
        sorted(Object.keys(sample)),
        `${name}: interface and schema declare different keys`,
      ).toEqual(sorted(Object.keys(properties ?? {})));
    }
  });

  // TC-272
  it("the report's top-level keys equal the schema's, both directions", () => {
    expect(sorted(Object.keys(report))).toEqual(
      sorted(Object.keys(schema.properties)),
    );
  });

  // TC-272
  it("the fully-populated typed report validates against the schema", () => {
    const result = validateCoverage(
      report as unknown as Record<string, unknown>,
    );
    expect(result.ok, JSON.stringify(result)).toBe(true);
  });

  // TC-272
  it("this file typechecks, so the Required<Interface> half is enforced", () => {
    // vitest strips types and the build's dts diagnostics are non-fatal, so
    // without this an interface missing a field would fail NOTHING at
    // runtime: the sample would keep the key as a value, the schema would
    // still accept it, and only an editor would ever complain. tsc over this
    // one file (and its transitive imports, src/quire/*) is the seam that
    // turns the compile error into a red test.
    const here = dirname(fileURLToPath(import.meta.url));
    const tsc = join(here, "..", "node_modules", ".bin", "tsc");
    try {
      execFileSync(
        tsc,
        [
          // File-list mode, deliberately: the repo tsconfig excludes tests/,
          // and tsconfig.eslint.json pulls in the whole tree, which carries
          // known diagnostics (quoin#180) this gate must not inherit.
          "--ignoreConfig",
          "--noEmit",
          "--strict",
          "--skipLibCheck",
          "--target",
          "esnext",
          "--module",
          "esnext",
          "--moduleResolution",
          "bundler",
          "--types",
          "node",
          join(here, "quire-types-conformance.test.ts"),
        ],
        {
          encoding: "utf8",
          stdio: ["ignore", "pipe", "pipe"],
          timeout: 30_000,
        },
      );
    } catch (cause) {
      const failure = cause as { stdout?: string; stderr?: string };
      expect.fail(
        `tsc rejected the typed samples:\n${failure.stdout ?? ""}${failure.stderr ?? ""}`,
      );
    }
  }, 35_000);
});
