/**
 * Capture the golden corpus for the Rust ports of `src/semantic/` and
 * `src/completeness/` (issue #378, EPIC #373 Stage 3).
 *
 * The TypeScript is the oracle **exactly once**: this script runs it, writes
 * the corpus and every expected verdict under the two crates’ `tests/goldens/` directories, and the
 * Rust tests then read those files. Nothing in the Rust test lane shells out to
 * Node.
 *
 * Usage (from the repository root):
 *
 *   pnpm install --frozen-lockfile
 *   node_modules/.bin/tsc -p tsconfig.json --outDir .oracle \
 *     --declaration false --declarationMap false
 *   cp -r src/semantic/schemas .oracle/semantic/schemas
 *   cp src/semantic/sweep-report.schema.json .oracle/semantic/
 *   node scripts/capture-semantic-goldens.mjs
 *
 * `.oracle/` is a throwaway build of `src/` — the compiler is the repository's
 * own `tsc` at the pinned `typescript` version, and the vendored schema tree is
 * copied verbatim because `contract.ts` resolves it relative to its own module.
 *
 * Every emitted file carries a `provenance` block naming the quoin revision the
 * oracle was built from. Re-running this script against a different revision
 * and committing the result is how a golden is legitimately changed; editing a
 * golden by hand is not.
 */

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..");
const oracleRoot = resolve(repoRoot, process.argv[2] ?? ".oracle");

const semanticGoldens = join(repoRoot, "rust/quoin-semantic/tests/goldens");
const completenessGoldens = join(
  repoRoot,
  "rust/quoin-completeness/tests/goldens",
);

const contract = await import(`${oracleRoot}/semantic/contract.js`);
const manifestModule = await import(`${oracleRoot}/semantic/manifest.js`);
const dataSchemaModule = await import(`${oracleRoot}/semantic/data-schema.js`);
const sweepModule = await import(`${oracleRoot}/semantic/sweep.js`);
const packageManifestModule = await import(
  `${oracleRoot}/semantic/package-manifest.js`
);
const assessModule = await import(`${oracleRoot}/completeness/assess.js`);
const bundleModule = await import(`${oracleRoot}/completeness/bundle.js`);
const runModule = await import(`${oracleRoot}/completeness/run.js`);
const declarationsModule = await import(
  `${oracleRoot}/completeness/declarations.js`
);

const provenance = {
  producer: "scripts/capture-semantic-goldens.mjs",
  oracle: "agent-ix/quoin src/semantic + src/completeness (TypeScript)",
  quoinRevision: execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: repoRoot,
    encoding: "utf8",
  }).trim(),
  ajvVersion: JSON.parse(
    readFileSync(join(repoRoot, "node_modules/ajv/package.json"), "utf8"),
  ).version,
  note:
    "Captured once from the TypeScript implementation. The Rust crates read " +
    "these files; they never invoke Node. Regenerate by re-running the " +
    "producer, never by hand-editing.",
};

/** Deterministic key order so a regeneration diffs cleanly. */
function stable(value) {
  if (Array.isArray(value)) return value.map(stable);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, stable(value[key])]),
    );
  }
  return value;
}

function emit(dir, name, body) {
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, name),
    `${JSON.stringify({ provenance, ...body }, null, 2)}\n`,
  );
  return join(dir, name);
}

// ---------------------------------------------------------------------------
// Schema-parity corpora
// ---------------------------------------------------------------------------

/**
 * The normalized diagnostic tuple the acceptance criterion fixes:
 * (instance-location, keyword, schema-location). Error *text* is explicitly
 * not contractual, so it is recorded for the divergence report and never
 * asserted.
 */
function normalize(errors) {
  return (errors ?? []).map((e) => ({
    instancePath: e.instancePath,
    keyword: e.keyword,
    schemaPath: e.schemaPath,
    params: stable(e.params ?? {}),
    message: e.message ?? null,
  }));
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

const moduleManifestSchema = readJson(contract.moduleManifestSchemaPath());
const semanticBlockSchema = moduleManifestSchema.properties.semantic;
const sweepReportSchema = readJson(
  join(oracleRoot, "semantic/sweep-report.schema.json"),
);
const packageManifestSchema = readJson(contract.packageManifestSchemaPath());
const commonSchema = readJson(contract.commonSchemaPath());

/** `semantic/manifest.ts` `semanticBlockValidator()`, byte for byte. */
function semanticBlockValidator() {
  const ajv = new Ajv2020({
    verbose: true,
    allErrors: true,
    strict: true,
    useDefaults: false,
  });
  return ajv.compile(semanticBlockSchema);
}

/** `semantic/manifest.ts` `sweepReportValidator()`. */
function sweepReportValidator() {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  return ajv.compile(sweepReportSchema);
}

/** `semantic/package-manifest.ts` `validatePackageManifest()`. */
function packageManifestValidator() {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  ajv.addSchema(commonSchema);
  return ajv.compile(packageManifestSchema);
}

function captureSchemaCorpus(schemaId, validate, cases) {
  const documents = cases.map(({ id, expect, document }) => {
    const valid = validate(document);
    if (expect !== undefined && expect !== valid) {
      throw new Error(
        `corpus case ${schemaId}/${id} expected valid=${expect}, ajv says ${valid}`,
      );
    }
    return {
      id,
      document,
      valid,
      errors: valid ? [] : normalize(validate.errors),
    };
  });
  const valid = documents.filter((d) => d.valid).length;
  return {
    schema: schemaId,
    counts: {
      total: documents.length,
      valid,
      invalid: documents.length - valid,
    },
    documents,
  };
}

// --- S1: the `semantic` block -------------------------------------------

const BLOCK = {
  contract_version: "1.0.0",
  semantic_core: "0.1.0",
  package: "agent-ix/spec-objects",
};

/** A copy of the minimal valid block with `patch` applied; `undefined` deletes. */
function block(patch = {}) {
  const out = { ...BLOCK };
  for (const [key, value] of Object.entries(patch)) {
    if (value === undefined) delete out[key];
    else out[key] = value;
  }
  return out;
}

const TARGETS = [
  "json-schema",
  "rust",
  "typescript",
  "python-pydantic-v2",
  "python-dataclass",
  "markdown",
  "json",
  "postgresql",
  "protobuf",
  "avro",
  "arrow",
  "parquet",
  "csv",
  "tsv",
];

const semanticBlockCases = [
  { id: "minimal", expect: true, document: block() },
  {
    id: "all-keys",
    expect: true,
    document: block({
      exports: ["entity", "enumeration"],
      imports: { "agent-ix/other": "1.2.3" },
      targets: ["json-schema", "markdown"],
      mappings: ["config-version"],
      compatibility_posture: "strict",
      legacy_forms: "error",
      sweep_report: "semantic/sweep.json",
    }),
  },
  {
    id: "empty-collections",
    expect: true,
    document: block({
      exports: [],
      imports: {},
      targets: [],
      mappings: [],
    }),
  },
  ...["strict", "additive", "declared-lossy"].map((posture) => ({
    id: `posture-${posture}`,
    expect: true,
    document: block({ compatibility_posture: posture }),
  })),
  ...["warning", "error"].map((value) => ({
    id: `legacy-forms-${value}`,
    expect: true,
    document: block({ legacy_forms: value }),
  })),
  ...TARGETS.map((target) => ({
    id: `target-${target}`,
    expect: true,
    document: block({ targets: [target] }),
  })),
  { id: "targets-all", expect: true, document: block({ targets: TARGETS }) },
  {
    id: "prerelease-versions",
    expect: false,
    document: block({
      contract_version: "1.0.0-rc.1",
    }),
  },
  {
    id: "package-dotted",
    expect: true,
    document: block({
      package: "agent-ix/spec.objects-v2",
    }),
  },
  // --- rejections
  ...["contract_version", "semantic_core", "package"].map((key) => ({
    id: `missing-${key}`,
    expect: false,
    document: block({ [key]: undefined }),
  })),
  { id: "missing-all-required", expect: false, document: {} },
  { id: "unknown-key", expect: false, document: block({ extra: 1 }) },
  { id: "two-unknown-keys", expect: false, document: block({ a: 1, b: 2 }) },
  {
    id: "unknown-key-and-missing",
    expect: false,
    document: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      nope: true,
    },
  },
  {
    id: "contract-version-not-semver",
    expect: false,
    document: block({
      contract_version: "1.0",
    }),
  },
  {
    id: "contract-version-not-string",
    expect: false,
    document: block({
      contract_version: 1,
    }),
  },
  {
    id: "semantic-core-not-semver",
    expect: false,
    document: block({
      semantic_core: "v0.1.0",
    }),
  },
  {
    id: "package-uppercase",
    expect: false,
    document: block({
      package: "Agent-IX/spec-objects",
    }),
  },
  {
    id: "package-no-slash",
    expect: false,
    document: block({
      package: "agent-ix",
    }),
  },
  {
    id: "package-url",
    expect: false,
    document: block({
      package: "https://agent-ix.dev/spec-objects",
    }),
  },
  {
    id: "package-leading-dash",
    expect: false,
    document: block({
      package: "-agent-ix/spec-objects",
    }),
  },
  {
    id: "package-three-segments",
    expect: false,
    document: block({
      package: "agent-ix/spec/objects",
    }),
  },
  {
    id: "exports-not-array",
    expect: false,
    document: block({
      exports: "entity",
    }),
  },
  {
    id: "exports-non-string",
    expect: false,
    document: block({
      exports: ["entity", 7],
    }),
  },
  {
    id: "exports-empty-string",
    expect: false,
    document: block({
      exports: [""],
    }),
  },
  {
    id: "exports-duplicate",
    expect: false,
    document: block({
      exports: ["entity", "entity"],
    }),
  },
  {
    id: "exports-two-faults",
    expect: false,
    document: block({
      exports: ["", "", 3],
    }),
  },
  {
    id: "imports-not-object",
    expect: false,
    document: block({
      imports: ["agent-ix/other"],
    }),
  },
  {
    id: "imports-bad-version",
    expect: false,
    document: block({
      imports: { "agent-ix/other": "^1.0.0" },
    }),
  },
  {
    id: "imports-two-bad-versions",
    expect: false,
    document: block({
      imports: { "agent-ix/a": "1", "agent-ix/b": "latest" },
    }),
  },
  {
    id: "imports-value-not-string",
    expect: false,
    document: block({
      imports: { "agent-ix/other": 1 },
    }),
  },
  {
    id: "targets-unknown",
    expect: false,
    document: block({
      targets: ["cobol"],
    }),
  },
  {
    id: "targets-duplicate",
    expect: false,
    document: block({
      targets: ["json-schema", "json-schema"],
    }),
  },
  {
    id: "targets-not-array",
    expect: false,
    document: block({
      targets: "json-schema",
    }),
  },
  {
    id: "targets-two-unknown",
    expect: false,
    document: block({
      targets: ["cobol", "fortran"],
    }),
  },
  {
    id: "mappings-empty-string",
    expect: false,
    document: block({
      mappings: [""],
    }),
  },
  {
    id: "mappings-duplicate",
    expect: false,
    document: block({
      mappings: ["m", "m"],
    }),
  },
  {
    id: "posture-unknown",
    expect: false,
    document: block({
      compatibility_posture: "lossy",
    }),
  },
  {
    id: "posture-null",
    expect: false,
    document: block({
      compatibility_posture: null,
    }),
  },
  {
    id: "legacy-forms-unknown",
    expect: false,
    document: block({
      legacy_forms: "fatal",
    }),
  },
  {
    id: "sweep-report-empty",
    expect: false,
    document: block({
      sweep_report: "",
    }),
  },
  {
    id: "sweep-report-not-string",
    expect: false,
    document: block({
      sweep_report: 42,
    }),
  },
  {
    id: "many-faults",
    expect: false,
    document: {
      contract_version: "1.0",
      semantic_core: "0.1",
      package: "BAD",
      exports: ["", ""],
      targets: ["cobol"],
      compatibility_posture: "lossy",
      legacy_forms: "fatal",
      extra: true,
    },
  },
  { id: "not-an-object-array", expect: false, document: [] },
  { id: "not-an-object-string", expect: false, document: "semantic" },
  { id: "not-an-object-null", expect: false, document: null },
  {
    id: "deeply-nested-unknown",
    expect: false,
    document: block({
      imports: { "agent-ix/other": { version: "1.0.0" } },
    }),
  },
];

