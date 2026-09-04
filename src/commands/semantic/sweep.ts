import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { basename, resolve } from "node:path";

import { Args, Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { sweepCorpus } from "../../semantic/sweep.js";

/** Best-effort `git rev-parse HEAD` for a corpus root; `worktree` when not a repository. */
function revisionOf(root: string): string {
  try {
    return execFileSync("git", ["-C", root, "rev-parse", "HEAD"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return "worktree";
  }
}

export default class SemanticSweep extends QuoinCommand {
  static summary =
    "Classify every Markdown artifact's Properties form across corpus roots (FR-074).";
  static description = `Walks each root, reads Markdown as text (never rewriting it), and reports how
many artifacts use the typed \`Field | Type | Multiplicity | Constraints\` table,
a \`sysml\` fence, or a legacy form (bullet list, free-column table). A module may
cite the report via \`semantic.sweep_report\` before promoting
\`semantic.legacy_forms\` from warning to error. The advisory sweep of quoin#291
runs this over the corpus.`;

  static examples = [
    "quoin semantic sweep --package agent-ix/spec-objects-business --module-version 0.3.0 ../config-service ../quire-rs",
    "quoin semantic sweep --package agent-ix/spec-objects-business --module-version 0.3.0 --out semantic/sweep.json .",
  ];

  static strict = false;

  static args = {
    roots: Args.string({
      description: "Corpus roots to walk (one or more directories).",
      required: true,
    }),
  };

  static flags = {
    package: Flags.string({
      description:
        "Semantic package identity (<org>/<repo>) the report is for.",
      required: true,
    }),
    "module-version": Flags.string({
      description: "Module version the report is for.",
      required: true,
    }),
    out: Flags.string({
      description: "Write the report here instead of stdout.",
    }),
  };

  async run(): Promise<void> {
    const { argv, flags } = await this.parse(SemanticSweep);
    const roots = (argv as string[]).map((root) => resolve(root));
    if (roots.length === 0)
      throw new Error("semantic sweep requires at least one root");
    const report = sweepCorpus(
      roots.map((root) => ({
        root,
        repository: basename(root),
        revision: revisionOf(root),
      })),
      { package: flags.package, version: flags["module-version"] },
    );
    const text = `${JSON.stringify(report, null, 2)}\n`;
    if (flags.out) {
      writeFileSync(flags.out, text);
      this.log(
        `sweep: ${report.counts.artifacts} artifacts, ${report.counts.legacy["bullet-list"]} bullet-list, ${report.counts.legacy["free-column-table"]} free-column-table → ${flags.out}`,
      );
      return;
    }
    this.log(text.trimEnd());
  }
}
