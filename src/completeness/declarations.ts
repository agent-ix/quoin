/**
 * Declared vocabulary coverage, read from module data (FR-037).
 *
 * quire-rs FR-059 computes which declared values no document claims. This reads
 * the *same declaration* the engine reads, so quoin can say what a finding means
 * — which vocabulary, how many values it has, and where a justified absence may
 * be recorded — without minting a second list.
 *
 * Minting one is the failure this whole area exists to avoid: the 25010
 * characteristic set is **12 values in module data**, and the original ticket
 * proposed walking a hardcoded 9.
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { parse as parseYaml } from "yaml";

import { defaultModuleRoots, locateModuleRoot } from "../catalog.js";

/** One `traceability.vocabulary_coverage` entry, with its values resolved. */
export interface VocabularyDeclaration {
  /** The declaration's own name, e.g. `quality-characteristics`. */
  name: string;
  /** Artifact type the projection reads, e.g. `NFR`. */
  from: string;
  /** Frontmatter field carrying the claim, e.g. `quality_attribute`. */
  field: string;
  /** Corpus-check token the engine reports under. */
  check: string;
  /** Frontmatter field recording a deliberate non-applicability, if declared. */
  justifiedAbsenceField?: string;
  /** The declared vocabulary, read from the artifact type's frontmatter schema. */
  values: string[];
  /** Module that declared it, for collision reporting. */
  moduleName: string;
}

export interface VocabularyDeclarations {
  declarations: VocabularyDeclaration[];
  /** Declarations whose vocabulary could not be resolved, and why. */
  unresolved: Array<{ name: string; reason: string }>;
}

/**
 * Load every module's declared vocabulary coverage.
 *
 * A module declaring none contributes none. A declaration whose values cannot be
 * resolved is **reported, not dropped** — it is the case where the engine emits
 * findings quoin cannot explain, and silence there is worse than an empty list.
 */
export function loadVocabularyCoverage(
  moduleRoots = defaultModuleRoots(),
): VocabularyDeclarations {
  const declarations: VocabularyDeclaration[] = [];
  const unresolved: Array<{ name: string; reason: string }> = [];
  const seenRoots = new Set<string>();

  for (const candidate of moduleRoots) {
    const moduleRoot = locateModuleRoot(candidate);
    if (!moduleRoot || seenRoots.has(moduleRoot)) continue;
    seenRoots.add(moduleRoot);

    let manifest: Record<string, unknown>;
    try {
      manifest = parseYaml(
        readFileSync(join(moduleRoot, "manifest.yaml"), "utf8"),
      ) as Record<string, unknown>;
    } catch {
      // Unreadable modules are the advisor's diagnostic to report; a module that
      // cannot be parsed declares no coverage, which is the honest reading here.
      continue;
    }
    if (!manifest || typeof manifest !== "object") continue;

    const moduleName = String(manifest.name ?? moduleRoot);
    const traceability = manifest.traceability as
      Record<string, unknown> | undefined;
    const entries = traceability?.vocabulary_coverage;
    if (!Array.isArray(entries)) continue;

    for (const raw of entries as Array<Record<string, unknown>>) {
      const name = String(raw.name ?? "");
      const from = String(raw.from ?? "");
      const field = String(raw.field ?? "");
      const resolved = enumFor(manifest, moduleRoot, from, field);
      if (typeof resolved === "string") {
        unresolved.push({ name, reason: resolved });
        continue;
      }
      declarations.push({
        name,
        from,
        field,
        check: String(raw.check ?? ""),
        justifiedAbsenceField:
          typeof raw.justified_absence_field === "string"
            ? raw.justified_absence_field
            : undefined,
        values: resolved,
        moduleName,
      });
    }
  }
  return { declarations, unresolved };
}

/**
 * The declared enum for `<artifactType>.<field>`, or a reason it is unavailable.
 *
 * The vocabulary lives in the artifact type's frontmatter schema — the same
 * place the engine reads it — so the two cannot disagree about how many values
 * exist. A field with no `enum` is an **open** vocabulary: coverage over it is
 * not a finite question, and reporting one would invent a denominator.
 */
function enumFor(
  manifest: Record<string, unknown>,
  moduleRoot: string,
  artifactType: string,
  field: string,
): string[] | string {
  const types = manifest.artifact_types;
  if (!Array.isArray(types)) return `module declares no artifact_types`;
  const declared = (types as Array<Record<string, unknown>>).find(
    (t) => String(t.name ?? "") === artifactType,
  );
  if (!declared) return `no artifact type '${artifactType}' in this module`;
  const ref = declared.frontmatter_schema_ref;
  if (typeof ref !== "string") {
    return `artifact type '${artifactType}' declares no frontmatter schema`;
  }

  let schema: Record<string, unknown>;
  try {
    schema = JSON.parse(readFileSync(join(moduleRoot, ref), "utf8")) as Record<
      string,
      unknown
    >;
  } catch (cause) {
    return `frontmatter schema '${ref}' unreadable: ${
      cause instanceof Error ? cause.message : String(cause)
    }`;
  }

  const properties = schema.properties as Record<string, unknown> | undefined;
  const property = properties?.[field] as Record<string, unknown> | undefined;
  if (!property) return `schema '${ref}' declares no property '${field}'`;
  const values = property.enum;
  if (!Array.isArray(values)) {
    return `property '${field}' declares no enum, so its vocabulary is open`;
  }
  return values.map(String);
}
