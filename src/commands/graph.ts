import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { readBundleFrontmatter } from "../completeness/index.js";
import { readBindings } from "../evidence/index.js";
import {
  analyzeChangeImpact,
  analyzeChurn,
  analyzeFanOut,
  buildTraceGraph,
  renderGraphAnalysis,
  type GraphAnalysis,
} from "../graph-analysis/index.js";
import {
  checkVersionPremise,
  parseCoverage,
  quireVersion,
  runQuire,
} from "../quire/index.js";

export default class Graph extends QuoinCommand {
  static summary =
    "Analyze trace fan-out, change impact, and requirement churn.";
  static description = `Read-only analyses over the authored relationship graph and evidence bindings
(FR-045). The command runs no verification and writes nothing.

fan-out counts the distinct obligations discharged by each suite. change-impact
walks both directions: downstream documents and obligations become suspect,
while upstream claims are review context. churn counts distinct obligation-level
re-affirmation events; it does not multiply one affirmation by the number of
suites it was copied onto.

An incomplete graph is labelled INCOMPLETE and carries limitations. The command
does not silently treat unresolved targets, unreadable documents, stale bindings,
or relationship verbs with unknown impact direction as a complete closure.`;

  static examples = [
    "quoin graph --view fan-out",
    "quoin graph --view change-impact --changed FR-030",
    "quoin graph --view change-impact --changed FR-030 --changed unit-tests",
    "quoin graph --view churn --json",
  ];

  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    module: Flags.string({
      description: "Module directory supplying the traceability model.",
    }),
    view: Flags.string({
      description: "Analysis to render.",
      options: ["fan-out", "change-impact", "churn"],
      required: true,
    }),
    changed: Flags.string({
      description:
        "Changed document, obligation, or suite id. Repeatable and required " +
        "for change-impact.",
      multiple: true,
    }),
    json: Flags.boolean({ description: "Emit the analysis as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Graph);
    if (flags.view === "change-impact" && !flags.changed?.length) {
      this.error("--view change-impact requires at least one --changed id", {
        exit: 2,
      });
    }
    if (flags.view !== "change-impact" && flags.changed?.length) {
      this.error("--changed is only valid with --view change-impact", {
        exit: 2,
      });
    }

    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const args = ["coverage", "--scope", flags.repo, "--json"];
    if (flags.module) args.push("--module", flags.module);
    const parsed = parseCoverage(runQuire(args));
    if (!parsed.ok) this.error(parsed.error.message, { exit: 2 });

    const bindings = readBindings(flags.repo).bindings;
    const bundle = readBundleFrontmatter(`${flags.repo}/spec`);
    const graph = buildTraceGraph({
      documents: bundle.documents,
      obligations: parsed.value.obligations ?? [],
      bindings,
      unreadable: bundle.unreadable,
    });

    let analysis: GraphAnalysis;
    if (flags.view === "fan-out") analysis = analyzeFanOut(graph);
    else if (flags.view === "churn") analysis = analyzeChurn(graph, bindings);
    else analysis = analyzeChangeImpact(graph, flags.changed ?? []);

    this.log(
      flags.json
        ? JSON.stringify(analysis, null, 2)
        : renderGraphAnalysis(analysis),
    );
  }
}
