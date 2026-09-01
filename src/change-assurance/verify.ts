import { blake3Hex, digestValue } from "./integrity.js";
import {
  array,
  compareUtf16,
  digest,
  exact,
  identity,
  nonempty,
  object,
  validateDecision,
  verifyChangeRecord,
  verifyLineage,
} from "./records.js";
import { verifyAttestation } from "./attestations.js";
import type {
  Check,
  Outcome,
  Reason,
  RetainedAuditInput,
  VerificationInput,
  VerificationReceipt,
} from "./types.js";

export function verifyChangeAssurance(
  input: VerificationInput,
): VerificationReceipt {
  const recordReasons: Reason[] = [];
  try {
    verifyChangeRecord(input.record);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (/digest mismatch/i.test(message))
      recordReasons.push("record_digest_mismatch");
    else return schemaInvalidReceipt(input);
  }
  const recordCheck = check(recordReasons, "invalid");

  const lineageReasons: Reason[] = [];
  let parentDigests: string[] = [];
  try {
    parentDigests = verifyLineage(input.record, input.parents);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (/missing|no parent/i.test(message))
      lineageReasons.push("parent_missing");
    else if (/revision gap/i.test(message)) lineageReasons.push("revision_gap");
    else if (/mismatch|cross-record/i.test(message))
      lineageReasons.push("parent_mismatch");
    else lineageReasons.push("parent_invalid");
  }
  const lineageCheck = check(lineageReasons, "invalid");

  const decision = validateDecision(input.record, input.decision_history);
  const reviewCheck: Check = {
    outcome: decision.outcome,
    reasons: decision.reasons,
  };

  const impactReasons: Reason[] = [];
  if (input.record.impact_snapshot.completeness === "incomplete")
    impactReasons.push("impact_incomplete");
  if (input.record.impact_snapshot.truncated)
    impactReasons.push("impact_truncated");
  if (
    input.record.definition.unknowns.some(
      (unknown) => unknown.disposition !== "resolved",
    )
  ) {
    impactReasons.push("unresolved_unknown");
  }
  const impactCheck = check(impactReasons, "incomplete");

  const selections = group(input.selections, (selection) => selection.proof_id);
  const attestationsByDigest = group(
    input.attestations,
    (pair) => pair.attestation?.digest ?? "",
  );
  const auditsByProof = group(input.audits, (audit) => audit.proof_id);
  const proofIds = new Set(
    input.record.definition.proof_obligations.map((proof) => proof.proof_id),
  );
  const globalReasons: Reason[] = [];
  if (input.selections.some((selection) => !proofIds.has(selection.proof_id))) {
    globalReasons.push("proof_id_mismatch");
  }

  const proofs = input.record.definition.proof_obligations
    .slice()
    .sort((left, right) => compareUtf16(left.proof_id, right.proof_id))
    .map((proof) => {
      const selected = selections.get(proof.proof_id) ?? [];
      const reasons: Reason[] = [];
      if (selected.length === 0) reasons.push("attestation_missing");
      if (selected.length > 1) reasons.push("attestation_schema_invalid");
      const retainedPairs =
        selected.length === 1
          ? (attestationsByDigest.get(selected[0].attestation_digest) ?? [])
          : [];
      if (retainedPairs.length > 1) reasons.push("attestation_schema_invalid");
      const pair = retainedPairs.length === 1 ? retainedPairs[0] : undefined;
      if (selected.length === 1 && retainedPairs.length === 0)
        reasons.push("attestation_missing");

      if (pair) {
        let validAttestation = true;
        try {
          verifyAttestation(pair.attestation);
        } catch (error) {
          validAttestation = false;
          if (
            /digest/i.test(
              error instanceof Error ? error.message : String(error),
            )
          ) {
            reasons.push("attestation_digest_mismatch");
          } else {
            reasons.push("attestation_schema_invalid");
          }
        }
        if (validAttestation) {
          if (pair.attestation.record_digest !== input.record.digest)
            reasons.push("record_binding_mismatch");
          if (pair.attestation.candidate_revision !== input.candidate_revision)
            reasons.push("candidate_revision_mismatch");
          if (pair.attestation.proof_id !== proof.proof_id)
            reasons.push("proof_id_mismatch");
          if (
            JSON.stringify(pair.attestation.command.argv) !==
              JSON.stringify(proof.command.argv) ||
            pair.attestation.command.working_directory !==
              proof.command.working_directory
          )
            reasons.push("command_mismatch");
          if (pair.attestation.tool.identity !== proof.tool_identity)
            reasons.push("tool_identity_mismatch");
          if (
            pair.attestation.tool.configuration_digest !==
            proof.configuration_digest
          ) {
            reasons.push("configuration_mismatch");
          }
          if (pair.output === null) {
            reasons.push("output_missing");
          } else if (
            pair.attestation.retained_output.size_bytes !==
              pair.output.byteLength ||
            pair.attestation.retained_output.digest !== blake3Hex(pair.output)
          ) {
            reasons.push("output_digest_mismatch");
          }
          if (pair.attestation.result === "failed")
            reasons.push("result_failed");
          if (pair.attestation.result === "unavailable")
            reasons.push("result_unavailable");
          if (pair.attestation.result === "not_computed")
            reasons.push("result_not_computed");
        }
      }

      const retainedAudits = auditsByProof.get(proof.proof_id) ?? [];
      let audit: ReturnType<typeof adaptAudit> | null = null;
      if (retainedAudits.length === 1) {
        try {
          audit = adaptAudit(retainedAudits[0], proof.obligation_ids);
        } catch {
          reasons.push("audit_finding");
        }
      }
      if (retainedAudits.length > 1) reasons.push("audit_finding");
      if (!audit || audit.state === "not_evaluated") {
        reasons.push("audit_not_evaluated");
      } else {
        for (const obligation of proof.obligation_ids) {
          if (!audit.healthy_obligation_ids.includes(obligation))
            reasons.push("audit_finding");
        }
        for (const finding of audit.findings)
          reasons.push(mapAuditFinding(finding.kind));
      }

      const orderedReasons = uniqueReasons(reasons);
      return {
        proof_id: proof.proof_id,
        obligation_ids: proof.obligation_ids.slice().sort(),
        attestation_digest:
          pair?.attestation.digest ?? selected[0]?.attestation_digest ?? null,
        retained_output_digest:
          pair?.attestation?.retained_output?.digest ?? null,
        audit_report_digest: audit?.report_digest ?? null,
        audit_findings: (audit?.findings ?? [])
          .slice()
          .sort(
            (left, right) =>
              compareUtf16(left.obligation_id, right.obligation_id) ||
              compareUtf16(left.kind, right.kind),
          ),
        outcome: outcomeForReasons(orderedReasons),
        reasons: orderedReasons,
      };
    });

  const reasons = uniqueReasons([
    ...globalReasons,
    ...recordCheck.reasons,
    ...lineageCheck.reasons,
    ...reviewCheck.reasons,
    ...impactCheck.reasons,
    ...proofs.flatMap((proof) => proof.reasons),
  ]);
  const unsigned: Omit<VerificationReceipt, "digest"> = {
    schema_version: 1,
    record_type: "verification_receipt",
    record_digest: input.record.digest,
    candidate_revision: input.candidate_revision,
    decision_event: decision.event,
    parent_digests: parentDigests,
    checks: {
      record: recordCheck,
      lineage: lineageCheck,
      review: reviewCheck,
      impact: impactCheck,
    },
    proofs,
    unknowns: input.record.definition.unknowns
      .map(({ id, disposition }) => ({ id, disposition }))
      .sort((left, right) => compareUtf16(left.id, right.id)),
    outcome: outcomeForReasons(reasons),
    reasons,
  };
  return verifyReceipt({
    ...unsigned,
    digest: digestValue(unsigned as unknown as Record<string, unknown>),
  });
}

