import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { canonicalJson } from "../store/canonical.js";
import { askCore, stringMember } from "./measurement/core.js";

/**
 * Every branch below asks `quoin-core` one question (quoin#478). The `build_*`
 * routes answer with the document this command's `--format json` prints, and
 * the `render_*` routes with the markdown its `human` format prints; which of
 * the two is asked is the format flag, so no report crosses the boundary twice.
 *
 * `renderSeries` stays here: there is no `measurement.render_series` route —
 * porting this helper is the cutover's work (#479), not this wave's wiring.
 */
export default class Report extends QuoinCommand {
  static summary = "Render QA plans and measurements from the evidence store.";
  static description = `A deterministic store view. It accepts no typed values and runs no
measurement producer. Plans with no records remain visible as not_computed.`;
  static examples = [
    "quoin report",
    "quoin report --since abc123 --format json",
    "quoin report --series finding_recall --format json",
    "quoin report --portfolio ../quoin --portfolio ../quire-rs",
  ];
  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    portfolio: Flags.string({
      description: "Repository root to include; repeat for a portfolio view.",
      multiple: true,
    }),
    "graph-export": Flags.string({
      description:
        "Repository=path mapping for an existing Quire assurance export; repeatable.",
      multiple: true,
    }),
    "graph-premises": Flags.string({
      description:
        "Repository=path mapping for accepted graph premises; repeatable.",
      multiple: true,
    }),
    "graph-audit": Flags.string({
      description:
        "Repository=path mapping for a source-bound audit envelope; repeatable.",
      multiple: true,
    }),
    changed: Flags.string({
      description:
        "Repository=requirement mapping for graph change-impact; repeatable.",
      multiple: true,
    }),
    since: Flags.string({
      description: "Compare the named source revision to latest.",
    }),
    series: Flags.string({ description: "Render the history of one metric." }),
    format: Flags.string({
      description: "Output format.",
      options: ["human", "json"],
      default: "human",
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Report);
    const fail = (message: string): never => this.error(message, { exit: 2 });
    if (flags.since && flags.series) {
      fail("--since and --series are mutually exclusive");
    }
    const json = flags.format === "json";
    /** Ask for the document, or for the markdown, and print what came back. */
    const answer = (build: string, render: string, request: unknown): void => {
      const op = json ? build : render;
      const payload = askCore(op, request, fail);
      this.log(
        json
          ? canonicalJson(payload).trimEnd()
          : stringMember(payload, "rendered", op, fail),
      );
    };

    const graphSelected = Boolean(
      flags["graph-export"]?.length ||
      flags["graph-premises"]?.length ||
      flags["graph-audit"]?.length ||
      flags.changed?.length,
    );
    if (graphSelected && !flags.portfolio?.length) {
      fail("graph portfolio mappings require --portfolio");
    }
    if (flags.portfolio?.length) {
      if (flags.since || flags.series) {
        fail("--portfolio cannot be combined with --since or --series");
      }
      if (graphSelected) {
        answer(
          "measurement.build_graph_portfolio",
          "measurement.render_graph_portfolio",
          {
            locations: flags.portfolio,
            graph_exports: flags["graph-export"] ?? [],
            graph_premises: flags["graph-premises"] ?? [],
            graph_audits: flags["graph-audit"] ?? [],
            changed: flags.changed ?? [],
            // The mappings name documents relative to where the operator ran
            // the command, which the subprocess does not inherit as a meaning.
            cwd: process.cwd(),
          },
        );
        return;
      }
      answer("measurement.build_portfolio", "measurement.render_portfolio", {
        locations: flags.portfolio,
      });
      return;
    }
    if (flags.series) {
      const op = "measurement.build_series";
      const payload = askCore(
        op,
        {
          repo: flags.repo,
          metric: flags.series,
        },
        fail,
      );
      this.log(
        json
          ? canonicalJson(payload).trimEnd()
          : renderSeries(
              flags.series,
              payload as Array<Record<string, unknown>>,
            ),
      );
      return;
    }
    if (flags.since) {
      answer("measurement.build_comparison", "measurement.render_comparison", {
        repo: flags.repo,
        before_revision: flags.since,
      });
      return;
    }
    answer("measurement.build_report", "measurement.render_report", {
      repo: flags.repo,
    });
  }
}

function renderSeries(
  metric: string,
  rows: Array<Record<string, unknown>>,
): string {
  const lines = [`# ${metric} series`, ""];
  if (rows.length === 0) return `${lines.join("\n")}not_computed: no records\n`;
  for (const row of rows) {
    const observation = row.observation as {
      value: unknown;
      unit: unknown;
      state: unknown;
    };
    lines.push(
      `- ${String(row.timestamp)} ${String(observation.state)} ` +
        `${String(observation.value)} ${String(observation.unit)} ` +
        `(source ${String(row.sourceRevision)}, tool ${String(row.toolIdentity)} ` +
        `${String(row.toolVersion)}, corpus ${String(row.corpusRevision)}, ` +
        `gaps ${String(row.corpusGaps)}, config ${String(row.configDigest)})`,
    );
  }
  lines.push("");
  return lines.join("\n");
}
