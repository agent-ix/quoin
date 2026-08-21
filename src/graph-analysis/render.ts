/** Deterministic Markdown renderers for FR-045 graph analyses. */

import type {
  ChangeImpactAnalysis,
  ChurnAnalysis,
  FanOutAnalysis,
  GraphLimitation,
} from "./analysis.js";

export type GraphAnalysis =
  FanOutAnalysis | ChangeImpactAnalysis | ChurnAnalysis;

export function renderGraphAnalysis(analysis: GraphAnalysis): string {
  const lines = [`# Trace graph: ${analysis.view}`, ""];
  lines.push(
    analysis.complete
      ? "Graph completeness: **complete**"
      : "Graph completeness: **incomplete** — see unknown inputs and/or limitations below.",
    "",
  );

  if (analysis.view === "fan-out") renderFanOut(analysis, lines);
  if (analysis.view === "change-impact") renderImpact(analysis, lines);
  if (analysis.view === "churn") renderChurn(analysis, lines);
  renderLimitations(analysis.limitations, lines);
  return `${lines.join("\n").trimEnd()}\n`;
}

function renderFanOut(analysis: FanOutAnalysis, lines: string[]): void {
  lines.push("| Suite | Obligation count | Obligations |", "|---|---:|---|");
  if (analysis.rows.length === 0) {
    lines.push("| *(none)* | 0 | — |");
  } else {
    for (const row of analysis.rows) {
      lines.push(
        `| ${cell(row.suite)} | ${row.obligationCount} | ${row.obligations.map(code).join(", ")} |`,
      );
    }
  }
  lines.push("");
}

function renderImpact(analysis: ChangeImpactAnalysis, lines: string[]): void {
  list(lines, "Changed", analysis.changed);
  list(lines, "Unknown inputs", analysis.unknown);
  list(lines, "Downstream documents", analysis.downstreamDocuments);
  list(lines, "Upstream documents", analysis.upstreamDocuments);
  list(lines, "Suspect obligations", analysis.suspectObligations);
  list(lines, "Affected suites", analysis.affectedSuites);
  list(
    lines,
    "Affected implementation",
    analysis.affectedImplementations.map(
      ({ id, requirements }) => `${id} (${requirements.join(", ")})`,
    ),
  );
  list(lines, "Shared-suite exposure", analysis.sharedSuiteExposure);
}

function renderChurn(analysis: ChurnAnalysis, lines: string[]): void {
  lines.push("| Obligation | Distinct affirmation events |", "|---|---:|");
  if (analysis.rows.length === 0) {
    lines.push("| *(none)* | 0 |");
  } else {
    for (const row of analysis.rows) {
      lines.push(`| ${code(row.obligation)} | ${row.affirmationCount} |`);
    }
  }
  lines.push("");
}

function renderLimitations(
  limitations: GraphLimitation[],
  lines: string[],
): void {
  if (limitations.length === 0) return;
  lines.push("## Limitations", "");
  for (const limitation of limitations) {
    const edge = limitation.target ? ` → ${code(limitation.target)}` : "";
    lines.push(
      `- **${limitation.kind}** ${code(limitation.source)}${edge}: ${limitation.reason}`,
    );
  }
  lines.push("");
}

function list(lines: string[], heading: string, values: string[]): void {
  lines.push(`## ${heading}`, "");
  if (values.length === 0) lines.push("*(none)*", "");
  else lines.push(values.map((value) => `- ${code(value)}`).join("\n"), "");
}

function cell(value: string): string {
  return value.replaceAll("|", "\\|").replaceAll("\n", " ");
}

function code(value: string): string {
  return `\`${value.replaceAll("`", "\\`")}\``;
}