// --- S2: the sweep report ------------------------------------------------

const FORMS = {
  "typed-table": 3,
  "free-column-table": 1,
  "bullet-list": 2,
  "sysml-fence": 0,
  none: 4,
};

const REPORT = {
  package: "agent-ix/spec-objects",
  version: "0.1.0",
  generatedAt: "2026-09-12T00:00:00.000Z",
  corpus: [{ repository: "agent-ix/quoin", revision: "4d27dcf" }],
  counts: {
    artifacts: 10,
    forms: { ...FORMS },
    legacy: { "bullet-list": 2, "free-column-table": 1 },
  },
};

function report(patch = {}) {
  const out = structuredClone(REPORT);
  for (const [key, value] of Object.entries(patch)) {
    if (value === undefined) delete out[key];
    else out[key] = value;
  }
  return out;
}

const sweepReportCases = [
  { id: "minimal", expect: true, document: report() },
  { id: "no-findings-key", expect: true, document: report() },
  {
    id: "with-findings",
    expect: true,
    document: report({
      findings: [
        { path: "a.md", form: "typed-table" },
        {
          path: "b.md",
          form: "bullet-list",
          line: 12,
          diagnostic: {
            code: "semantic.legacy-properties-form",
            severity: "warning",
            form: "bullet-list",
            line: 12,
            migration: "typed-table",
          },
        },
      ],
    }),
  },
  { id: "empty-findings", expect: true, document: report({ findings: [] }) },
  { id: "empty-corpus", expect: true, document: report({ corpus: [] }) },
  {
    id: "two-corpus-roots",
    expect: true,
    document: report({
      corpus: [
        { repository: "a/b", revision: "1" },
        { repository: "c/d", revision: "2" },
      ],
    }),
  },
  {
    id: "generated-at-no-fraction",
    expect: true,
    document: report({
      generatedAt: "2026-09-12T00:00:00Z",
    }),
  },
  {
    id: "zero-counts",
    expect: true,
    document: report({
      counts: {
        artifacts: 0,
        forms: {
          "typed-table": 0,
          "free-column-table": 0,
          "bullet-list": 0,
          "sysml-fence": 0,
          none: 0,
        },
        legacy: { "bullet-list": 0, "free-column-table": 0 },
      },
    }),
  },
  ...[
    "typed-table",
    "free-column-table",
    "bullet-list",
    "sysml-fence",
    "none",
  ].map((form) => ({
    id: `finding-form-${form}`,
    expect: true,
    document: report({ findings: [{ path: "x.md", form }] }),
  })),
  // --- rejections
  ...["package", "version", "generatedAt", "corpus", "counts"].map((key) => ({
    id: `missing-${key}`,
    expect: false,
    document: report({ [key]: undefined }),
  })),
  { id: "unknown-top-key", expect: false, document: report({ extra: 1 }) },
  {
    id: "package-bad-pattern",
    expect: false,
    document: report({
      package: "AgentIX",
    }),
  },
  {
    id: "version-bad-pattern",
    expect: false,
    document: report({
      version: "0.1",
    }),
  },
  {
    id: "generated-at-not-utc",
    expect: false,
    document: report({
      generatedAt: "2026-09-12T00:00:00+02:00",
    }),
  },
  {
    id: "generated-at-date-only",
    expect: false,
    document: report({
      generatedAt: "2026-09-12",
    }),
  },
  {
    id: "generated-at-not-string",
    expect: false,
    document: report({
      generatedAt: 1757635200,
    }),
  },
  { id: "corpus-not-array", expect: false, document: report({ corpus: {} }) },
  {
    id: "corpus-entry-missing-revision",
    expect: false,
    document: report({
      corpus: [{ repository: "a/b" }],
    }),
  },
  {
    id: "corpus-entry-unknown-key",
    expect: false,
    document: report({
      corpus: [{ repository: "a/b", revision: "1", branch: "main" }],
    }),
  },
  {
    id: "corpus-entry-empty-repository",
    expect: false,
    document: report({
      corpus: [{ repository: "", revision: "1" }],
    }),
  },
  {
    id: "counts-missing-forms",
    expect: false,
    document: report({
      counts: {
        artifacts: 1,
        legacy: { "bullet-list": 0, "free-column-table": 0 },
      },
    }),
  },
  {
    id: "counts-artifacts-negative",
    expect: false,
    document: report({
      counts: { ...REPORT.counts, artifacts: -1 },
    }),
  },
  {
    id: "counts-artifacts-float",
    expect: false,
    document: report({
      counts: { ...REPORT.counts, artifacts: 1.5 },
    }),
  },
  {
    id: "counts-artifacts-integral-float",
    expect: true,
    document: report({
      counts: { ...REPORT.counts, artifacts: 10.0 },
    }),
  },
  {
    id: "counts-forms-missing-one",
    expect: false,
    document: report({
      counts: {
        ...REPORT.counts,
        forms: {
          "typed-table": 1,
          "free-column-table": 0,
          "bullet-list": 0,
          "sysml-fence": 0,
        },
      },
    }),
  },
  {
    id: "counts-forms-unknown-key",
    expect: false,
    document: report({
      counts: { ...REPORT.counts, forms: { ...FORMS, prose: 1 } },
    }),
  },
  {
    id: "counts-legacy-unknown-key",
    expect: false,
    document: report({
      counts: {
        ...REPORT.counts,
        legacy: { "bullet-list": 0, "free-column-table": 0, "sysml-fence": 0 },
      },
    }),
  },
  {
    id: "counts-unknown-key",
    expect: false,
    document: report({
      counts: { ...REPORT.counts, total: 10 },
    }),
  },
  {
    id: "finding-missing-form",
    expect: false,
    document: report({
      findings: [{ path: "a.md" }],
    }),
  },
  {
    id: "finding-unknown-form",
    expect: false,
    document: report({
      findings: [{ path: "a.md", form: "prose" }],
    }),
  },
  {
    id: "finding-empty-path",
    expect: false,
    document: report({
      findings: [{ path: "", form: "none" }],
    }),
  },
  {
    id: "finding-line-zero",
    expect: false,
    document: report({
      findings: [{ path: "a.md", form: "none", line: 0 }],
    }),
  },
  {
    id: "finding-extra-key-allowed",
    expect: true,
    document: report({
      findings: [{ path: "a.md", form: "none", note: "kept" }],
    }),
  },
  {
    id: "finding-diagnostic-not-object",
    expect: false,
    document: report({
      findings: [{ path: "a.md", form: "none", diagnostic: "warning" }],
    }),
  },
  {
    id: "findings-not-array",
    expect: false,
    document: report({
      findings: {},
    }),
  },
  {
    id: "two-findings-both-bad",
    expect: false,
    document: report({
      findings: [
        { path: "", form: "prose" },
        { path: "b.md", form: "x" },
      ],
    }),
  },
  { id: "not-an-object", expect: false, document: [] },
  { id: "empty-object", expect: false, document: {} },
  {
    id: "many-faults",
    expect: false,
    document: {
      package: "X",
      version: "1",
      generatedAt: "now",
      corpus: [{}],
      counts: {},
      extra: 1,
    },
  },
];

