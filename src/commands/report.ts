import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { canonicalJson } from "../evidence/store.js";
import {
  buildMeasurementReport,
  buildPortfolioReport,
  comparisonFor,
  renderPortfolioReport,
  renderPortfolioReportJson,
  renderMeasurementComparison,
  renderMeasurementReport,
  renderMeasurementReportJson,
  seriesFor,
} from "../measurement/index.js";

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
    if (flags.since && flags.series) {
      this.error("--since and --series are mutually exclusive", { exit: 2 });
    }
    try {
      if (flags.portfolio?.length) {
        if (flags.since || flags.series) {
          this.error(
            "--portfolio cannot be combined with --since or --series",
            { exit: 2 },
          );
        }
        const portfolio = buildPortfolioReport(flags.portfolio);
        this.log(
          flags.format === "json"
            ? renderPortfolioReportJson(portfolio).trimEnd()
            : renderPortfolioReport(portfolio),
        );
        return;
      }
      if (flags.series) {
        const value = seriesFor(flags.repo, flags.series);
        this.log(
          flags.format === "json"
            ? canonicalJson(value).trimEnd()
            : renderSeries(
                flags.series,
                value as Array<Record<string, unknown>>,
              ),
        );
        return;
      }
      if (flags.since) {
        const comparison = comparisonFor(flags.repo, flags.since);
        this.log(
          flags.format === "json"
            ? canonicalJson(comparison).trimEnd()
            : renderMeasurementComparison(comparison),
        );
        return;
      }
      const report = buildMeasurementReport(flags.repo);
      this.log(
        flags.format === "json"
          ? renderMeasurementReportJson(report).trimEnd()
          : renderMeasurementReport(report),
      );
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.error(detail, { exit: 2 });
    }
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
        `(source ${String(row.sourceRevision)}, tool ${String(row.toolVersion)})`,
    );
  }
  lines.push("");
  return lines.join("\n");
}
