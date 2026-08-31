/** Deterministic human and JSON projections of the same FR-062 report. */

import { canonicalJson } from "../evidence/index.js";
import type {
  ChangeImpactAnalysis,
  ChurnAnalysis,
  FanOutAnalysis,
  GraphAnalysis,
} from "./analysis.js";

export function renderGraphAnalysisJson(analysis: GraphAnalysis): string {
  return canonicalJson(analysis);
}

export function renderGraphAnalysis(analysis: GraphAnalysis): string {
  const lines = [
    `# Evidence graph: ${analysis.view}`,
    "",
    `State: **${analysis.state}**`,
    `Source: \`${escapeCode(analysis.source.repository)}@${analysis.source.revision}\``,
    `Export: \`${analysis.export.format} v${analysis.export.format_version}\``,
    "",
    "## Accepted module premises",
    "",
  ];
  if (analysis.premises.modules.length === 0) lines.push("*(none)*", "");
  else {
    for (const module of analysis.premises.modules) {
      lines.push(
        `- \`${escapeCode(module.name)}@${escapeCode(module.version)}\`: ${module.schemas.length} schema(s)`,
      );
    }
    lines.push("");
  }

  if (analysis.view === "fan-out") renderFanOut(analysis, lines);
  else if (analysis.view === "change-impact") renderImpact(analysis, lines);
  else renderChurn(analysis, lines);
  renderGaps(analysis, lines);
  return `${lines.join("\n").trimEnd()}\n`;
}

function renderFanOut(analysis: FanOutAnalysis, lines: string[]): void {
  lines.push(
    "## Fan-out",
    "",
    "| Suite | Live obligations | Count | Unresolved |",
    "| --- | --- | ---: | --- |",
  );
  if (analysis.rows.length === 0) lines.push("| *(none)* | — | 0 | — |");
  else {
    for (const row of analysis.rows) {
      const obligations = row.obligations
        .map(
          ({ obligation, requirements }) =>
            `${obligation} (${requirements.join(", ") || "owner unknown"})`,
        )
        .join(", ");
      lines.push(
        `| ${cell(row.suite)} | ${cell(obligations) || "—"} | ${row.obligationCount} | ${cell(row.unresolvedBindings.join(", ")) || "—"} |`,
      );
    }
  }
  lines.push("");
}

function renderImpact(analysis: ChangeImpactAnalysis, lines: string[]): void {
  lines.push(
    "## Change impact",
    "",
    `Requested: ${analysis.requested.map(code).join(", ") || "*(none)*"}`,
    `Relations: ${analysis.relationKinds.map(code).join(", ") || "*(none)*"}`,
    "",
    "| Requirement | Depth | Seed | Obligations |",
    "| --- | ---: | --- | --- |",
  );
  if (analysis.rows.length === 0) lines.push("| *(none)* | — | — | — |");
  else {
    for (const row of analysis.rows) {
      const obligations = row.obligations
        .map(({ obligation }) => obligation)
        .join(", ");
      lines.push(
        `| ${cell(row.requirement)} | ${row.depth} | ${cell(row.path.seed)} | ${cell(obligations) || "—"} |`,
      );
    }
  }
  lines.push("");
}

function renderChurn(analysis: ChurnAnalysis, lines: string[]): void {
  lines.push(
    "## Retained reaffirmation history",
    "",
    "| Obligation | Requirements | Suites | Events |",
    "| --- | --- | --- | ---: |",
  );
  if (analysis.rows.length === 0) lines.push("| *(none)* | — | — | 0 |");
  else {
    for (const row of analysis.rows) {
      lines.push(
        `| ${cell(row.obligation)} | ${cell(row.requirements.join(", ")) || "—"} | ${cell(row.suites.join(", ")) || "—"} | ${row.eventCount} |`,
      );
    }
  }
  lines.push("");
}

function renderGaps(analysis: GraphAnalysis, lines: string[]): void {
  if (analysis.gaps.length === 0) return;
  lines.push("## Gaps", "");
  for (const gap of analysis.gaps) {
    lines.push(`- **${gap.kind}** ${code(gap.subject)}: ${gap.reason}`);
  }
  lines.push("");
}

function cell(value: string): string {
  return value.replaceAll("|", "\\|").replaceAll("\n", " ");
}

function code(value: string): string {
  return `\`${escapeCode(value)}\``;
}

function escapeCode(value: string): string {
  return value.replaceAll("`", "\\`");
}
