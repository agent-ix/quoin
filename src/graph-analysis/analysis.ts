/**
 * Read-only analyses over the authored trace graph and evidence bindings
 * (FR-045).
 *
 * Direction matters. Documents author edges as source -> target, but the
 * relationship verbs accepted here all mean that the source depends on or
 * elaborates the target. The normalized graph is therefore target -> source:
 * a change at the target can flow downstream to the source. Verbs whose
 * direction is not safe to infer are reported as limitations rather than
 * guessed.
 */

import type { BundleDocument } from "../completeness/index.js";
import type { Binding } from "../evidence/index.js";
import type { ImplementsRecord, Obligation } from "../quire/index.js";
import { requirementOf } from "../assurance/index.js";

/** A change to the authored target can affect the source artifact. */
const TARGET_TO_SOURCE = new Set([
  "covers",
  "depends_on",
  "derives_from",
  "extends",
  "implements",
  "mitigates",
  "refines",
  "requires",
  "satisfies",
  "traces_to",
  "verifies",
]);

/** A change to the authored source can affect the target artifact. */
const SOURCE_TO_TARGET = new Set(["constrains", "satisfied_by", "specifies"]);

export type GraphLimitationKind =
  | "duplicate-document-id"
  | "orphan-binding"
  | "orphan-implementation"
  | "orphan-obligation"
  | "unreadable-document"
  | "unresolved-relationship"
  | "unsupported-relationship";

export interface GraphLimitation {
  kind: GraphLimitationKind;
  source: string;
  target?: string;
  relationship?: string;
  reason: string;
}

export interface DocumentEdge {
  /** The prerequisite, parent, or requirement being verified. */
  from: string;
  /** The dependent, derived artifact, implementation, or verification. */
  to: string;
  relationship: string;
}

export interface ObligationNode {
  id: string;
  owner: string;
}

export interface ObligationSuiteEdge {
  obligation: string;
  suite: string;
}

export interface ImplementationNode {
  /** Stable change-impact input id: `<path>#<symbol>`. */
  id: string;
  path: string;
  symbol: string;
  forms: string[];
  requirements: string[];
}

export interface TraceGraph {
  documents: string[];
  documentEdges: DocumentEdge[];
  obligations: ObligationNode[];
  obligationSuites: ObligationSuiteEdge[];
  implementations: ImplementationNode[];
  limitations: GraphLimitation[];
  complete: boolean;
}

export interface TraceGraphInput {
  documents: BundleDocument[];
  obligations: Obligation[];
  bindings: Binding[];
  implementations?: ImplementsRecord[];
  unreadable?: Array<{ path: string; reason: string }>;
}

export interface FanOutRow {
  suite: string;
  obligations: string[];
  obligationCount: number;
}

export interface FanOutAnalysis {
  view: "fan-out";
  rows: FanOutRow[];
  complete: boolean;
  limitations: GraphLimitation[];
}

export interface ChangeImpactAnalysis {
  view: "change-impact";
  changed: string[];
  unknown: string[];
  /** Authored artifacts that depend on a changed document. */
  downstreamDocuments: string[];
  /** Claims and prerequisites on which an affected document depends. */
  upstreamDocuments: string[];
  /** Obligations whose owning document or evidence-producing suite changed. */
  suspectObligations: string[];
  /** Suites bound to a suspect obligation, or named directly as changed. */
  affectedSuites: string[];
  /** Production symbols in the affected requirements' implementation scope. */
  affectedImplementations: ImplementationNode[];
  /** Other obligations sharing an affected suite; exposed, not called suspect. */
  sharedSuiteExposure: string[];
  complete: boolean;
  limitations: GraphLimitation[];
}

export interface ChurnEvent {
  who: string;
  commit: string;
  note?: string;
}

export interface ChurnRow {
  obligation: string;
  affirmationCount: number;
  events: ChurnEvent[];
}

export interface ChurnAnalysis {
  view: "churn";
  rows: ChurnRow[];
  complete: boolean;
  limitations: GraphLimitation[];
}

