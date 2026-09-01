/**
 * Typed consumer model for quire-rs's assurance-v1 export (FR-062).
 *
 * The JSON Schema is authoritative. These types describe the value only after
 * schema and caller-premise validation have both succeeded.
 */

export interface AssuranceSource {
  repository: string;
  revision: string;
}

export interface AssuranceSchemaPremise {
  archetype: string;
  schema_digest: string;
}

export interface AssuranceModulePremise {
  name: string;
  version: string;
  schemas: AssuranceSchemaPremise[];
}

export interface AcceptedAssurancePremises {
  format: "quire-assurance";
  format_version: 1;
  modules: AssuranceModulePremise[];
}

export interface AssuranceLocator {
  path: string;
  line: number;
  digest: string;
}

export interface AssuranceArtifact {
  id: string;
  uuid?: string;
  artifact_type: string;
  locator: AssuranceLocator;
}

export interface AssuranceObligation {
  source: string;
  id: string;
  document: string;
  statement: string;
  statement_hash: string;
  method?: string;
  parameters?: Record<string, string>;
  criticality?: string;
  target_ids: string[];
  locator: AssuranceLocator;
}

export interface AssuranceSymbol {
  id: string;
  language: "rust" | "python" | "typescript";
  kind:
    "function" | "test_function" | "container" | "benchmark" | "fuzz_target";
  qualified_name: string;
  container: string | null;
  capabilities: Array<"verifies" | "implements">;
  locator: AssuranceLocator;
}

export interface AssuranceRelationKind {
  kind: string;
  availability: "available";
  sources: Array<
    "module_vocabulary" | "required_relation" | "trace_binding" | "observed"
  >;
}

export type AssuranceFreshness =
  "current" | "suspect" | "unknown" | "not_applicable";

export interface AssuranceCorpusRelation {
  kind: "corpus";
  source: string;
  target: string;
  edge_type: string;
  resolution: "resolved" | "dangling";
  locator: AssuranceLocator;
  freshness: AssuranceFreshness;
}

export interface AssuranceVerifiesRelation {
  kind: "verifies";
  source: string;
  target: string;
  form: string;
  provenance: "canonical" | "legacy";
  locator: AssuranceLocator;
  freshness: AssuranceFreshness;
}

export interface AssuranceImplementsRelation {
  kind: "implements";
  source: string;
  target: string;
  form: string;
  locator: AssuranceLocator;
  freshness: AssuranceFreshness;
}

export type AssuranceRelation =
  | AssuranceCorpusRelation
  | AssuranceVerifiesRelation
  | AssuranceImplementsRelation;

export interface AssuranceRelationObservation {
  declaration: string;
  subject?: string;
  availability: "available" | "missing" | "not_applicable" | "unknown";
  freshness: AssuranceFreshness;
  reason?: string;
}

export interface AssuranceExport {
  format: "quire-assurance";
  format_version: 1;
  source: AssuranceSource;
  modules: AssuranceModulePremise[];
  artifacts: AssuranceArtifact[];
  obligations: AssuranceObligation[];
  symbols: AssuranceSymbol[];
  relation_kinds: AssuranceRelationKind[];
  relations: AssuranceRelation[];
  relation_observations: AssuranceRelationObservation[];
}