function adaptAudit(
  retained: RetainedAuditInput,
  obligationIds: string[],
): {
  report_digest: string;
  state: "evaluated" | "not_evaluated";
  findings: Array<{ obligation_id: string; kind: string }>;
  healthy_obligation_ids: string[];
} {
  digest(retained.report_digest, "FR-032 report digest");
  if (
    !retained.report ||
    !Array.isArray(retained.report.findings) ||
    !Array.isArray(retained.report.healthy) ||
    !Array.isArray(retained.report.unevaluated)
  ) {
    throw new Error("invalid retained FR-032 report");
  }
  for (const finding of retained.report.findings) {
    nonempty(finding.obligation, "FR-032 finding obligation");
    nonempty(finding.kind, "FR-032 finding kind");
  }
  for (const healthy of retained.report.healthy) {
    nonempty(healthy, "FR-032 healthy obligation");
  }
  const owning = new Set(obligationIds);
  return {
    report_digest: retained.report_digest,
    state: retained.report.unevaluated.some((entry) =>
      owning.has(entry.obligation),
    )
      ? "not_evaluated"
      : "evaluated",
    findings: retained.report.findings
      .filter((finding) => owning.has(finding.obligation))
      .map((finding) => ({
        obligation_id: finding.obligation,
        kind: finding.kind,
      })),
    healthy_obligation_ids: retained.report.healthy.filter((id) =>
      owning.has(id),
    ),
  };
}