// --- S3: the derived package manifest -----------------------------------

const MANIFEST = {
  contractVersion: "1.0.0",
  package: { identity: "agent-ix/spec-objects", version: "0.1.0" },
  schemaDialect: "https://json-schema.org/draft/2020-12/schema",
  sourceRoots: ["schemas/"],
  imports: [
    {
      packageIdentity: "agent-ix/semantic-core",
      versionConstraint: "=0.1.0",
      exports: [],
      capabilities: [],
    },
  ],
  exports: [
    {
      name: "entity",
      typeIdentity: "ix://agent-ix/spec-objects/type/entity",
      visibility: "public",
    },
  ],
  profiles: [
    {
      name: "default",
      version: "0.1.0",
      exports: ["entity"],
      targets: ["json-schema"],
      mappings: [],
      options: {},
      compatibilityPosture: "additive",
    },
  ],
  targets: ["json-schema"],
  mappings: [],
  extensions: [],
};

function manifest(patch = {}) {
  const out = structuredClone(MANIFEST);
  for (const [key, value] of Object.entries(patch)) {
    if (value === undefined) delete out[key];
    else out[key] = value;
  }
  return out;
}

const packageManifestCases = [
  { id: "minimal", expect: true, document: manifest() },
  {
    id: "no-exports",
    expect: true,
    document: manifest({
      exports: [],
      profiles: [{ ...MANIFEST.profiles[0], exports: [] }],
    }),
  },
  {
    id: "representation-format-target",
    expect: true,
    document: manifest({
      targets: ["markdown", "parquet"],
      profiles: [{ ...MANIFEST.profiles[0], targets: ["markdown", "parquet"] }],
    }),
  },
  {
    id: "mapping-identity",
    expect: true,
    document: manifest({
      mappings: ["ix://agent-ix/spec-objects/mapping/config-version"],
      profiles: [
        {
          ...MANIFEST.profiles[0],
          mappings: ["ix://agent-ix/spec-objects/mapping/config-version"],
        },
      ],
    }),
  },
  {
    id: "private-export",
    expect: true,
    document: manifest({
      exports: [
        {
          name: "internal",
          typeIdentity: "ix://agent-ix/spec-objects/type/internal",
          visibility: "private",
        },
      ],
      profiles: [{ ...MANIFEST.profiles[0], exports: ["internal"] }],
    }),
  },
  {
    id: "prerelease-package-version",
    expect: true,
    document: manifest({
      package: { identity: "agent-ix/spec-objects", version: "0.1.0-rc.1" },
    }),
  },
  {
    id: "build-metadata-version",
    expect: true,
    document: manifest({
      package: { identity: "agent-ix/spec-objects", version: "0.1.0+build.7" },
    }),
  },
  ...["strict", "additive", "declared-lossy"].map((posture) => ({
    id: `posture-${posture}`,
    expect: true,
    document: manifest({
      profiles: [{ ...MANIFEST.profiles[0], compatibilityPosture: posture }],
    }),
  })),
  {
    id: "extension-source-origin",
    expect: true,
    document: manifest({
      extensions: [
        {
          identity: "ix://agent-ix/quoin/ext/a",
          version: "1.0.0",
          required: false,
          payload: { any: "thing" },
        },
      ],
    }),
  },
  {
    id: "extension-with-capability",
    expect: true,
    document: manifest({
      extensions: [
        {
          identity: "ix://agent-ix/quoin/ext/a",
          version: "1.0.0",
          required: true,
          capability: "render",
          payload: null,
        },
      ],
    }),
  },
  {
    id: "import-with-exports",
    expect: true,
    document: manifest({
      imports: [
        {
          packageIdentity: "agent-ix/other",
          versionConstraint: "=1.0.0",
          exports: ["thing"],
          capabilities: ["cap"],
        },
      ],
    }),
  },
  {
    id: "two-profiles",
    expect: true,
    document: manifest({
      profiles: [
        MANIFEST.profiles[0],
        {
          ...MANIFEST.profiles[0],
          name: "wide",
          targets: ["json-schema", "rust"],
        },
      ],
    }),
  },
  // --- rejections
  ...[
    "contractVersion",
    "package",
    "schemaDialect",
    "sourceRoots",
    "exports",
    "imports",
    "profiles",
    "targets",
    "mappings",
    "extensions",
  ].map((key) => ({
    id: `missing-${key}`,
    expect: false,
    document: manifest({ [key]: undefined }),
  })),
  {
    id: "contract-version-wrong-const",
    expect: false,
    document: manifest({
      contractVersion: "1.1.0",
    }),
  },
  {
    id: "schema-dialect-wrong-const",
    expect: false,
    document: manifest({
      schemaDialect: "https://json-schema.org/draft-07/schema",
    }),
  },
  { id: "unknown-top-key", expect: false, document: manifest({ extra: 1 }) },
  {
    id: "package-identity-bad",
    expect: false,
    document: manifest({
      package: { identity: "AgentIX", version: "0.1.0" },
    }),
  },
  {
    id: "package-version-bad-semver",
    expect: false,
    document: manifest({
      package: { identity: "agent-ix/spec-objects", version: "0.1" },
    }),
  },
  {
    id: "package-leading-zero-semver",
    expect: false,
    document: manifest({
      package: { identity: "agent-ix/spec-objects", version: "01.0.0" },
    }),
  },
  {
    id: "package-unknown-key",
    expect: false,
    document: manifest({
      package: {
        identity: "agent-ix/spec-objects",
        version: "0.1.0",
        name: "x",
      },
    }),
  },
  {
    id: "source-roots-empty",
    expect: false,
    document: manifest({
      sourceRoots: [],
    }),
  },
  {
    id: "source-roots-duplicate",
    expect: false,
    document: manifest({
      sourceRoots: ["schemas/", "schemas/"],
    }),
  },
  {
    id: "source-roots-empty-string",
    expect: false,
    document: manifest({
      sourceRoots: [""],
    }),
  },
  {
    id: "export-identity-not-ix",
    expect: false,
    document: manifest({
      exports: [
        {
          name: "entity",
          typeIdentity: "agent-ix/spec-objects/type/entity",
          visibility: "public",
        },
      ],
    }),
  },
  {
    id: "export-identity-uppercase-org",
    expect: false,
    document: manifest({
      exports: [
        {
          name: "entity",
          typeIdentity: "ix://Agent-IX/spec-objects/type/entity",
          visibility: "public",
        },
      ],
    }),
  },
  {
    id: "export-visibility-unknown",
    expect: false,
    document: manifest({
      exports: [
        {
          name: "entity",
          typeIdentity: "ix://agent-ix/spec-objects/type/entity",
          visibility: "internal",
        },
      ],
    }),
  },
  {
    id: "export-unknown-key",
    expect: false,
    document: manifest({
      exports: [
        {
          name: "entity",
          typeIdentity: "ix://agent-ix/spec-objects/type/entity",
          visibility: "public",
          doc: "x",
        },
      ],
    }),
  },
  {
    id: "export-missing-name",
    expect: false,
    document: manifest({
      exports: [
        {
          typeIdentity: "ix://agent-ix/spec-objects/type/entity",
          visibility: "public",
        },
      ],
    }),
  },
  {
    id: "import-missing-capabilities",
    expect: false,
    document: manifest({
      imports: [
        {
          packageIdentity: "agent-ix/other",
          versionConstraint: "=1.0.0",
          exports: [],
        },
      ],
    }),
  },
  {
    id: "import-identity-bad",
    expect: false,
    document: manifest({
      imports: [
        {
          packageIdentity: "ix://agent-ix/other",
          versionConstraint: "=1.0.0",
          exports: [],
          capabilities: [],
        },
      ],
    }),
  },
  {
    id: "import-constraint-empty",
    expect: false,
    document: manifest({
      imports: [
        {
          packageIdentity: "agent-ix/other",
          versionConstraint: "",
          exports: [],
          capabilities: [],
        },
      ],
    }),
  },
  {
    id: "profile-unknown-target",
    expect: false,
    document: manifest({
      profiles: [{ ...MANIFEST.profiles[0], targets: ["cobol"] }],
    }),
  },
  {
    id: "profile-duplicate-target",
    expect: false,
    document: manifest({
      profiles: [
        {
          ...MANIFEST.profiles[0],
          targets: ["json-schema", "json-schema"],
        },
      ],
    }),
  },
  {
    id: "profile-bad-version",
    expect: false,
    document: manifest({
      profiles: [{ ...MANIFEST.profiles[0], version: "one" }],
    }),
  },
  {
    id: "profile-posture-unknown",
    expect: false,
    document: manifest({
      profiles: [{ ...MANIFEST.profiles[0], compatibilityPosture: "lossy" }],
    }),
  },
  {
    id: "profile-options-not-object",
    expect: false,
    document: manifest({
      profiles: [{ ...MANIFEST.profiles[0], options: [] }],
    }),
  },
  {
    id: "profile-mapping-not-identity",
    expect: false,
    document: manifest({
      profiles: [{ ...MANIFEST.profiles[0], mappings: ["config-version"] }],
    }),
  },
  {
    id: "profile-unknown-key",
    expect: false,
    document: manifest({
      profiles: [{ ...MANIFEST.profiles[0], description: "x" }],
    }),
  },
  {
    id: "targets-unknown",
    expect: false,
    document: manifest({
      targets: ["cobol"],
    }),
  },
  {
    id: "targets-duplicate",
    expect: false,
    document: manifest({
      targets: ["rust", "rust"],
    }),
  },
  {
    id: "mappings-not-identity",
    expect: false,
    document: manifest({
      mappings: ["config-version"],
    }),
  },
  {
    id: "extension-missing-payload",
    expect: false,
    document: manifest({
      extensions: [
        {
          identity: "ix://agent-ix/quoin/ext/a",
          version: "1.0.0",
          required: true,
        },
      ],
    }),
  },
  {
    id: "extension-bad-version",
    expect: false,
    document: manifest({
      extensions: [
        {
          identity: "ix://agent-ix/quoin/ext/a",
          version: "1",
          required: true,
          payload: {},
        },
      ],
    }),
  },
  {
    id: "extension-required-not-boolean",
    expect: false,
    document: manifest({
      extensions: [
        {
          identity: "ix://agent-ix/quoin/ext/a",
          version: "1.0.0",
          required: "yes",
          payload: {},
        },
      ],
    }),
  },
  {
    id: "extension-unknown-key",
    expect: false,
    document: manifest({
      extensions: [
        {
          identity: "ix://agent-ix/quoin/ext/a",
          version: "1.0.0",
          required: true,
          payload: {},
          owner: "x",
        },
      ],
    }),
  },
  {
    id: "many-faults",
    expect: false,
    document: {
      contractVersion: "2.0.0",
      package: { identity: "X", version: "1" },
      schemaDialect: "draft-07",
      sourceRoots: [],
      imports: [{}],
      exports: [{}],
      profiles: [{}],
      targets: ["cobol"],
      mappings: ["x"],
      extensions: [{}],
      extra: true,
    },
  },
  { id: "not-an-object", expect: false, document: "manifest" },
  { id: "empty-object", expect: false, document: {} },
];

