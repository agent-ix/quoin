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
  CoverageDiagnostic,
  CoverageReport,
  CoverageTotals,
  CriteriaCounts,
  GroupCounts,
  ImplementsRecord,
  NoSymbolRow,
  Obligation,
  StatusLie,
  UnbackedRow,
  UndeclaredStatus,
  UntrackedSymbol,
  VocabularyValueRecord,
} from "../src/quire/types.js";

const schema = readSchema("coverage-v1.schema.json") as {
  properties: Record<string, unknown>;
  $defs: Record<string, { properties: Record<string, unknown> }>;
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
};

const statusLie: Required<StatusLie> = {
  reference: "traces-to",
  document: "spec/tests.md",
  row_id: "TC-002",
  status: "✅",
  target_ids: ["TC-002"],
};

const noSymbolRow: Required<NoSymbolRow> = {
  reference: "traces-to",
  document: "spec/tests.md",
  row_id: "TC-003",
  test_type: "Eval",
  target_ids: ["TC-003"],
};

const undeclaredStatus: Required<UndeclaredStatus> = {
  reference: "traces-to",
  document: "spec/tests.md",
  row_id: "TC-004",
  status: "⚠️ scale evidence deferred",
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
};

const coverageDiagnostic: Required<CoverageDiagnostic> = {
  declaration: "test-case",
  reason: "uncatalogued-verification-method",
  message:
    "'Deferred' is neither a declared verification_catalog method id nor a declared class",
  path: null,
  value: "Deferred",
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
};

/** Sample per `$defs` entry, keyed by the schema's own def names. */
const samples: Record<string, Record<string, unknown>> = {
  UnbackedRow: unbackedRow,
  StatusLie: statusLie,
  NoSymbolRow: noSymbolRow,
  UndeclaredStatus: undeclaredStatus,
  ImplementsRecord: implementsRecord,
  UntrackedSymbol: untrackedSymbol,
  GroupCounts: groupCounts,
  CriteriaCounts: criteriaCounts,
  CoverageDiagnostic: coverageDiagnostic,
  VocabularyValueRecord: vocabularyValueRecord,
  Obligation: obligation,
  CoverageTotals: coverageTotals,
};

const report: Required<CoverageReport> = {
  unbacked_rows: [unbackedRow],
  status_lies: [statusLie],
  no_symbol_rows: [noSymbolRow],
  undeclared_statuses: [undeclaredStatus],
  untracked_symbols: [untrackedSymbol],
  groups: [groupCounts],
  criteria: [criteriaCounts],
  diagnostics: [coverageDiagnostic],
  obligations: [obligation],
  implements: [implementsRecord],
  vocabulary_coverage: [vocabularyValueRecord],
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
      expect(
        sorted(Object.keys(sample)),
        `${name}: interface and schema declare different keys`,
      ).toEqual(sorted(Object.keys(schema.$defs[name].properties)));
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
        { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
      );
    } catch (cause) {
      const failure = cause as { stdout?: string; stderr?: string };
      expect.fail(
        `tsc rejected the typed samples:\n${failure.stdout ?? ""}${failure.stderr ?? ""}`,
      );
    }
  });
});