function schemaInvalidReceipt(input: VerificationInput): VerificationReceipt {
  const rawRecord = input?.record as unknown;
  const recordObject =
    rawRecord !== null &&
    typeof rawRecord === "object" &&
    !Array.isArray(rawRecord)
      ? (rawRecord as Record<string, unknown>)
      : { retained_record: rawRecord };
  const declaredDigest = recordObject.digest;
  const recordDigest =
    typeof declaredDigest === "string" && /^[a-f0-9]{64}$/.test(declaredDigest)
      ? declaredDigest
      : digestValue({ retained_record: recordObject });
  const candidateRevision =
    typeof input?.candidate_revision === "string" &&
    input.candidate_revision.length > 0
      ? input.candidate_revision
      : "invalid";
  const invalid: Omit<VerificationReceipt, "digest"> = {
    schema_version: 1,
    record_type: "verification_receipt",
    record_digest: recordDigest,
    candidate_revision: candidateRevision,
    decision_event: null,
    parent_digests: [],
    checks: {
      record: { outcome: "invalid", reasons: ["schema_invalid"] },
      lineage: { outcome: "valid", reasons: [] },
      review: { outcome: "valid", reasons: [] },
      impact: { outcome: "valid", reasons: [] },
    },
    proofs: [],
    unknowns: [],
    outcome: "invalid",
    reasons: ["schema_invalid"],
  };
  return verifyReceipt({
    ...invalid,
    digest: digestValue(invalid as unknown as Record<string, unknown>),
  });
}

export function verifyReceipt(value: unknown): VerificationReceipt {
  const root = object(value, "verification receipt");
  exact(root, [
    "schema_version",
    "record_type",
    "digest",
    "record_digest",
    "candidate_revision",
    "decision_event",
    "parent_digests",
    "checks",
    "proofs",
    "unknowns",
    "outcome",
    "reasons",
  ]);
  if (
    root.schema_version !== 1 ||
    root.record_type !== "verification_receipt"
  ) {
    throw new Error("invalid verification receipt schema identity");
  }
  digest(root.digest, "receipt digest");
  digest(root.record_digest, "record digest");
  nonempty(root.candidate_revision, "candidate revision");
  if (root.decision_event !== null) {
    const event = object(root.decision_event, "decision event");
    exact(event, [
      "run_id",
      "event_id",
      "event_hash",
      "chain_tail_hash",
      "recorded_actor",
      "decision",
    ]);
    identity(event.run_id, "decision run_id");
    identity(event.event_id, "decision event_id");
    digest(event.event_hash, "decision event_hash");
    digest(event.chain_tail_hash, "decision chain_tail_hash");
    nonempty(event.recorded_actor, "decision recorded_actor");
    assertOneOf(event.decision, ["approved", "rejected", "revise"], "decision");
  }
  for (const parent of array(root.parent_digests, "parent digests", false)) {
    digest(parent, "parent digest");
  }
  const checks = object(root.checks, "checks");
  exact(checks, ["record", "lineage", "review", "impact"]);
  for (const name of ["record", "lineage", "review", "impact"] as const) {
    validateCheck(checks[name], `checks.${name}`);
  }
  for (const [index, rawProof] of array(
    root.proofs,
    "proofs",
    false,
  ).entries()) {
    const proof = object(rawProof, `proofs[${index}]`);
    exact(proof, [
      "proof_id",
      "obligation_ids",
      "attestation_digest",
      "retained_output_digest",
      "audit_report_digest",
      "audit_findings",
      "outcome",
      "reasons",
    ]);
    identity(proof.proof_id, "proof_id");
    for (const obligation of array(
      proof.obligation_ids,
      "obligation_ids",
      true,
    )) {
      identity(obligation, "obligation_id");
    }
    for (const name of [
      "attestation_digest",
      "retained_output_digest",
      "audit_report_digest",
    ] as const) {
      if (proof[name] !== null) digest(proof[name], name);
    }
    for (const rawFinding of array(
      proof.audit_findings,
      "audit_findings",
      false,
    )) {
      const finding = object(rawFinding, "audit finding");
      exact(finding, ["obligation_id", "kind"]);
      identity(finding.obligation_id, "audit obligation_id");
      nonempty(finding.kind, "audit finding kind");
    }
    validateOutcomeAndReasons(proof);
  }
  for (const rawUnknown of array(root.unknowns, "unknowns", false)) {
    const unknown = object(rawUnknown, "receipt unknown");
    exact(unknown, ["id", "disposition"]);
    identity(unknown.id, "unknown id");
    assertOneOf(
      unknown.disposition,
      ["open", "accepted", "deferred", "resolved"],
      "unknown disposition",
    );
  }
  validateOutcomeAndReasons(root);
  if (root.digest !== digestValue(root)) {
    throw new Error("verification receipt digest mismatch");
  }
  return value as VerificationReceipt;
}

