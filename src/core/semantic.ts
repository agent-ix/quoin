import { SEMANTIC_ROOT } from "../semantic/root.js";
import { carriesPayload, runCoreAllowFailure } from "./exec.js";
import type {
  MigrationExamplePayload,
  ModuleSemanticView,
  ReadBlocksPayload,
  ReadBlocksRequest,
  SweepCorpusPayload,
  SweepCorpusRequest,
  SweepReport,
} from "./types.js";

/**
 * The semantic contract, read across the `quoin-core` boundary (quoin#452,
 * Wave 2 of quoin#373).
 *
 * Replaces `src/semantic/{contract,data-schema,index,manifest,package-manifest,
 * sweep}.ts`. Everything they decided — how a manifest's `semantic` block
 * parses and what it defaults to, which diagnostics reading it produces, how a
 * Markdown artifact's Properties form classifies, and what the FR-074 migration
 * guidance says — is now decided by `quoin-semantic`, which the `golden_parity`
 * suite already pinned against those files before they were deleted.
 *
 * # Why these ops carry no documents
 *
 * The vendored contract is 35 JSON files and a corpus sweep walks whole
 * repositories; neither fits on stdin. So the capability is granted rather than
 * transported: `quoin-core`'s `main.rs` — the one file outside
 * `tc_library_containment.rs`'s audit — compiles the validators and hands them
 * to the operation, which still names no filesystem of its own. See
 * `quoin_core::capabilities::SemanticHost`.
 */

export type {
  ModuleSemanticView,
  SemanticBlock,
  SemanticDiagnostic,
  SweepReport,
} from "./types.js";

/**
 * The vendored semantic contract, published to the subprocess.
 *
 * `??=` rather than an assignment so a caller — a test pointing at a fixture
 * contract, an operator debugging one — keeps whatever it set. The same
 * publication `src/core/modules.ts` performs, from the same constant.
 */
function publishSemanticRoot(): void {
  process.env.QUOIN_SEMANTIC_ROOT ??= SEMANTIC_ROOT;
}

/** One boundary call, with the failure surfaced rather than swallowed. */
function call<T>(op: string, request: unknown): T {
  publishSemanticRoot();
  const result = runCoreAllowFailure(op, request);
  if (!carriesPayload(result.exitCode)) {
    const detail = result.diagnostics
      .map((d) => `${d.code}: ${d.message}`)
      .join("\n");
    throw new Error(
      `quoin-core ${op} exited ${result.exitCode}` +
        (detail ? `:\n${detail}` : " with no diagnostic on stderr."),
    );
  }
  return result.payload as T;
}

/**
 * Every module root's `semantic` block and the diagnostics reading it produced,
 * in the order asked.
 *
 * One call for the whole module set, not one per module: `loadCatalog` reads
 * every installed module on every `quoin write`, and a subprocess per module
 * would make the cost of the boundary proportional to the module set. An empty
 * list answers without crossing at all.
 */
export function readSemanticBlocks(roots: string[]): ModuleSemanticView[] {
  if (roots.length === 0) return [];
  return call<ReadBlocksPayload>("semantic.read_blocks", {
    roots,
  } satisfies ReadBlocksRequest).modules;
}

/**
 * Classify every Markdown artifact's Properties form across corpus roots
 * (FR-074).
 *
 * `generatedAt` is the caller's clock and travels in the request: a timestamp
 * minted inside the boundary could not be asserted by a test, and
 * `quoin_semantic::sweep_corpus` takes the same parameter for the same reason.
 */
export function sweepCorpus(
  roots: SweepCorpusRequest["roots"],
  identity: { package: string; version: string },
  generatedAt: string = new Date().toISOString(),
): SweepReport {
  return call<SweepCorpusPayload>("semantic.sweep_corpus", {
    roots,
    package: identity.package,
    version: identity.version,
    generated_at: generatedAt,
  } satisfies SweepCorpusRequest).report;
}

/**
 * The FR-074 migration guidance, as `quoin write` prints it.
 *
 * Served by the boundary rather than restated here: it is the same string every
 * sweep finding is judged against, and two copies in two languages is exactly
 * what the cutover removes. The operation needs no capability — it reads
 * nothing — so it answers without a vendored contract.
 */
export function migrationExample(): string {
  return call<MigrationExamplePayload>("semantic.migration_example", {})
    .example;
}
