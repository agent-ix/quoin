import type { InterventionExperimentRecord } from "./intervention-types.js";

export interface InterventionReportEntry {
  record_id: string;
  observed_at: string;
  subject: InterventionExperimentRecord["subject"];
  status: InterventionExperimentRecord["status"];
  claims: string[];
  evidence: {
    measured_effects: InterventionExperimentRecord["measured_effects"];
    raw_evidence: InterventionExperimentRecord["raw_evidence"];
  };
  counterevidence: Array<{
    kind: "interaction" | "confounder";
    description: string;
    disposition: "uncontrolled" | "unknown";
  }>;
  gaps: string[];
  owner: string;
  actions: string[];
  producer: InterventionExperimentRecord["producer"];
}

export function buildInterventionReport(
  records: InterventionExperimentRecord[],
): InterventionReportEntry[] {
  return [...records]
    .sort(
      (a, b) =>
        compare(a.observed_at, b.observed_at) ||
        compare(a.record_id, b.record_id),
    )
    .map((record) => ({
      record_id: record.record_id,
      observed_at: record.observed_at,
      subject: record.subject,
      status: record.status,
      claims: [record.conclusion.statement],
      evidence: {
        measured_effects: [...record.measured_effects].sort(
          (a, b) =>
            compare(a.treatment_id, b.treatment_id) ||
            compare(a.metric, b.metric),
        ),
        raw_evidence: [...record.raw_evidence].sort((a, b) =>
          compare(a.path, b.path),
        ),
      },
      counterevidence: [
        ...record.interactions
          .filter(isAdverse)
          .map((item) => ({ kind: "interaction" as const, ...item })),
        ...record.confounders
          .filter(isAdverse)
          .map((item) => ({ kind: "confounder" as const, ...item })),
      ].sort(
        (a, b) =>
          compare(a.kind, b.kind) || compare(a.description, b.description),
      ),
      gaps: [...record.gaps],
      owner: record.owner,
      actions: [...record.actions],
      producer: record.producer,
    }));
}

export function renderInterventionReport(
  entries: InterventionReportEntry[],
): string {
  if (entries.length === 0) return "";
  const lines = ["## Intervention experiments", ""];
  for (const entry of entries) {
    lines.push(`### ${entry.record_id}`, "", "#### Claims", "");
    lines.push(
      ...entry.claims.map((claim) => `- ${claim}`),
      "",
      "#### Evidence",
      "",
    );
    for (const effect of entry.evidence.measured_effects) {
      lines.push(
        `- ${effect.treatment_id}/${effect.metric}: baseline ${String(effect.baseline_value)}, ` +
          `treatment ${String(effect.treatment_value)}, effect ${String(effect.effect)} ${effect.unit}`,
      );
    }
    lines.push(
      ...entry.evidence.raw_evidence.map(
        (raw) =>
          `- ${raw.path} — ${raw.digest}; ${raw.size_bytes} bytes; ${raw.media_type}`,
      ),
      "",
      "#### Counterevidence",
      "",
    );
    lines.push(
      ...(entry.counterevidence.length > 0
        ? entry.counterevidence.map(
            (item) =>
              `- ${item.kind}: ${item.description} (${item.disposition})`,
          )
        : ["No declared counterevidence."]),
      "",
      "#### Gaps",
      "",
      ...(entry.gaps.length > 0
        ? entry.gaps.map((gap) => `- ${gap}`)
        : ["No declared gaps."]),
      "",
      `#### Owner`,
      "",
      entry.owner,
      "",
      "#### Actions",
      "",
      ...entry.actions.map((action) => `- ${action}`),
      "",
    );
  }
  return lines.join("\n");
}

function isAdverse(
  value: InterventionExperimentRecord["interactions"][number],
): value is typeof value & { disposition: "uncontrolled" | "unknown" } {
  return (
    value.disposition === "uncontrolled" || value.disposition === "unknown"
  );
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