/** Build the common graph once; every analysis below is a projection of it. */
export function buildTraceGraph(input: TraceGraphInput): TraceGraph {
  const limitations: GraphLimitation[] = (input.unreadable ?? []).map(
    ({ path, reason }) => ({
      kind: "unreadable-document",
      source: path,
      reason,
    }),
  );
  const documents = new Set<string>();
  const identity = bundleIdentity(input.documents);

  for (const document of input.documents) {
    const id = documentId(document);
    if (!id) continue;
    if (documents.has(id)) {
      limitations.push({
        kind: "duplicate-document-id",
        source: document.path,
        target: id,
        reason: `more than one document declares id ${id}`,
      });
    }
    documents.add(id);
  }

  const documentEdges: DocumentEdge[] = [];
  for (const document of input.documents) {
    const source = documentId(document);
    if (!source) continue;
    for (const relationship of relationshipsOf(document)) {
      const target = localTarget(relationship.target, identity);
      const targetToSource = TARGET_TO_SOURCE.has(relationship.type);
      const sourceToTarget = SOURCE_TO_TARGET.has(relationship.type);
      if (!targetToSource && !sourceToTarget) {
        limitations.push({
          kind: "unsupported-relationship",
          source,
          target: target ?? relationship.target,
          relationship: relationship.type,
          reason: `impact direction is not declared for ${relationship.type}`,
        });
        continue;
      }
      if (!target || !documents.has(target)) {
        limitations.push({
          kind: "unresolved-relationship",
          source,
          target: target ?? relationship.target,
          relationship: relationship.type,
          reason: `${relationship.target} is outside or absent from this bundle`,
        });
        continue;
      }
      documentEdges.push({
        from: targetToSource ? target : source,
        to: targetToSource ? source : target,
        relationship: relationship.type,
      });
    }
  }

  const obligations = input.obligations.map((obligation) => ({
    id: obligation.id,
    owner: requirementOf(obligation.id),
  }));
  const obligationIds = new Set(obligations.map(({ id }) => id));
  for (const obligation of obligations) {
    if (!documents.has(obligation.owner)) {
      limitations.push({
        kind: "orphan-obligation",
        source: obligation.id,
        target: obligation.owner,
        reason: `owning document ${obligation.owner} is outside or absent from this bundle`,
      });
    }
  }

  const obligationSuites = unique(
    input.bindings.map(({ obligation, suite }) => ({ obligation, suite })),
    (edge) => `${edge.obligation}\u0000${edge.suite}`,
  );
  for (const edge of obligationSuites) {
    if (!obligationIds.has(edge.obligation)) {
      limitations.push({
        kind: "orphan-binding",
        source: edge.suite,
        target: edge.obligation,
        reason: `${edge.obligation} is not a current obligation`,
      });
    }
  }

  const implementations = implementationNodes(input.implementations ?? []);
  for (const implementation of implementations) {
    for (const requirement of implementation.requirements) {
      if (!documents.has(requirement)) {
        limitations.push({
          kind: "orphan-implementation",
          source: implementation.id,
          target: requirement,
          reason: `${requirement} is outside or absent from this bundle`,
        });
      }
    }
  }

  const graph: TraceGraph = {
    documents: sorted(documents),
    documentEdges: unique(
      documentEdges,
      (edge) => `${edge.from}\u0000${edge.to}\u0000${edge.relationship}`,
    ).sort(edgeOrder),
    obligations: unique(obligations, (node) => node.id).sort((a, b) =>
      compare(a.id, b.id),
    ),
    obligationSuites: obligationSuites.sort(
      (a, b) =>
        compare(a.suite, b.suite) || compare(a.obligation, b.obligation),
    ),
    implementations,
    limitations: unique(limitations, limitationKey).sort(limitationOrder),
    complete: false,
  };
  graph.complete = graph.limitations.length === 0;
  return graph;
}

