/**
 * The completeness domain, asked of `quoin-core` (quoin#445, FR-037, FR-096).
 *
 * This file is what is left of `src/completeness/` on the TypeScript side: the
 * walk, the reads, and three calls. The analysis — frontmatter parsing,
 * vocabulary resolution, the severity policy and the verdict — lives in
 * `rust/crates/quoin-completeness/` and is reached through the boundary.
 *
 * It is deliberately NOT a re-export surface shaped like the module it
 * replaces. Every caller here goes through `runCore`, so a caller that wants
 * the old synchronous library semantics has to say so rather than get them by
 * an import that happens to still resolve.
 */

import { runCore } from "./exec.js";
import { bundleSnapshot, moduleSnapshots } from "./snapshot.js";
import type {
  BundleAssessment,
  FrontmatterRead,
  SchemaRefsPayload,
} from "./types.js";

import { defaultModuleRoots } from "../module-roots.js";

/**
 * Which frontmatter schemas one module manifest reaches for.
 *
 * The shell asks rather than guesses: module layout is a rule the domain owns,
 * and a second copy of it in the walker would be a rule in two places.
 */
export function schemaRefs(manifest: string): string[] {
  return (
    runCore("completeness.schema_refs", { manifest }) as SchemaRefsPayload
  ).refs;
}

/** What {@link assessBundle} is asked for. */
export interface AssessOptions {
  /** Bundle root — the directory whose documents are read. */
  bundleRoot: string;
  /** Promote an admitted gap to a failing verdict. */
  strict?: boolean;
  /** Module roots to read declarations from; defaults to the installed set. */
  moduleRoots?: string[];
}

/**
 * Assess a bundle against every declared vocabulary.
 *
 * **Zero declarations is reported, not passed** — the far side answers
 * `UNCHECKED` rather than `PASS`, because a repository whose module set
 * declares no vocabulary coverage has not been checked, and printing `PASS`
 * over it is the "green matrix over dead links" result this program exists to
 * stop. The distinction is asserted on the Rust side
 * (`an_assessment_crosses_the_boundary_with_no_filesystem_at_all`), not here.
 */
export function assessBundle(options: AssessOptions): BundleAssessment {
  const { documents, unreadable } = bundleSnapshot(options.bundleRoot);
  const modules = moduleSnapshots(
    options.moduleRoots ?? defaultModuleRoots(),
    schemaRefs,
  );
  return runCore("completeness.assess_bundle", {
    bundle_root: options.bundleRoot,
    strict: options.strict ?? false,
    documents,
    unreadable,
    modules,
  }) as BundleAssessment;
}

/**
 * Every document under `bundleRoot` that carries parseable frontmatter.
 *
 * ONE reader for the whole boundary. The completeness sweep (FR-037) and the
 * assurance case view (FR-040) both need every document's leading `---` block
 * and the body under it; two readers over the same files would drift, and the
 * second would be written by whoever needed a field the first did not expose.
 */
export function readBundleFrontmatter(bundleRoot: string): FrontmatterRead {
  const { documents, unreadable } = bundleSnapshot(bundleRoot);
  return runCore("completeness.read_frontmatter", {
    documents,
    unreadable,
  }) as FrontmatterRead;
}
