/**
 * `data_schema` by emitted-schema path and digest (FR-073, issue #293).
 *
 * An object type under a module with a `semantic` block references its
 * emitted JSON Schema as `{ schema: <module-relative .json>, digest: sha256:… }`.
 * Quoin resolves the reference at `quoin module install`: the file must sit
 * inside the module root (no `..`, no symlink escape), hash to the recorded
 * digest over its raw bytes, be a JSON Schema 2020-12 document with an absolute
 * `$id` under the module's semantic package base, and every `$ref` it carries
 * must resolve inside the shipped bundle or the vendored semantic-core bundle at
 * the version the manifest records. No network read, ever.
 */

import { createHash } from "node:crypto";
import { existsSync, readFileSync, realpathSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";

import { SCHEMA_DIR } from "./contract.js";

export type Severity = "error" | "warning";

export interface SemanticDiagnostic {
  code: string;
  severity: Severity;
  /** Manifest or file locus, e.g. `object_types[entity].data_schema.schema`. */
  path: string;
  message: string;
}

export interface DataSchemaReference {
  schema: string;
  digest: string;
}

export interface ResolvedDataSchema {
  kind: "inline" | "reference";
  /** The parsed schema document (inline object or referenced file). */
  schema?: Record<string, unknown>;
  /** Absolute path of the referenced file, for the reference form. */
  file?: string;
  diagnostics: SemanticDiagnostic[];
}

const SEMANTIC_CORE_BASE = "https://schemas.agent-ix.org/semantic-core/";
const PACKAGE_BASE = "https://schemas.agent-ix.org/";

function isObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

/** Classify a `data_schema` value: inline object, reference, or ambiguous. */
export function classifyDataSchema(
  value: unknown,
  locus: string,
): {
  form: "inline" | "reference" | "invalid";
  diagnostics: SemanticDiagnostic[];
} {
  if (!isObject(value)) {
    return {
      form: "invalid",
      diagnostics: [
        {
          code: "semantic.data-schema-shape",
          severity: "error",
          path: locus,
          message: "data_schema must be an object",
        },
      ],
    };
  }
  const hasRefKeys = "schema" in value || "digest" in value;
  if (!hasRefKeys) return { form: "inline", diagnostics: [] };
  const extra = Object.keys(value).filter(
    (k) => k !== "schema" && k !== "digest",
  );
  if (extra.length > 0 || !("schema" in value) || !("digest" in value)) {
    return {
      form: "invalid",
      diagnostics: [
        {
          code: "semantic.data-schema-ambiguous",
          severity: "error",
          path: locus,
          message: `data_schema mixes the reference form with other keys (${extra.join(", ") || "missing schema or digest"})`,
        },
      ],
    };
  }
  return { form: "reference", diagnostics: [] };
}

function sha256(bytes: Buffer): string {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

/** True when `candidate` resolves (through symlinks) inside `root`. */
function insideRoot(root: string, candidate: string): boolean {
  const realRoot = realpathSync(root);
  let real: string;
  try {
    real = realpathSync(candidate);
  } catch {
    real = resolve(candidate);
  }
  const rel = relative(realRoot, real);
  return (
    rel !== "" &&
    !rel.startsWith("..") &&
    !isAbsolute(rel) &&
    !rel.startsWith(`..${sep}`)
  );
}

function collectRefs(node: unknown, out: string[]): void {
  if (Array.isArray(node)) {
    for (const child of node) collectRefs(child, out);
  } else if (isObject(node)) {
    if (typeof node.$ref === "string") out.push(node.$ref);
    for (const child of Object.values(node)) collectRefs(child, out);
  }
}

export interface ResolveContext {
  moduleRoot: string;
  /** `<org>/<repo>` from `semantic.package`. */
  packageIdentity: string;
  /** Module `version`. */
  moduleVersion: string;
  /** `semantic.semantic_core`. */
  semanticCore: string;
  objectType: string;
}

/** Resolve one object type's `data_schema` per FR-073. */
export function resolveDataSchema(
  value: unknown,
  ctx: ResolveContext,
  semanticBlockPresent: boolean,
): ResolvedDataSchema {
  const locus = `object_types[${ctx.objectType}].data_schema`;
  const classified = classifyDataSchema(value, locus);
  if (classified.form === "invalid") {
    return { kind: "inline", diagnostics: classified.diagnostics };
  }
  if (classified.form === "inline") {
    const diagnostics: SemanticDiagnostic[] = semanticBlockPresent
      ? [
          {
            code: "semantic.inline-data-schema",
            severity: "warning",
            path: locus,
            message: `object type ${ctx.objectType} still carries an inline data_schema; migrate to { schema, digest } referencing the module's emitted schema (FR-073, FR-074)`,
          },
        ]
      : [];
    return {
      kind: "inline",
      schema: value as Record<string, unknown>,
      diagnostics,
    };
  }

  const ref = value as DataSchemaReference;
  const diagnostics: SemanticDiagnostic[] = [];
  const fail = (
    code: string,
    message: string,
    suffix = "",
  ): ResolvedDataSchema => {
    diagnostics.push({
      code,
      severity: "error",
      path: `${locus}${suffix}`,
      message,
    });
    return { kind: "reference", diagnostics };
  };

  if (typeof ref.schema !== "string" || ref.schema.length === 0) {
    return fail(
      "semantic.data-schema-path",
      "schema must be a module-relative path",
      ".schema",
    );
  }
  if (!/^sha256:[0-9a-f]{64}$/.test(String(ref.digest))) {
    return fail(
      "semantic.data-schema-digest",
      `digest must be sha256:<64 hex>, got ${String(ref.digest)}`,
      ".digest",
    );
  }
  if (isAbsolute(ref.schema) || ref.schema.split(/[\\/]/).includes("..")) {
    return fail(
      "semantic.data-schema-escape",
      `schema path escapes the module root: ${ref.schema}`,
      ".schema",
    );
  }
  const file = resolve(ctx.moduleRoot, ref.schema);
  if (!existsSync(file)) {
    return fail(
      "semantic.data-schema-missing",
      `schema file is missing: ${ref.schema}`,
      ".schema",
    );
  }
  if (!insideRoot(ctx.moduleRoot, file)) {
    return fail(
      "semantic.data-schema-escape",
      `schema path escapes the module root by symlink: ${ref.schema}`,
      ".schema",
    );
  }
  let bytes: Buffer;
  try {
    bytes = readFileSync(file);
  } catch (error) {
    return fail(
      "semantic.data-schema-unreadable",
      `schema file is unreadable: ${ref.schema} (${String(error)})`,
      ".schema",
    );
  }
  const actual = sha256(bytes);
  if (actual !== ref.digest) {
    return fail(
      "semantic.data-schema-digest-mismatch",
      `schema ${ref.schema} hashes to ${actual}, manifest records ${ref.digest}`,
      ".digest",
    );
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(bytes.toString("utf8"));
  } catch {
    return fail(
      "semantic.data-schema-not-json",
      `schema file is not JSON: ${ref.schema}`,
      ".schema",
    );
  }
  if (!isObject(parsed) || typeof parsed.$id !== "string") {
    return fail(
      "semantic.data-schema-not-schema",
      `schema file is not a JSON Schema document with an absolute $id: ${ref.schema}`,
      ".schema",
    );
  }
  const [org, repo] = ctx.packageIdentity.split("/");
  const base = `${PACKAGE_BASE}${org}/${repo}/${ctx.moduleVersion}/`;
  const expectedId = `${base}${ref.schema.split("/").pop()}`;
  if (parsed.$id !== expectedId) {
    return fail(
      "semantic.data-schema-id",
      `schema $id is ${String(parsed.$id)}, expected ${expectedId}`,
      ".schema",
    );
  }
  if (parsed.$schema !== "https://json-schema.org/draft/2020-12/schema") {
    return fail(
      "semantic.data-schema-not-schema",
      `schema file does not declare JSON Schema 2020-12: ${ref.schema}`,
      ".schema",
    );
  }

  // $ref resolution: module bundle (same directory) or vendored semantic-core.
  const coreDir = join(SCHEMA_DIR, "semantic-core");
  const coreBase = `${SEMANTIC_CORE_BASE}${ctx.semanticCore}/`;
  const visited = new Set<string>();
  const stack: string[] = [];
  const walk = (schemaDoc: unknown, fileKey: string): boolean => {
    if (stack.includes(fileKey)) {
      diagnostics.push({
        code: "semantic.schema-ref-cycle",
        severity: "error",
        path: `${locus}.schema`,
        message: `$ref cycle: ${[...stack, fileKey].join(" -> ")}`,
      });
      return false;
    }
    if (visited.has(fileKey)) return true;
    visited.add(fileKey);
    stack.push(fileKey);
    const refs: string[] = [];
    collectRefs(schemaDoc, refs);
    for (const target of refs) {
      const [url] = target.split("#");
      if (!url) continue; // fragment-only
      if (url.startsWith(coreBase)) {
        const name = url.slice(coreBase.length);
        const corePath = join(coreDir, name);
        if (!existsSync(corePath)) {
          diagnostics.push({
            code: "semantic.schema-ref-unshipped",
            severity: "error",
            path: `${locus}.schema`,
            message: `$ref ${url} names no file in the vendored semantic-core ${ctx.semanticCore} bundle`,
          });
          continue;
        }
        if (
          !walk(
            JSON.parse(readFileSync(corePath, "utf8")),
            `semantic-core/${name}`,
          )
        )
          return false;
      } else if (url.startsWith(SEMANTIC_CORE_BASE)) {
        const version = url.slice(SEMANTIC_CORE_BASE.length).split("/")[0];
        diagnostics.push({
          code: "semantic.schema-ref-version",
          severity: "error",
          path: `${locus}.schema`,
          message: `$ref ${url} names semantic-core ${version}, manifest records ${ctx.semanticCore}`,
        });
      } else if (url.startsWith(base)) {
        const name = url.slice(base.length);
        const sibling = join(dirname(file), name);
        if (!existsSync(sibling) || !insideRoot(ctx.moduleRoot, sibling)) {
          diagnostics.push({
            code: "semantic.schema-ref-unshipped",
            severity: "error",
            path: `${locus}.schema`,
            message: `$ref ${url} names no shipped file (${name})`,
          });
          continue;
        }
        let doc: unknown;
        try {
          doc = JSON.parse(readFileSync(sibling, "utf8"));
        } catch {
          diagnostics.push({
            code: "semantic.data-schema-not-json",
            severity: "error",
            path: `${locus}.schema`,
            message: `referenced file is not JSON: ${name}`,
          });
          continue;
        }
        if (!walk(doc, `module/${name}`)) return false;
      } else {
        diagnostics.push({
          code: "semantic.schema-ref-unshipped",
          severity: "error",
          path: `${locus}.schema`,
          message: `$ref ${url} is outside the module bundle and the semantic-core bundle`,
        });
      }
    }
    stack.pop();
    return true;
  };
  walk(parsed, `module/${ref.schema.split("/").pop()}`);
  return { kind: "reference", schema: parsed, file, diagnostics };
}