/** Count the distinct obligations discharged by each suite. No threshold is implied. */
export function analyzeFanOut(graph: TraceGraph): FanOutAnalysis {
  const bySuite = new Map<string, Set<string>>();
  for (const { suite, obligation } of graph.obligationSuites) {
    const obligations = bySuite.get(suite) ?? new Set<string>();
    obligations.add(obligation);
    bySuite.set(suite, obligations);
  }
  const rows = [...bySuite].map(([suite, obligations]) => ({
    suite,
    obligations: sorted(obligations),
    obligationCount: obligations.size,
  }));
  rows.sort(
    (a, b) =>
      b.obligationCount - a.obligationCount || compare(a.suite, b.suite),
  );
  return common(graph, { view: "fan-out", rows });
}

/**
 * Compute typed impact closure without walking the graph as one undirected
 * component. Upstream claims are context; downstream requirements and their
 * evidence are suspect. Shared-suite obligations are exposure, not silently
 * promoted to suspect.
 */
export function analyzeChangeImpact(
  graph: TraceGraph,
  changedIds: string[],
): ChangeImpactAnalysis {
  const changed = sorted(new Set(changedIds));
  const documentIds = new Set(graph.documents);
  const obligationIds = new Set(graph.obligations.map(({ id }) => id));
  const suiteIds = new Set(graph.obligationSuites.map(({ suite }) => suite));
  const implementationIds = new Set(graph.implementations.map(({ id }) => id));
  const implementationPaths = new Set(
    graph.implementations.map(({ path }) => path),
  );
  const downstream = adjacency(graph.documentEdges, "from", "to");
  const upstream = adjacency(graph.documentEdges, "to", "from");
  const ownerFor = new Map(
    graph.obligations.map(({ id, owner }) => [id, owner] as const),
  );
  const obligationsForOwner = group(
    graph.obligations,
    ({ owner }) => owner,
    ({ id }) => id,
  );
  const suitesForObligation = group(
    graph.obligationSuites,
    ({ obligation }) => obligation,
    ({ suite }) => suite,
  );
  const obligationsForSuite = group(
    graph.obligationSuites,
    ({ suite }) => suite,
    ({ obligation }) => obligation,
  );
  const implementationsForRequirement = new Map<string, ImplementationNode[]>();
  for (const implementation of graph.implementations) {
    for (const requirement of implementation.requirements) {
      const nodes = implementationsForRequirement.get(requirement) ?? [];
      nodes.push(implementation);
      implementationsForRequirement.set(requirement, nodes);
    }
  }

  const unknown = new Set<string>();
  const affectedDocuments = new Set<string>();
  const suspectObligations = new Set<string>();
  const obligationsRequiringAllSuites = new Set<string>();
  const affectedSuites = new Set<string>();
  const affectedImplementationIds = new Set<string>();
  const codeChangedRequirements = new Set<string>();

  for (const id of changed) {
    if (documentIds.has(id)) {
      affectedDocuments.add(id);
      for (const dependent of closure([id], downstream))
        affectedDocuments.add(dependent);
    } else if (obligationIds.has(id)) {
      suspectObligations.add(id);
      obligationsRequiringAllSuites.add(id);
    } else if (suiteIds.has(id)) {
      affectedSuites.add(id);
      for (const obligation of obligationsForSuite.get(id) ?? [])
        suspectObligations.add(obligation);
    } else if (implementationIds.has(id) || implementationPaths.has(id)) {
      for (const implementation of graph.implementations) {
        if (implementation.id !== id && implementation.path !== id) continue;
        affectedImplementationIds.add(implementation.id);
        for (const requirement of implementation.requirements)
          codeChangedRequirements.add(requirement);
      }
    } else {
      unknown.add(id);
    }
  }

  for (const document of affectedDocuments) {
    for (const obligation of obligationsForOwner.get(document) ?? []) {
      suspectObligations.add(obligation);
      obligationsRequiringAllSuites.add(obligation);
    }
    for (const implementation of implementationsForRequirement.get(document) ??
      [])
      affectedImplementationIds.add(implementation.id);
  }
  for (const obligation of obligationsRequiringAllSuites) {
    const owner = ownerFor.get(obligation);
    if (!owner) continue;
    for (const implementation of implementationsForRequirement.get(owner) ?? [])
      affectedImplementationIds.add(implementation.id);
  }
  for (const requirement of codeChangedRequirements) {
    for (const obligation of obligationsForOwner.get(requirement) ?? []) {
      suspectObligations.add(obligation);
      obligationsRequiringAllSuites.add(obligation);
    }
  }
  for (const obligation of obligationsRequiringAllSuites) {
    for (const suite of suitesForObligation.get(obligation) ?? [])
      affectedSuites.add(suite);
  }

  const upstreamDocuments = new Set<string>();
  for (const document of affectedDocuments) {
    for (const prerequisite of closure([document], upstream)) {
      if (!affectedDocuments.has(prerequisite))
        upstreamDocuments.add(prerequisite);
    }
  }
  for (const obligation of suspectObligations) {
    const owner = ownerFor.get(obligation);
    if (!owner) continue;
    for (const prerequisite of closure([owner], upstream)) {
      if (!affectedDocuments.has(prerequisite))
        upstreamDocuments.add(prerequisite);
    }
  }

  const sharedSuiteExposure = new Set<string>();
  for (const suite of affectedSuites) {
    for (const obligation of obligationsForSuite.get(suite) ?? []) {
      if (!suspectObligations.has(obligation))
        sharedSuiteExposure.add(obligation);
    }
  }

  return {
    ...common(graph, {
      view: "change-impact" as const,
      changed,
      unknown: sorted(unknown),
      downstreamDocuments: sorted(affectedDocuments),
      upstreamDocuments: sorted(upstreamDocuments),
      suspectObligations: sorted(suspectObligations),
      affectedSuites: sorted(affectedSuites),
      affectedImplementations: graph.implementations.filter(({ id }) =>
        affectedImplementationIds.has(id),
      ),
      sharedSuiteExposure: sorted(sharedSuiteExposure),
    }),
    // Graph completeness and query resolution are separate failure modes. A
    // perfectly readable graph still cannot answer for an id it does not know.
    complete: graph.complete && unknown.size === 0,
  };
}