const schemaCorpora = [
  {
    file: "schema-semantic-block.json",
    capture: captureSchemaCorpus(
      "semantic-block",
      semanticBlockValidator(),
      semanticBlockCases,
    ),
    ajv: {
      options: {
        verbose: true,
        allErrors: true,
        strict: true,
        useDefaults: false,
      },
      compiledFrom:
        "src/semantic/schemas/module-manifest.schema.json #/properties/semantic",
      callSite: "src/semantic/manifest.ts semanticBlockValidator()",
    },
  },
  {
    file: "schema-sweep-report.json",
    capture: captureSchemaCorpus(
      "sweep-report",
      sweepReportValidator(),
      sweepReportCases,
    ),
    ajv: {
      options: { allErrors: true, strict: true },
      compiledFrom: "src/semantic/sweep-report.schema.json",
      callSite: "src/semantic/manifest.ts sweepReportValidator()",
    },
  },
  {
    file: "schema-package-manifest.json",
    capture: captureSchemaCorpus(
      "package-manifest",
      packageManifestValidator(),
      packageManifestCases,
    ),
    ajv: {
      options: { allErrors: true, strict: true },
      compiledFrom:
        "src/semantic/schemas/filament-core-data/package-manifest.schema.json " +
        "(+ common.schema.json via addSchema)",
      callSite: "src/semantic/package-manifest.ts validatePackageManifest()",
    },
  },
];

for (const corpus of schemaCorpora) {
  emit(semanticGoldens, corpus.file, { ajv: corpus.ajv, ...corpus.capture });
}

// ---------------------------------------------------------------------------
// Behavioural goldens — readSemanticBlock over real module trees
// ---------------------------------------------------------------------------

const scratch = join(repoRoot, ".oracle-fixtures");
rmSync(scratch, { recursive: true, force: true });

function writeModule(name, files) {
  const root = join(scratch, name);
  for (const [rel, body] of Object.entries(files)) {
    const path = join(root, rel);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(
      path,
      typeof body === "string" ? body : JSON.stringify(body, null, 2),
    );
  }
  return root;
}

