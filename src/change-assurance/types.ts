import type { AuditReport } from "../auditor/audit.js";

export type Digest = string;
export type Outcome = "valid" | "invalid" | "incomplete";
export type Reason =
  | "schema_invalid"
  | "record_digest_mismatch"
  | "parent_missing"
  | "parent_invalid"
  | "parent_mismatch"
  | "revision_gap"
  | "impact_incomplete"
  | "impact_truncated"
  | "unresolved_unknown"
  | "decision_missing"
  | "event_chain_missing"
  | "event_chain_invalid"
  | "decision_mismatch"
  | "review_rejected"
  | "review_revision_requested"
  | "attestation_missing"
  | "attestation_schema_invalid"
  | "attestation_digest_mismatch"
  | "output_missing"
  | "output_digest_mismatch"
  | "record_binding_mismatch"
  | "candidate_revision_mismatch"
  | "proof_id_mismatch"
  | "command_mismatch"
  | "tool_identity_mismatch"
  | "configuration_mismatch"
  | "result_failed"
  | "result_unavailable"
  | "result_not_computed"
  | "evidence_stale"
  | "evidence_suspect"
  | "evidence_vacuous"
  | "evidence_unrelated"
  | "audit_finding"
  | "audit_not_evaluated";

export interface CommandBinding {
  argv: string[];
  working_directory: string;
}

export interface ChangeAssuranceRecord {
  schema_version: 1;
  record_type: "change_assurance";
  record_id: string;
  revision: number;
  parent_digest: Digest | null;
  digest: Digest;
  subject: {
    repository: string;
    base_revision: string;
    scope: string[];
  };
  source_connections: Array<{
    source_id: string;
    kind:
      | "requirement"
      | "test"
      | "api"
      | "architecture"
      | "impact_evidence"
      | "recovery_evidence"
      | "other";
    revision: string;
    digest: Digest;
  }>;
  impact_snapshot: {
    identity: string;
    revision: string;
    digest: Digest;
    completeness: "complete" | "incomplete";
    truncated: boolean;
    gaps: string[];
  };
  definition: {
    requirements: Array<{
      id: string;
      statement: string;
      source_ids: string[];
    }>;
    preservation_constraints: Array<{
      id: string;
      statement: string;
      source_ids: string[];
    }>;
    proof_obligations: Array<{
      proof_id: string;
      statement: string;
      obligation_ids: string[];
      evidence_kind: string;
      command: CommandBinding;
      tool_identity: string;
      configuration_digest: Digest;
    }>;
    unknowns: Array<{
      id: string;
      statement: string;
      disposition: "open" | "accepted" | "deferred" | "resolved";
      owner: string;
      resolution?: string;
    }>;
  };
  review_workflow: {
    run_id: string;
    decision_event_kind: "change_assurance.review_decided";
  };
}

export interface ProofAttestation {
  schema_version: 1;
  record_type: "proof_attestation";
  attestation_id: string;
  digest: Digest;
  record_digest: Digest;
  candidate_revision: string;
  proof_id: string;
  command: CommandBinding;
  tool: {
    identity: string;
    version: string;
    configuration_digest: Digest;
  };
  environment: Record<string, string | number | boolean | null>;
  observed_at: string;
  result: "passed" | "failed" | "unavailable" | "not_computed";
  retained_output: {
    media_type: string;
    digest: Digest;
    size_bytes: number;
  };
}

export type JsonValue =
  null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

/** Exact retained ix-flow FR-013 event shape. */
export interface RetainedDecisionEvent {
  id: string;
  ts: string;
  actor: {
    kind: "agent" | "human" | "service";
    id: string;
    ackToken?: string;
  };
  kind: string;
  payload: JsonValue;
  prevHash: Digest;
  hash: Digest;
}

export interface DecisionHistory {
  /** ix-flow workflow instance/run identity retained with its event history. */
  run_id: string;
  events: RetainedDecisionEvent[];
}

/** Retained FR-032 output. Verification adapts it; it never reruns the auditor. */
export interface RetainedAuditInput {
  proof_id: string;
  report_digest: Digest;
  report: AuditReport;
}

export interface VerificationInput {
  record: ChangeAssuranceRecord;
  parents: ChangeAssuranceRecord[];
  candidate_revision: string;
  selections: Array<{ proof_id: string; attestation_digest: Digest }>;
  attestations: Array<{
    attestation: ProofAttestation;
    output: Uint8Array | null;
  }>;
  decision_history: DecisionHistory;
  audits: RetainedAuditInput[];
}

export interface Check {
  outcome: Outcome;
  reasons: Reason[];
}

export interface ReceiptDecisionEvent {
  run_id: string;
  event_id: string;
  event_hash: Digest;
  chain_tail_hash: Digest;
  recorded_actor: string;
  decision: "approved" | "rejected" | "revise";
}

export interface VerificationReceipt {
  schema_version: 1;
  record_type: "verification_receipt";
  digest: Digest;
  record_digest: Digest;
  candidate_revision: string;
  decision_event: ReceiptDecisionEvent | null;
  parent_digests: Digest[];
  checks: {
    record: Check;
    lineage: Check;
    review: Check;
    impact: Check;
  };
  proofs: Array<{
    proof_id: string;
    obligation_ids: string[];
    attestation_digest: Digest | null;
    retained_output_digest: Digest | null;
    audit_report_digest: Digest | null;
    audit_findings: Array<{ obligation_id: string; kind: string }>;
    outcome: Outcome;
    reasons: Reason[];
  }>;
  unknowns: Array<{
    id: string;
    disposition: "open" | "accepted" | "deferred" | "resolved";
  }>;
  outcome: Outcome;
  reasons: Reason[];
}