/**
 * Rank obligation-level re-affirmation events. `affirm()` copies one event to
 * every selected suite binding, so identical events are deliberately
 * deduplicated here instead of counting the number of suites.
 */
export function analyzeChurn(
  graph: TraceGraph,
  bindings: Binding[],
): ChurnAnalysis {
  const events = new Map<string, Map<string, ChurnEvent>>();
  for (const binding of bindings) {
    for (const affirmation of binding.affirmations ?? []) {
      const event: ChurnEvent = {
        who: affirmation.who,
        commit: affirmation.commit,
        ...(affirmation.note === undefined ? {} : { note: affirmation.note }),
      };
      const byIdentity = events.get(binding.obligation) ?? new Map();
      byIdentity.set(eventKey(event), event);
      events.set(binding.obligation, byIdentity);
    }
  }
  const rows = [...events].map(([obligation, byIdentity]) => ({
    obligation,
    affirmationCount: byIdentity.size,
    events: [...byIdentity.values()].sort(eventOrder),
  }));
  rows.sort(
    (a, b) =>
      b.affirmationCount - a.affirmationCount ||
      compare(a.obligation, b.obligation),
  );
  return common(graph, { view: "churn", rows });
}

function implementationNodes(
  records: ImplementsRecord[],
): ImplementationNode[] {
  const nodes = new Map<string, ImplementationNode>();
  for (const record of records) {
    const id = `${record.path}#${record.symbol}`;
    const prior = nodes.get(id) ?? {
      id,
      path: record.path,
      symbol: record.symbol,
      forms: [],
      requirements: [],
    };
    prior.forms = sorted(new Set([...prior.forms, record.form]));
    prior.requirements = sorted(
      new Set([...prior.requirements, requirementOf(record.trace_id)]),
    );
    nodes.set(id, prior);
  }
  return [...nodes.values()].sort((a, b) => compare(a.id, b.id));
}

function documentId(document: BundleDocument): string | null {
  const id = document.frontmatter.id;
  return typeof id === "string" && id.length > 0 ? id : null;
}

