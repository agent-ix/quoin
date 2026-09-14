/**
 * The verification-method catalog, merged and readable (FR-031).
 *
 * quire-rs owns the merge for its own consumers; this reads the same manifests
 * from the same module roots so an agent — and the advisor — can reach the
 * catalog without shelling into the engine for a value that is plain module
 * data on disk.
 *
 * Merge is **first-wins by method id**, matching quire-rs FR-054 exactly. If
 * the two disagreed, the advisor would recommend from one catalog while the
 * auditor checked conformance against another. Both read it, so it belongs to
 * neither: the auditor reaching into `advisor/` for the catalog type was one
 * half of an import cycle Cargo forbids (agent-ix/quoin#376).
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { parse as parseYaml } from "yaml";

import { defaultModuleRoots, locateModuleRoot } from "./catalog.js";
import type { MethodCatalog, VerificationMethod } from "./core/types.js";

export type { MethodCatalog, VerificationMethod } from "./core/types.js";

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
  const unreadable: Array<{ moduleRoot: string; reason: string }> = [];

  for (const candidate of moduleRoots) {
    const moduleRoot = locateModuleRoot(candidate);
    if (!moduleRoot || seenRoots.has(moduleRoot)) continue;
    seenRoots.add(moduleRoot);

    // A module root that resolves but has no readable `manifest.yaml` — or one
    // whose YAML is malformed — used to take down `quoin catalog methods`,
    // which is the command an operator runs *to diagnose* module problems
    // (agent-ix/quoin#106). Skipped and reported instead.
    let manifest: Record<string, unknown>;
    try {
      manifest = parseYaml(
        readFileSync(join(moduleRoot, "manifest.yaml"), "utf8"),
      ) as Record<string, unknown>;
    } catch (cause) {
      unreadable.push({
        moduleRoot,
        reason: cause instanceof Error ? cause.message : String(cause),
      });
      continue;
    }
    if (!manifest || typeof manifest !== "object") continue;
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

  // Plain comparison, not `localeCompare`: the merged catalog drives advice and
  // conformance, and ordering must not depend on the runtime's ICU data.
  const byId = (a: { id: string }, b: { id: string }) =>
    a.id === b.id ? 0 : a.id < b.id ? -1 : 1;
  return {
    methods: [...methods.values()].sort(byId),
    duplicates: [...collisions.entries()]
      .map(([id, modules]) => ({ id, modules }))
      .sort(byId),
    unreadable: unreadable.sort((a, b) =>
      a.moduleRoot === b.moduleRoot ? 0 : a.moduleRoot < b.moduleRoot ? -1 : 1,
    ),
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