function entitySchema(pkg, version, name = "Entity.json", extra = {}) {
  const [org, repo] = pkg.split("/");
  return {
    $schema: "https://json-schema.org/draft/2020-12/schema",
    $id: `https://schemas.agent-ix.org/${org}/${repo}/${version}/${name}`,
    type: "object",
    properties: {
      fields: {
        type: "array",
        items: {
          $ref: "https://schemas.agent-ix.org/semantic-core/0.1.0/FieldDecl.json",
        },
      },
      ...extra,
    },
    required: ["fields"],
  };
}

function sha256Of(text) {
  return `sha256:${execFileSync("sha256sum", [], { input: text, encoding: "utf8" }).slice(0, 64)}`;
}

/** Build the `data_schema` reference for a schema written verbatim. */
function schemaFile(body) {
  const text = JSON.stringify(body, null, 2);
  return { text, digest: sha256Of(text) };
}

const manifestCases = [];

function manifestCase(id, { files, manifest: manifestBody, moduleRoot }) {
  const root = moduleRoot ?? writeModule(id, files ?? {});
  const result = manifestModule.readSemanticBlock(manifestBody, root);
  manifestCases.push({
    id,
    manifest: manifestBody,
    files: Object.fromEntries(
      Object.entries(files ?? {}).map(([rel, body]) => [
        rel,
        typeof body === "string" ? body : JSON.stringify(body, null, 2),
      ]),
    ),
    diagnostics: result.diagnostics,
    module: result.module
      ? {
          name: result.module.name,
          version: result.module.version,
          block: result.module.block,
          dataSchemas: Object.fromEntries(
            Object.entries(result.module.dataSchemas).map(([k, v]) => [
              k,
              {
                kind: v.kind,
                hasFile: v.file !== undefined,
                diagnostics: v.diagnostics,
              },
            ]),
          ),
        }
      : null,
  });
}

{
  const ok = schemaFile(entitySchema("agent-ix/fixture", "0.1.0"));
  manifestCase("ok-reference-form", {
    files: { "schemas/Entity.json": ok.text },
    manifest: {
      manifest_version: "1.0.0",
      name: "fixture",
      version: "0.1.0",
      object_types: [
        {
          name: "entity",
          data_schema: { schema: "schemas/Entity.json", digest: ok.digest },
        },
      ],
      semantic: {
        contract_version: "1.0.0",
        semantic_core: "0.1.0",
        package: "agent-ix/fixture",
        exports: ["entity"],
        targets: ["json-schema"],
      },
    },
  });

  manifestCase("digest-mismatch", {
    files: { "schemas/Entity.json": ok.text },
    manifest: {
      manifest_version: "1.0.0",
      name: "fixture",
      version: "0.1.0",
      object_types: [
        {
          name: "entity",
          data_schema: {
            schema: "schemas/Entity.json",
            digest: `sha256:${"0".repeat(64)}`,
          },
        },
      ],
      semantic: {
        contract_version: "1.0.0",
        semantic_core: "0.1.0",
        package: "agent-ix/fixture",
        exports: ["entity"],
      },
    },
  });

  manifestCase("missing-schema-file", {
    files: {},
    manifest: {
      manifest_version: "1.0.0",
      name: "fixture",
      version: "0.1.0",
      object_types: [
        {
          name: "entity",
          data_schema: { schema: "schemas/Nope.json", digest: ok.digest },
        },
      ],
      semantic: {
        contract_version: "1.0.0",
        semantic_core: "0.1.0",
        package: "agent-ix/fixture",
        exports: ["entity"],
      },
    },
  });

  const wrongId = schemaFile(entitySchema("agent-ix/other", "0.1.0"));
  manifestCase("wrong-schema-id", {
    files: { "schemas/Entity.json": wrongId.text },
    manifest: {
      manifest_version: "1.0.0",
      name: "fixture",
      version: "0.1.0",
      object_types: [
        {
          name: "entity",
          data_schema: {
            schema: "schemas/Entity.json",
            digest: wrongId.digest,
          },
        },
      ],
      semantic: {
        contract_version: "1.0.0",
        semantic_core: "0.1.0",
        package: "agent-ix/fixture",
        exports: ["entity"],
      },
    },
  });

  const unshipped = schemaFile({
    ...entitySchema("agent-ix/fixture", "0.1.0"),
    properties: {
      fields: {
        $ref: "https://schemas.agent-ix.org/agent-ix/fixture/0.1.0/Missing.json",
      },
    },
  });
  manifestCase("ref-unshipped", {
    files: { "schemas/Entity.json": unshipped.text },
    manifest: {
      manifest_version: "1.0.0",
      name: "fixture",
      version: "0.1.0",
      object_types: [
        {
          name: "entity",
          data_schema: {
            schema: "schemas/Entity.json",
            digest: unshipped.digest,
          },
        },
      ],
      semantic: {
        contract_version: "1.0.0",
        semantic_core: "0.1.0",
        package: "agent-ix/fixture",
        exports: ["entity"],
      },
    },
  });

  const wrongCore = schemaFile({
    ...entitySchema("agent-ix/fixture", "0.1.0"),
    properties: {
      fields: {
        $ref: "https://schemas.agent-ix.org/semantic-core/0.2.0/FieldDecl.json",
      },
    },
  });
  manifestCase("ref-wrong-semantic-core", {
    files: { "schemas/Entity.json": wrongCore.text },
    manifest: {
      manifest_version: "1.0.0",
      name: "fixture",
      version: "0.1.0",
      object_types: [
        {
          name: "entity",
          data_schema: {
            schema: "schemas/Entity.json",
            digest: wrongCore.digest,
          },
        },
      ],
      semantic: {
        contract_version: "1.0.0",
        semantic_core: "0.1.0",
        package: "agent-ix/fixture",
        exports: ["entity"],
      },
    },
  });

  const foreign = schemaFile({
    ...entitySchema("agent-ix/fixture", "0.1.0"),
    properties: { fields: { $ref: "https://example.invalid/Other.json" } },
  });
  manifestCase("ref-foreign-host", {
    files: { "schemas/Entity.json": foreign.text },
    manifest: {
      manifest_version: "1.0.0",
      name: "fixture",
      version: "0.1.0",
      object_types: [
        {
          name: "entity",
          data_schema: {
            schema: "schemas/Entity.json",
            digest: foreign.digest,
          },
        },
      ],
      semantic: {
        contract_version: "1.0.0",
        semantic_core: "0.1.0",
        package: "agent-ix/fixture",
        exports: ["entity"],
      },
    },
  });
}

manifestCase("no-semantic-block", {
  files: {},
  manifest: { manifest_version: "1.0.0", name: "plain", version: "0.1.0" },
});
manifestCase("semantic-not-an-object", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: [],
  },
});
manifestCase("unsupported-contract-version", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "2.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
    },
  },
});
manifestCase("unknown-semantic-core", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "9.9.9",
      package: "a/b",
    },
  },
});
manifestCase("export-not-in-object-types", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    object_types: [{ name: "entity" }],
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
      exports: ["ghost"],
    },
  },
});
manifestCase("export-without-reference-schema", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    object_types: [{ name: "entity", data_schema: { type: "object" } }],
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
      exports: ["entity"],
    },
  },
});
manifestCase("inline-data-schema-warning", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    object_types: [{ name: "enumeration", data_schema: { type: "object" } }],
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
    },
  },
});
manifestCase("ambiguous-data-schema", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    object_types: [
      {
        name: "entity",
        data_schema: { schema: "a.json", digest: "sha256:x", type: "object" },
      },
    ],
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
    },
  },
});
manifestCase("data-schema-escape", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    object_types: [
      {
        name: "entity",
        data_schema: {
          schema: "../outside/Entity.json",
          digest: `sha256:${"a".repeat(64)}`,
        },
      },
    ],
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
    },
  },
});
manifestCase("data-schema-bad-digest-form", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    object_types: [
      {
        name: "entity",
        data_schema: { schema: "a.json", digest: "sha1:deadbeef" },
      },
    ],
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
    },
  },
});
manifestCase("schema-block-rejected-unknown-key", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
      nope: 1,
    },
  },
});
manifestCase("legacy-forms-error-without-sweep-report", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
      legacy_forms: "error",
    },
  },
});
manifestCase("legacy-forms-error-sweep-report-missing-file", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
      legacy_forms: "error",
      sweep_report: "semantic/sweep.json",
    },
  },
});
manifestCase("legacy-forms-error-sweep-report-escapes", {
  files: {},
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "a/b",
      legacy_forms: "error",
      sweep_report: "../sweep.json",
    },
  },
});
manifestCase("legacy-forms-error-sweep-report-wrong-package", {
  files: {
    "semantic/sweep.json": { ...REPORT, package: "other/pkg" },
  },
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "agent-ix/spec-objects",
      legacy_forms: "error",
      sweep_report: "semantic/sweep.json",
    },
  },
});
manifestCase("legacy-forms-error-sweep-report-wrong-version", {
  files: { "semantic/sweep.json": { ...REPORT, version: "9.9.9" } },
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "agent-ix/spec-objects",
      legacy_forms: "error",
      sweep_report: "semantic/sweep.json",
    },
  },
});
manifestCase("legacy-forms-error-sweep-report-not-json", {
  files: { "semantic/sweep.json": "{ not json" },
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "agent-ix/spec-objects",
      legacy_forms: "error",
      sweep_report: "semantic/sweep.json",
    },
  },
});
manifestCase("legacy-forms-error-sweep-report-schema-invalid", {
  files: {
    "semantic/sweep.json": {
      ...REPORT,
      version: "0.1.0",
      package: "agent-ix/spec-objects",
      counts: {
        artifacts: -1,
        forms: FORMS,
        legacy: { "bullet-list": 0, "free-column-table": 0 },
      },
    },
  },
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "agent-ix/spec-objects",
      legacy_forms: "error",
      sweep_report: "semantic/sweep.json",
    },
  },
});
manifestCase("legacy-forms-error-sweep-report-ok", {
  files: {
    "semantic/sweep.json": {
      ...REPORT,
      version: "0.1.0",
      package: "agent-ix/spec-objects",
    },
  },
  manifest: {
    manifest_version: "1.0.0",
    name: "x",
    version: "0.1.0",
    semantic: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: "agent-ix/spec-objects",
      legacy_forms: "error",
      sweep_report: "semantic/sweep.json",
    },
  },
});