function relationshipsOf(
  document: BundleDocument,
): Array<{ type: string; target: string }> {
  const relationships = document.frontmatter.relationships;
  if (!Array.isArray(relationships)) return [];
  const out: Array<{ type: string; target: string }> = [];
  for (const value of relationships as Array<Record<string, unknown>>) {
    if (!value || typeof value !== "object") continue;
    const type = typeof value.type === "string" ? value.type : "";
    const rawTarget = typeof value.target === "string" ? value.target : "";
    if (type && rawTarget) out.push({ type, target: rawTarget });
  }
  return out;
}

interface BundleIdentity {
  org: string;
  component: string;
}

/** The master-requirements document declares the local ix URI authority. */
function bundleIdentity(documents: BundleDocument[]): BundleIdentity | null {
  for (const document of documents) {
    const org = document.frontmatter.org;
    const component =
      document.frontmatter.component ?? document.frontmatter.name;
    if (
      typeof org === "string" &&
      org.length > 0 &&
      typeof component === "string" &&
      component.length > 0
    ) {
      return { org, component };
    }
  }
  return null;
}

/**
 * Resolve a bare id or a full local ix URI. External URIs stay unresolved;
 * taking only their last segment would alias `other/FR-001` to local FR-001.
 */
function localTarget(
  target: string,
  identity: BundleIdentity | null,
): string | null {
  if (!target.startsWith("ix://")) return target;
  const match = /^ix:\/\/([^/]+)\/([^/]+)\/([^/]+)$/.exec(target);
  if (!match || !identity) return null;
  return match[1] === identity.org && match[2] === identity.component
    ? match[3]
    : null;
}

function adjacency(
  edges: DocumentEdge[],
  key: "from" | "to",
  value: "from" | "to",
): Map<string, string[]> {
  const result = new Map<string, Set<string>>();
  for (const edge of edges) {
    const values = result.get(edge[key]) ?? new Set<string>();
    values.add(edge[value]);
    result.set(edge[key], values);
  }
  return new Map(
    [...result].map(([id, values]) => [id, sorted(values)] as const),
  );
}

function closure(
  seeds: Iterable<string>,
  edges: Map<string, string[]>,
): Set<string> {
  const seen = new Set<string>();
  const queue = [...seeds];
  while (queue.length > 0) {
    const current = queue.shift();
    if (!current) continue;
    for (const next of edges.get(current) ?? []) {
      if (seen.has(next)) continue;
      seen.add(next);
      queue.push(next);
    }
  }
  return seen;
}

function group<T>(
  values: T[],
  key: (value: T) => string,
  item: (value: T) => string,
): Map<string, string[]> {
  const result = new Map<string, Set<string>>();
  for (const value of values) {
    const items = result.get(key(value)) ?? new Set<string>();
    items.add(item(value));
    result.set(key(value), items);
  }
  return new Map(
    [...result].map(([id, items]) => [id, sorted(items)] as const),
  );
}

function common<T extends object>(
  graph: TraceGraph,
  analysis: T,
): T & { complete: boolean; limitations: GraphLimitation[] } {
  return {
    ...analysis,
    complete: graph.complete,
    limitations: graph.limitations,
  };
}

function unique<T>(values: T[], key: (value: T) => string): T[] {
  return [...new Map(values.map((value) => [key(value), value])).values()];
}

function sorted(values: Iterable<string>): string[] {
  return [...values].sort(compare);
}

function compare(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function edgeOrder(a: DocumentEdge, b: DocumentEdge): number {
  return (
    compare(a.from, b.from) ||
    compare(a.to, b.to) ||
    compare(a.relationship, b.relationship)
  );
}

function limitationKey(limitation: GraphLimitation): string {
  return [
    limitation.kind,
    limitation.source,
    limitation.target ?? "",
    limitation.relationship ?? "",
    limitation.reason,
  ].join("\u0000");
}

function limitationOrder(a: GraphLimitation, b: GraphLimitation): number {
  return compare(limitationKey(a), limitationKey(b));
}

function eventKey(event: ChurnEvent): string {
  return `${event.commit}\u0000${event.who}\u0000${event.note ?? ""}`;
}

function eventOrder(a: ChurnEvent, b: ChurnEvent): number {
  return compare(eventKey(a), eventKey(b));
}
