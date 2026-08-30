import type { OperationalEvidenceRecord } from "./operational-types.js";

export interface OperationalReportEntry {
  record_id: string;
  observed_at: string;
  record_shape: OperationalEvidenceRecord["record_shape"];
  control_kind: OperationalEvidenceRecord["control_kind"];
  claims: string[];
  evidence: string[];
  counterevidence: string[];
  gaps: string[];
  owner: string;
  actions: string[];
  raw_evidence: OperationalEvidenceRecord["raw_evidence"];
}

export function buildOperationalReport(
  records: OperationalEvidenceRecord[],
): OperationalReportEntry[] {
  return [...records]
    .sort(
      (a, b) =>
        compare(a.observed_at, b.observed_at) ||
        compare(a.record_id, b.record_id),
    )
    .map((record) => {
      const claims: string[] = [];
      const evidence: string[] = [];
      const counterevidence: string[] = [];
      const gaps = [...record.gaps];
      if (record.record_shape === "standing_capability") {
        if (record.capability.status === "available") {
          claims.push(
            `${record.control_kind} control ${record.capability.control_id} is available for ${record.scope.service}`,
          );
          evidence.push(
            `surface ${record.capability.surface}; coverage ${record.capability.coverage}`,
          );
        } else if (record.capability.status === "unknown") {
          gaps.push(
            `capability state is unknown for ${record.capability.control_id}`,
          );
        } else {
          counterevidence.push(
            `${record.capability.control_id} capability is ${record.capability.status}`,
          );
        }
      } else {
        const clock = record.exercise.clock;
        if (
          record.exercise.outcome === "succeeded" &&
          (clock.applicability === "not_applicable" || clock.status === "met")
        ) {
          claims.push(
            `${record.control_kind} control ${record.exercise.control_id} was exercised successfully`,
          );
          evidence.push(
            `${record.exercise.mode} exercise completed ${record.exercise.completed_at}; clock ${clock.status}`,
          );
        } else if (clock.status === "open" || clock.status === "unknown") {
          gaps.push(
            `${record.exercise.control_id} exercise is ${record.exercise.outcome}; clock ${clock.status}`,
          );
        } else {
          counterevidence.push(
            `${record.exercise.control_id} exercise is ${record.exercise.outcome}; clock ${clock.status}`,
          );
        }
      }
      return {
        record_id: record.record_id,
        observed_at: record.observed_at,
        record_shape: record.record_shape,
        control_kind: record.control_kind,
        claims,
        evidence,
        counterevidence,
        gaps,
        owner: record.owner,
        actions: [...record.actions],
        raw_evidence: [...record.raw_evidence].sort((a, b) =>
          compare(a.path, b.path),
        ),
      };
    });
}

export function renderOperationalReport(
  entries: OperationalReportEntry[],
): string {
  if (entries.length === 0) return "";
  const lines = ["## Operational evidence", ""];
  for (const entry of entries) {
    lines.push(`### ${entry.record_id}`, "");
    for (const [heading, values, empty] of [
      ["Claims", entry.claims, "No affirmative claim."],
      [
        "Evidence",
        [
          ...entry.evidence,
          ...entry.raw_evidence.map((raw) => `${raw.path} — ${raw.digest}`),
        ],
        "No affirmative evidence.",
      ],
      [
        "Counterevidence",
        entry.counterevidence,
        "No declared counterevidence.",
      ],
      ["Gaps", entry.gaps, "No declared gaps."],
    ] as const) {
      lines.push(
        `#### ${heading}`,
        "",
        ...(values.length ? values.map((item) => `- ${item}`) : [empty]),
        "",
      );
    }
    lines.push(
      "#### Owner",
      "",
      entry.owner,
      "",
      "#### Actions",
      "",
      ...entry.actions.map((item) => `- ${item}`),
      "",
    );
  }
  return lines.join("\n");
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
