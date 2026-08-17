/**
 * The verification-method catalog, merged and readable (FR-031).
 *
 * quire-rs owns the merge for its own consumers; this reads the same manifests
 * from the same module roots so an agent — and this advisor — can reach the
 * catalog without shelling into the engine for a value that is plain module
 * data on disk.
 *
 * Merge is **first-wins by method id**, matching quire-rs FR-054 exactly. If
 * the two disagreed, the advisor would recommend from one catalog while the
 * auditor checked conformance against another.
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { parse as parseYaml } from "yaml";

import { defaultModuleRoots, locateModuleRoot } from "../catalog.js";

/** One catalog entry, as the module declared it. */
export interface VerificationMethod {
  id: string;
  name: string;
  /** IADT here; a free string to the engine, so a free string here. */
  class: string;
  definition: string;
  evidenceKind?: string;
  /**
   * Rule name → values. **Never interpreted structurally** — the advisor
   * matches values, and which axes exist is the declaring module's business
   * (quire-rs FR-054-CON-2).
   */
  applicability: Record<string, string[]>;
  tooling: string[];
  /** Module that contributed this entry (first-wins). */
  moduleName: string;
}

/** The merged catalog plus the collisions the merge skipped. */
export interface MethodCatalog {
  methods: VerificationMethod[];
  /** Method ids more than one module declared, in first-wins order. */
  duplicates: Array<{ id: string; modules: string[] }>;
}

/**
 * Load and merge every module's `verification_catalog`.
 *
 * A module declaring none contributes none, and a catalog nobody declares is
 * an empty list rather than an error — a repository that has not adopted the
 * catalog can still run everything else.
 */
export function loadMethodCatalog(
  moduleRoots = defaultModuleRoots(),
): MethodCatalog {
  const methods = new Map<string, VerificationMethod>();
  const collisions = new Map<string, string[]>();
  const seenRoots = new Set<string>();

  for (const candidate of moduleRoots) {
    const moduleRoot = locateModuleRoot(candidate);
    if (!moduleRoot || seenRoots.has(moduleRoot)) continue;
    seenRoots.add(moduleRoot);

    const manifest = parseYaml(
      readFileSync(join(moduleRoot, "manifest.yaml"), "utf8"),
    ) as Record<string, unknown>;
    const moduleName = String(manifest.name ?? moduleRoot);
    const catalog = manifest.verification_catalog;
    if (!catalog || typeof catalog !== "object") continue;

    for (const [id, raw] of Object.entries(
      catalog as Record<string, Record<string, unknown>>,
    )) {
      const existing = methods.get(id);
      if (existing) {
        // Reported rather than absorbed: two modules disagreeing about what
        // `mutation-testing` means is a collision an operator must see.
        const listed = collisions.get(id) ?? [existing.moduleName];
        listed.push(moduleName);
        collisions.set(id, listed);
        continue;
      }
      methods.set(id, {
        id,
        name: String(raw.name ?? ""),
        class: String(raw.class ?? ""),
        definition: String(raw.definition ?? ""),
        evidenceKind:
          typeof raw.evidence_kind === "string" ? raw.evidence_kind : undefined,
        applicability: normalizeRules(raw.applicability),
        tooling: Array.isArray(raw.tooling) ? raw.tooling.map(String) : [],
        moduleName,
      });
    }
  }

  return {
    methods: [...methods.values()].sort((a, b) => a.id.localeCompare(b.id)),
    duplicates: [...collisions.entries()]
      .map(([id, modules]) => ({ id, modules }))
      .sort((a, b) => a.id.localeCompare(b.id)),
  };
}

function normalizeRules(value: unknown): Record<string, string[]> {
  if (!value || typeof value !== "object") return {};
  const out: Record<string, string[]> = {};
  for (const [rule, values] of Object.entries(
    value as Record<string, unknown>,
  )) {
    if (Array.isArray(values)) out[rule] = values.map(String);
  }
  return out;
}

/** Every distinct `class` in the catalog, sorted — the IADT axis in practice. */
export function methodClasses(catalog: MethodCatalog): string[] {
  return [...new Set(catalog.methods.map((m) => m.class))].sort();
}