emit(semanticGoldens, "read-semantic-block.json", {
  source: "src/semantic/manifest.ts readSemanticBlock()",
  counts: { total: manifestCases.length },
  cases: manifestCases,
});

// ---------------------------------------------------------------------------
// Behavioural goldens — classifyProperties / classifyArtifact
// ---------------------------------------------------------------------------

const sweepDocuments = [
  ["no-properties-section", "# Doc\n\nSome prose.\n"],
  [
    "typed-table",
    "## Properties\n\n| Field | Type | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n| id | UUID | 1 | identity |\n",
  ],
  [
    "typed-table-no-outer-pipes",
    "## Properties\n\nField | Type | Multiplicity | Constraints\n--- | --- | --- | ---\nid | UUID | 1 | identity\n",
  ],
  [
    "free-column-table",
    "## Properties\n\n| Column | Type | Constraints |\n| --- | --- | --- |\n| id | UUID | PK |\n",
  ],
  [
    "typed-header-wrong-order",
    "## Properties\n\n| Type | Field | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n| UUID | id | 1 | x |\n",
  ],
  [
    "typed-header-extra-column",
    "## Properties\n\n| Field | Type | Multiplicity | Constraints | Notes |\n| --- | --- | --- | --- | --- |\n| id | UUID | 1 | x | y |\n",
  ],
  [
    "typed-header-case-differs",
    "## Properties\n\n| field | Type | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n| id | UUID | 1 | x |\n",
  ],
  ["bullet-list-dash", "## Properties\n\n- id: UUID\n- name: string\n"],
  ["bullet-list-star", "## Properties\n\n* id: UUID\n"],
  ["bullet-list-indented", "## Properties\n\n  - id: UUID\n"],
  ["sysml-fence", "## Properties\n\n```sysml\npart def X;\n```\n"],
  ["other-fence-first", "## Properties\n\n```json\n{}\n```\n\n- id: UUID\n"],
  [
    "fence-hides-table",
    "## Properties\n\n```\n| Field | Type |\n| --- | --- |\n```\n\n- id: UUID\n",
  ],
  ["empty-properties-section", "## Properties\n\n## Next\n\n- x\n"],
  [
    "properties-then-next-heading",
    "## Properties\n\n- id: UUID\n\n## Other\n\n| Field | Type | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n",
  ],
  ["properties-heading-with-trailing-space", "## Properties \n\n- id: UUID\n"],
  ["properties-heading-level-3", "### Properties\n\n- id: UUID\n"],
  ["properties-heading-suffixed", "## Properties (draft)\n\n- id: UUID\n"],
  [
    "table-without-separator",
    "## Properties\n\n| Field | Type |\n| id | UUID |\n",
  ],
  [
    "table-separator-with-colons",
    "## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|:---|:---:|---:|---|\n| id | UUID | 1 | x |\n",
  ],
  [
    "two-tables-first-wins",
    "## Properties\n\n| Column | Type |\n| --- | --- |\n| id | UUID |\n\n| Field | Type | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n| id | UUID | 1 | x |\n",
  ],
  [
    "list-then-table",
    "## Properties\n\n- id: UUID\n\n| Field | Type | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n| id | UUID | 1 | x |\n",
  ],
  ["unterminated-fence", "## Properties\n\n```sysml\npart def X;\n"],
  [
    "crlf-typed-table",
    "## Properties\r\n\r\n| Field | Type | Multiplicity | Constraints |\r\n| --- | --- | --- | --- |\r\n",
  ],
  ["bullet-with-no-text", "## Properties\n\n-\n- id: UUID\n"],
  ["empty-document", ""],
  ["properties-at-end", "# Doc\n\n## Properties\n"],
  ["two-lists-one-block", "## Properties\n\n- a\n- b\n\n- c\n"],
];

emit(semanticGoldens, "classify-properties.json", {
  source: "src/semantic/sweep.ts classifyProperties() / classifyArtifact()",
  counts: { total: sweepDocuments.length },
  cases: sweepDocuments.map(([id, markdown]) => {
    const classified = sweepModule.classifyProperties(markdown);
    return {
      id,
      markdown,
      form: classified.form,
      line: classified.line ?? null,
      finding: sweepModule.classifyArtifact(`fixture:${id}.md`, markdown),
    };
  }),
});

// ---------------------------------------------------------------------------
// Behavioural goldens — derivePackageManifest / identities / resolveImports
// ---------------------------------------------------------------------------

function semanticModule(name, version, blockPatch, dataSchemas = {}) {
  return {
    name,
    version,
    root: `/modules/${name}`,
    block: {
      contract_version: "1.0.0",
      semantic_core: "0.1.0",
      package: `agent-ix/${name}`,
      exports: [],
      imports: {},
      targets: [],
      mappings: [],
      compatibility_posture: "additive",
      legacy_forms: "warning",
      ...blockPatch,
    },
    dataSchemas,
  };
}

const deriveCases = [
  ["empty", semanticModule("a", "0.1.0", {})],
  [
    "exports-sorted",
    semanticModule("a", "0.1.0", { exports: ["zeta", "alpha", "mid"] }),
  ],
  ["mappings-sorted", semanticModule("a", "0.1.0", { mappings: ["z", "a"] })],
  [
    "imports-sorted",
    semanticModule("a", "0.2.0", {
      imports: { "agent-ix/z": "1.0.0", "agent-ix/a": "2.0.0" },
    }),
  ],
  [
    "targets-preserved-order",
    semanticModule("a", "0.1.0", {
      targets: ["markdown", "json-schema"],
    }),
  ],
  [
    "posture-strict",
    semanticModule("a", "0.1.0", { compatibility_posture: "strict" }),
  ],
  [
    "full",
    semanticModule("a", "1.2.3", {
      exports: ["b", "a"],
      imports: { "agent-ix/other": "0.9.0" },
      targets: ["rust", "typescript"],
      mappings: ["m2", "m1"],
      compatibility_posture: "declared-lossy",
    }),
  ],
];

const pmValidator = packageManifestValidator();

emit(semanticGoldens, "derive-package-manifest.json", {
  source: "src/semantic/package-manifest.ts derivePackageManifest()",
  counts: { total: deriveCases.length },
  cases: deriveCases.map(([id, module]) => {
    const derived = packageManifestModule.derivePackageManifest(module);
    return {
      id,
      module: {
        name: module.name,
        version: module.version,
        block: module.block,
      },
      derived,
      validAgainstSchema: pmValidator(derived),
      registryPin: packageManifestModule.registryPin(module),
    };
  }),
});