function validateCheck(value: unknown, name: string): void {
  const result = object(value, name);
  exact(result, ["outcome", "reasons"]);
  validateOutcomeAndReasons(result);
}

function validateOutcomeAndReasons(value: Record<string, unknown>): void {
  assertOneOf(value.outcome, ["valid", "invalid", "incomplete"], "outcome");
  const reasons = array(value.reasons, "reasons", false);
  for (const reason of reasons) assertOneOf(reason, ALL_REASONS, "reason");
  const normalized = uniqueReasons(reasons as Reason[]);
  if (
    normalized.length !== reasons.length ||
    normalized.some((reason, index) => reason !== reasons[index])
  ) {
    throw new Error(
      "verification receipt reasons must be unique and lexicographically ordered",
    );
  }
  if (value.outcome !== outcomeForReasons(normalized)) {
    throw new Error(
      "verification receipt outcome disagrees with reason precedence",
    );
  }
}

function assertOneOf(
  value: unknown,
  allowed: readonly unknown[],
  name: string,
): void {
  if (!allowed.includes(value)) {
    throw new Error(`invalid verification receipt ${name}`);
  }
}

function mapAuditFinding(kind: string): Reason {
  if (kind === "stale-evidence") return "evidence_stale";
  if (kind === "suspect-link") return "evidence_suspect";
  if (kind === "vacuous-evidence") return "evidence_vacuous";
  if (kind === "undischarged") return "evidence_unrelated";
  return "audit_finding";
}

function group<T>(items: T[], key: (item: T) => string): Map<string, T[]> {
  const result = new Map<string, T[]>();
  for (const item of items)
    result.set(key(item), [...(result.get(key(item)) ?? []), item]);
  return result;
}

function check(reasons: Reason[], nonInvalid: Outcome = "valid"): Check {
  const ordered = uniqueReasons(reasons);
  return {
    outcome: ordered.length === 0 ? "valid" : nonInvalid,
    reasons: ordered,
  };
}

const ALL_REASONS: readonly Reason[] = [
  "schema_invalid",
  "record_digest_mismatch",
  "parent_missing",
  "parent_invalid",
  "parent_mismatch",
  "revision_gap",
  "impact_incomplete",
  "impact_truncated",
  "unresolved_unknown",
  "decision_missing",
  "event_chain_missing",
  "event_chain_invalid",
  "decision_mismatch",
  "review_rejected",
  "review_revision_requested",
  "attestation_missing",
  "attestation_schema_invalid",
  "attestation_digest_mismatch",
  "output_missing",
  "output_digest_mismatch",
  "record_binding_mismatch",
  "candidate_revision_mismatch",
  "proof_id_mismatch",
  "command_mismatch",
  "tool_identity_mismatch",
  "configuration_mismatch",
  "result_failed",
  "result_unavailable",
  "result_not_computed",
  "evidence_stale",
  "evidence_suspect",
  "evidence_vacuous",
  "evidence_unrelated",
  "audit_finding",
  "audit_not_evaluated",
];

const INCOMPLETE = new Set<Reason>([
  "parent_missing",
  "impact_incomplete",
  "impact_truncated",
  "unresolved_unknown",
  "decision_missing",
  "event_chain_missing",
  "attestation_missing",
  "output_missing",
  "result_unavailable",
  "result_not_computed",
  "audit_not_evaluated",
]);

function outcomeForReasons(reasons: Reason[]): Outcome {
  if (reasons.length === 0) return "valid";
  return reasons.every((reason) => INCOMPLETE.has(reason))
    ? "incomplete"
    : "invalid";
}

function uniqueReasons(reasons: Reason[]): Reason[] {
  return [...new Set(reasons)].sort();
}
