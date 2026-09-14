/**
 * The three `quoin graph` views, asked of `quoin-core` (quoin#500, Stage 7).
 *
 * `src/graph-analysis/` — 1,373 lines across five files — was deleted at this
 * ticket. What it computed is `quoin-graph-analysis`, reached through
 * `graph.fan_out`, `graph.churn` and `graph.change_impact`; what is left on
 * this side is the oclif surface: the flags, the request they become, and the
 * one document the boundary answers with.
 *
 * `askGraph` is deliberately a THIRD `askCore`, not an import of
 * `src/commands/measurement/core.ts`'s or `src/commands/change-assurance/
 * common.ts`'s. Those two are copies of each other on purpose and say so: they
 * are on opposite sides of a retirement, and the measurement copy is deleted
 * when Stage 6 finishes. The graph commands are RETAINED, so importing a helper
 * scheduled for deletion would make that deletion a refactor of live code.
 * Every one of the three is the same six lines over `src/core/exec.ts`, which
 * is the file that actually owns running the subprocess.
 */

import { Flags } from "@oclif/core";

import { carriesPayload, runCoreAllowFailure } from "../../core/exec.js";

/** The three declared inputs and the repository the retained store is under. */
export const graphInputFlags = {
  repo: Flags.string({
    description: "Repository root for retained Quoin state.",
    default: ".",
  }),
  export: Flags.string({
    description: "Existing Quire assurance-v1 JSON artifact.",
    required: true,
  }),
  premises: Flags.string({
    description: "Accepted assurance format/module/schema premises JSON.",
    required: true,
  }),
  audit: Flags.string({
    description: "Source-bound FR-032 audit-envelope JSON.",
    required: true,
  }),
  json: Flags.boolean({ description: "Emit canonical JSON." }),
};

/** What every `graph.*` operation is asked. */
export interface GraphRequest {
  repo: string;
  export_path: string;
  premises_path: string;
  audit_path: string;
  json: boolean;
}

/**
 * The flags oclif parsed, as the boundary spells them.
 *
 * `json` is coerced rather than copied: an unset `Flags.boolean` parses to
 * `undefined`, not `false`, and `JSON.stringify` drops an `undefined` member
 * entirely — so copying it would send a request with no `json` at all, which
 * the boundary refuses as a missing field. The type says `boolean` because
 * oclif's own generated flag type does; the value at runtime does not agree,
 * and the request is what has to be right.
 */
export function graphRequest(flags: {
  repo: string;
  export: string;
  premises: string;
  audit: string;
  json: boolean;
}): GraphRequest {
  return {
    repo: flags.repo,
    export_path: flags.export,
    premises_path: flags.premises,
    audit_path: flags.audit,
    json: flags.json === true,
  };
}

/**
 * Ask `quoin-core` for one rendered graph view.
 *
 * Every refusal the engine reports reaches the caller as this surface's own
 * exit 2, which is the status `loadGraphFlags` already failed with on an
 * unreadable or non-conforming input. The engine's status and its diagnostic
 * codes are named in the message so the distinction is not lost, only re-graded
 * — and the `input` context key the engine sets still says WHICH of the three
 * declared inputs declined.
 *
 * `fail` is the command's `this.error`, which never returns.
 */
export function askGraph(
  op: string,
  request: unknown,
  fail: (message: string) => never,
): string {
  const result = runCoreAllowFailure(op, request);
  if (!carriesPayload(result.exitCode)) {
    const detail = result.diagnostics
      .map((d) => `${d.code}: ${d.message}`)
      .join("\n");
    fail(
      detail ||
        `quoin-core ${op} exited ${result.exitCode} with no diagnostic on stderr.`,
    );
  }
  const rendered =
    result.payload !== null && typeof result.payload === "object"
      ? (result.payload as Record<string, unknown>).rendered
      : undefined;
  if (typeof rendered !== "string") {
    fail(`quoin-core ${op} returned no rendered report`);
  }
  return rendered.trimEnd();
}