const importCases = [
  [
    "resolved",
    semanticModule("a", "0.1.0", { imports: { "agent-ix/b": "0.2.0" } }),
    [semanticModule("b", "0.2.0", {})],
  ],
  [
    "version-mismatch",
    semanticModule("a", "0.1.0", { imports: { "agent-ix/b": "0.2.0" } }),
    [semanticModule("b", "0.3.0", {})],
  ],
  [
    "absent",
    semanticModule("a", "0.1.0", { imports: { "agent-ix/b": "0.2.0" } }),
    [],
  ],
  [
    "two-absent",
    semanticModule("a", "0.1.0", {
      imports: { "agent-ix/b": "0.2.0", "agent-ix/c": "0.3.0" },
    }),
    [],
  ],
  [
    "self-cycle",
    semanticModule("a", "0.1.0", { imports: { "agent-ix/a": "0.1.0" } }),
    [],
  ],
  [
    "two-cycle",
    semanticModule("a", "0.1.0", { imports: { "agent-ix/b": "0.2.0" } }),
    [semanticModule("b", "0.2.0", { imports: { "agent-ix/a": "0.1.0" } })],
  ],
  [
    "three-cycle",
    semanticModule("a", "0.1.0", { imports: { "agent-ix/b": "0.2.0" } }),
    [
      semanticModule("b", "0.2.0", { imports: { "agent-ix/c": "0.3.0" } }),
      semanticModule("c", "0.3.0", { imports: { "agent-ix/a": "0.1.0" } }),
    ],
  ],
  [
    "diamond-no-cycle",
    semanticModule("a", "0.1.0", {
      imports: { "agent-ix/b": "0.2.0", "agent-ix/c": "0.3.0" },
    }),
    [
      semanticModule("b", "0.2.0", { imports: { "agent-ix/d": "0.4.0" } }),
      semanticModule("c", "0.3.0", { imports: { "agent-ix/d": "0.4.0" } }),
      semanticModule("d", "0.4.0", {}),
    ],
  ],
  [
    "no-imports",
    semanticModule("a", "0.1.0", {}),
    [semanticModule("b", "0.2.0", {})],
  ],
];

emit(semanticGoldens, "resolve-imports.json", {
  source: "src/semantic/package-manifest.ts resolveImports()",
  counts: { total: importCases.length },
  cases: importCases.map(([id, candidate, installed]) => ({
    id,
    candidate: {
      name: candidate.name,
      version: candidate.version,
      block: candidate.block,
    },
    installed: installed.map((m) => ({
      name: m.name,
      version: m.version,
      block: m.block,
    })),
    diagnostics: packageManifestModule.resolveImports(candidate, installed),
  })),
});

const duplicateCases = [
  [
    "no-duplicate",
    semanticModule("a", "0.1.0", {}),
    [semanticModule("b", "0.2.0", {})],
  ],
  [
    "duplicate",
    { ...semanticModule("a", "0.1.0", {}), root: "/modules/a2" },
    [semanticModule("a", "0.1.0", {})],
  ],
  [
    "same-root-is-not-duplicate",
    semanticModule("a", "0.1.0", {}),
    [semanticModule("a", "0.1.0", {})],
  ],
];

emit(semanticGoldens, "duplicate-package.json", {
  source: "src/semantic/manifest.ts duplicatePackageDiagnostic()",
  counts: { total: duplicateCases.length },
  cases: duplicateCases.map(([id, candidate, installed]) => ({
    id,
    candidate: {
      name: candidate.name,
      root: candidate.root,
      block: candidate.block,
    },
    installed: installed.map((m) => ({
      name: m.name,
      root: m.root,
      block: m.block,
    })),
    diagnostic:
      manifestModule.duplicatePackageDiagnostic(candidate, installed) ?? null,
  })),
});

const identityCases = [
  ["agent-ix/spec-objects", "entity"],
  ["a/b", "X"],
  ["org.with.dots/repo-name", "Type_1"],
];

emit(semanticGoldens, "identities.json", {
  source: "src/semantic/package-manifest.ts typeIdentity() / mappingIdentity()",
  counts: { total: identityCases.length },
  cases: identityCases.map(([pkg, name]) => ({
    package: pkg,
    name,
    typeIdentity: packageManifestModule.typeIdentity(pkg, name),
    mappingIdentity: packageManifestModule.mappingIdentity(pkg, name),
  })),
});

emit(semanticGoldens, "contract.json", {
  source: "src/semantic/contract.ts",
  contract: contract.SEMANTIC_CONTRACT,
  semanticCoreBundleDigest: contract.semanticCoreBundleDigest(),
  moduleManifestSchemaSha256: contract.fileSha256(
    contract.moduleManifestSchemaPath(),
  ),
  packageManifestSchemaSha256: contract.fileSha256(
    contract.packageManifestSchemaPath(),
  ),
  commonSchemaSha256: contract.fileSha256(contract.commonSchemaPath()),
});

// `classifyDataSchema` in isolation: the three-way form decision.
const classifyCases = [
  ["inline-object", { type: "object" }],
  ["empty-object", {}],
  ["reference", { schema: "a.json", digest: `sha256:${"a".repeat(64)}` }],
  ["reference-missing-digest", { schema: "a.json" }],
  ["reference-missing-schema", { digest: `sha256:${"a".repeat(64)}` }],
  ["reference-with-extra", { schema: "a.json", digest: "x", type: "object" }],
  ["not-an-object", "a.json"],
  ["array", []],
  ["null", null],
  ["number", 7],
];

emit(semanticGoldens, "classify-data-schema.json", {
  source: "src/semantic/data-schema.ts classifyDataSchema()",
  counts: { total: classifyCases.length },
  cases: classifyCases.map(([id, value]) => ({
    id,
    value,
    ...dataSchemaModule.classifyDataSchema(
      value,
      "object_types[x].data_schema",
    ),
  })),
});

// ---------------------------------------------------------------------------
// Behavioural goldens — completeness
// ---------------------------------------------------------------------------

const reasonCases = [
  ["table-row-with-reason", "safety", "| safety | controls no hardware |\n"],
  ["table-row-backticked", "safety", "| `safety` | controls no hardware |\n"],
  [
    "table-row-case-insensitive",
    "Safety",
    "| safety | controls no hardware |\n",
  ],
  ["dash-non-answer", "safety", "| safety | - |\n"],
  ["emdash-non-answer", "safety", "| safety | — |\n"],
  ["na-non-answer", "safety", "| safety | n/a |\n"],
  ["tbd-non-answer", "safety", "| safety | TBD |\n"],
  ["none-non-answer", "safety", "| safety | none |\n"],
  ["question-mark", "safety", "| safety | ? |\n"],
  ["empty-cell", "safety", "| safety |  |\n"],
  ["two-words-too-short", "safety", "| safety | not applicable |\n"],
  ["three-words-accepted", "safety", "| safety | no hardware here |\n"],
  ["trailing-period-stripped", "safety", "| safety | n/a. |\n"],
  [
    "second-cell-empty-third-real",
    "safety",
    "| safety |  | controls no hardware |\n",
  ],
  [
    "prose-mention-not-a-reason",
    "safety",
    "safety does not apply to this product at all\n",
  ],
  ["wrong-value-row", "safety", "| security | controls no hardware |\n"],
  ["single-cell-row", "safety", "| safety |\n"],
  ["no-table", "safety", "nothing here\n"],
  ["indented-row", "safety", "   | safety | controls no hardware |\n"],
  [
    "multiple-rows-first-match",
    "safety",
    "| safety | - |\n| safety | controls no hardware |\n",
  ],
];

emit(completenessGoldens, "written-reason.json", {
  source: "src/completeness/assess.ts writtenReasonFor()",
  counts: { total: reasonCases.length },
  cases: reasonCases.map(([id, value, body]) => ({
    id,
    value,
    body,
    reason: assessModule.writtenReasonFor(value, body),
  })),
});

const DECL = {
  name: "quality-characteristics",
  from: "NFR",
  field: "quality_attribute",
  check: "vocabulary-coverage",
  justifiedAbsenceField: "quality_attributes_not_applicable",
  values: ["security", "reliability", "safety", "compliance"],
  moduleName: "iso",
};

const assessCases = [
  [
    "all-owned",
    DECL,
    [
      {
        path: "a.md",
        claims: ["security", "reliability"],
        excuses: [],
        body: "",
      },
      { path: "b.md", claims: ["safety", "compliance"], excuses: [], body: "" },
    ],
  ],
  ["none-owned", DECL, []],
  [
    "one-unowned",
    DECL,
    [
      {
        path: "a.md",
        claims: ["security", "reliability", "safety"],
        excuses: [],
        body: "",
      },
    ],
  ],
  [
    "justified-exclusion",
    DECL,
    [
      {
        path: "a.md",
        claims: ["security", "reliability", "compliance"],
        excuses: ["safety"],
        body: "| safety | controls no hardware |\n",
      },
    ],
  ],
  [
    "unjustified-exclusion",
    DECL,
    [
      {
        path: "a.md",
        claims: ["security", "reliability", "compliance"],
        excuses: ["safety"],
        body: "",
      },
    ],
  ],
  [
    "undeclared-exclusion",
    DECL,
    [
      {
        path: "a.md",
        claims: ["security", "reliability", "safety", "compliance"],
        excuses: ["saftey"],
        body: "| saftey | typo here indeed |\n",
      },
    ],
  ],
  [
    "claim-outside-vocabulary-ignored",
    DECL,
    [{ path: "a.md", claims: ["performance"], excuses: [], body: "" }],
  ],
  [
    "excuse-in-two-documents",
    DECL,
    [
      {
        path: "a.md",
        claims: [],
        excuses: ["safety"],
        body: "| safety | controls no hardware |\n",
      },
      { path: "b.md", claims: [], excuses: ["safety"], body: "" },
    ],
  ],
  [
    "everything-excused",
    DECL,
    [
      {
        path: "a.md",
        claims: [],
        excuses: ["security", "reliability", "safety", "compliance"],
        body: "",
      },
    ],
  ],
  ["empty-vocabulary", { ...DECL, values: [] }, []],
  [
    "no-absence-field",
    { ...DECL, justifiedAbsenceField: undefined },
    [{ path: "a.md", claims: ["security"], excuses: [], body: "" }],
  ],
];

emit(completenessGoldens, "assess-vocabulary.json", {
  source: "src/completeness/assess.ts assessVocabulary()",
  counts: { total: assessCases.length },
  cases: assessCases.map(([id, declaration, documents]) => ({
    id,
    declaration,
    documents,
    ...assessModule.assessVocabulary(declaration, documents),
  })),
});

const verdictCases = [];
for (const vocabularies of [0, 1, 3]) {
  for (const strict of [false, true]) {
    for (const [name, findings] of [
      ["none", []],
      [
        "medium-only",
        [
          {
            vocabulary: "v",
            value: "a",
            kind: "unowned",
            severity: "medium",
            message: "m",
          },
        ],
      ],
      [
        "high",
        [
          {
            vocabulary: "v",
            value: "a",
            kind: "unjustified-exclusion",
            severity: "high",
            message: "m",
          },
        ],
      ],
      [
        "mixed",
        [
          {
            vocabulary: "v",
            value: "a",
            kind: "unowned",
            severity: "medium",
            message: "m",
          },
          {
            vocabulary: "v",
            value: "b",
            kind: "undeclared-exclusion",
            severity: "high",
            message: "m",
          },
        ],
      ],
    ]) {
      verdictCases.push({
        id: `v${vocabularies}-${strict ? "strict" : "advisory"}-${name}`,
        vocabulariesChecked: vocabularies,
        strict,
        findings,
        verdict: assessModule.verdictFor(findings, strict, vocabularies),
      });
    }
  }
}

emit(completenessGoldens, "verdict.json", {
  source: "src/completeness/assess.ts verdictFor()",
  counts: { total: verdictCases.length },
  cases: verdictCases,
});

// Frontmatter reading over a real tree.
const bundleFiles = {
  "a.md": "---\nid: NFR-001\nquality_attribute: security\n---\n\nBody A\n",
  "nested/b.md":
    "---\nid: NFR-002\nquality_attribute:\n  - reliability\n  - safety\n---\n\nBody B\n",
  "nested/deep/c.md":
    "---\nid: NFR-003\nquality_attributes_not_applicable: [compliance]\n---\n\n| compliance | not a regulated product |\n",
  "no-frontmatter.md": "# Index\n\nnothing\n",
  "broken.md": "---\nid: [unclosed\n---\n\nbody\n",
  "empty-frontmatter.md": "---\n---\n\nbody\n",
  "crlf.md":
    "---\r\nid: NFR-004\r\nquality_attribute: security\r\n---\r\n\r\nBody\r\n",
  "not-markdown.txt": "---\nquality_attribute: security\n---\n",
  "scalar-frontmatter.md": "---\njust-a-string\n---\n\nbody\n",
  "no-trailing-newline.md":
    "---\nid: NFR-005\nquality_attribute: compliance\n---\nbody",
};

const bundleRoot = writeModule("bundle", bundleFiles);
const frontmatter = bundleModule.readBundleFrontmatter(bundleRoot);

emit(completenessGoldens, "bundle-frontmatter.json", {
  source: "src/completeness/bundle.ts readBundleFrontmatter() / claimsFor()",
  files: bundleFiles,
  documents: frontmatter.documents,
  unreadable: frontmatter.unreadable,
  claims: bundleModule.claimsFor(frontmatter.documents, DECL),
});

// End-to-end: assessBundle over a module tree declaring vocabulary coverage.
const moduleFiles = {
  "manifest.yaml": [
    "name: iso",
    "version: 1.0.0",
    "artifact_types:",
    "- name: NFR",
    "  grammar_ref: g",
    "  frontmatter_schema_ref: schemas/nfr.json",
    "traceability:",
    "  vocabulary_coverage:",
    "  - name: quality-characteristics",
    "    from: NFR",
    "    field: quality_attribute",
    "    check: vocabulary-coverage",
    "    justified_absence_field: quality_attributes_not_applicable",
    "  - name: open-vocabulary",
    "    from: NFR",
    "    field: owner",
    "    check: owner-coverage",
    "  - name: missing-type",
    "    from: ZZZ",
    "    field: whatever",
    "    check: zzz",
    "",
  ].join("\n"),
  "schemas/nfr.json": {
    type: "object",
    properties: {
      quality_attribute: {
        enum: ["security", "reliability", "safety", "compliance"],
      },
      owner: { type: "string" },
    },
  },
};
const moduleRoot = writeModule("iso-module", moduleFiles);

const assessBundleCases = [
  ["advisory", { bundleRoot, strict: false, moduleRoots: [moduleRoot] }],
  ["strict", { bundleRoot, strict: true, moduleRoots: [moduleRoot] }],
  ["no-modules", { bundleRoot, strict: false, moduleRoots: [] }],
  [
    "absent-bundle-root",
    {
      bundleRoot: join(scratch, "does-not-exist"),
      strict: false,
      moduleRoots: [moduleRoot],
    },
  ],
];

emit(completenessGoldens, "assess-bundle.json", {
  source:
    "src/completeness/run.ts assessBundle() + declarations.ts loadVocabularyCoverage()",
  moduleFiles: Object.fromEntries(
    Object.entries(moduleFiles).map(([k, v]) => [
      k,
      typeof v === "string" ? v : JSON.stringify(v, null, 2),
    ]),
  ),
  bundleFiles,
  declarations: declarationsModule.loadVocabularyCoverage([moduleRoot]),
  cases: assessBundleCases.map(([id, options]) => {
    const assessment = runModule.assessBundle(options);
    return {
      id,
      strict: options.strict,
      hasModules: options.moduleRoots.length > 0,
      bundleRootExists: options.bundleRoot === bundleRoot,
      assessment: { ...assessment, bundleRoot: undefined },
    };
  }),
});

rmSync(scratch, { recursive: true, force: true });

console.log(
  `goldens written from quoin@${provenance.quoinRevision} (ajv ${provenance.ajvVersion})`,
);
for (const corpus of schemaCorpora) {
  console.log(
    `  ${corpus.capture.schema}: ${corpus.capture.counts.total} documents ` +
      `(${corpus.capture.counts.valid} valid / ${corpus.capture.counts.invalid} invalid)`,
  );
}
