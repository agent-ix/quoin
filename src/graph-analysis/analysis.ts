/** Deterministic, read-only assurance graph projections (FR-062). */

import type {
  AuditReport,
  Finding,
  UnevaluatedCheck,
} from "../auditor/index.js";
import type { Binding } from "../evidence/index.js";
import type {
  AcceptedAssurancePremises,
  AssuranceCorpusRelation,
  AssuranceExport,
  AssuranceObligation,
  AssuranceSource,
} from "../quire/index.js";
import type { AuditEnvelope } from "./input.js";

export const DEFAULT_RELATION_KINDS = [
  "depends_on",
  "derives_from",
  "implements",
  "mitigates",
  "refines",
  "requires",
  "satisfies",
  "traces_to",
] as const;

export type GraphAnalysisState = "complete" | "incomplete" | "not_computed";
export type GraphGapKind =
  | "absent-bindings-store"
  | "dangling-relation"
  | "empty-bindings-store"
  | "missing-auditor-verdict"
  | "missing-required-relation"
  | "unknown-relation-availability"
  | "unknown-relation-kind"
  | "unknown-requirement"
  | "unreadable-bindings-store"
  | "unresolved-binding"
  | "unresolved-obligation-owner";

export interface GraphGap {
  kind: GraphGapKind;
  subject: string;
  reason: string;
}

export type BindingInput =
  | { availability: "available"; bindings: Binding[] }
  | { availability: "absent"; reason: string }
  | { availability: "unreadable"; reason: string };

export interface GraphAnalysisInput {
  assurance: AssuranceExport;
  premises: AcceptedAssurancePremises;
  audit: AuditEnvelope;
  bindings: BindingInput;
}

export interface GraphReportBase {
  view: "fan-out" | "change-impact" | "churn";
  source: AssuranceSource;
  export: { format: "quire-assurance"; format_version: 1 };
  premises: AcceptedAssurancePremises;
  state: GraphAnalysisState;
  gaps: GraphGap[];
}

export interface OwnedObligation {
  obligation: string;
  requirements: string[];
}

export interface FanOutRow {
  suite: string;
  obligations: OwnedObligation[];
  obligationCount: number;
  unresolvedBindings: string[];
}

export interface FanOutAnalysis extends GraphReportBase {
  view: "fan-out";
  rows: FanOutRow[];
}

export interface ChurnEvent {
  who: string;
  commit: string;
  note?: string;
  suites: string[];
}

export interface ChurnRow extends OwnedObligation {
  suites: string[];
  events: ChurnEvent[];
  eventCount: number;
}

export interface ChurnAnalysis extends GraphReportBase {
  view: "churn";
  rows: ChurnRow[];
}

export interface ImpactPathEdge {
  source: string;
  target: string;
  relationship: string;
}

export interface ImpactPath {
  seed: string;
  edges: ImpactPathEdge[];
}

export interface AuditorVerdict {
  findings: Finding[];
  healthy: string[];
  unevaluated: UnevaluatedCheck[];
}

export interface ImpactBinding {
  suite: string;
  auditorVerdict: AuditorVerdict;
}

export interface ImpactObligation extends OwnedObligation {
  bindings: ImpactBinding[];
}

export interface ChangeImpactRow {
  requirement: string;
  depth: number;
  path: ImpactPath;
  obligations: ImpactObligation[];
}

export interface ChangeImpactAnalysis extends GraphReportBase {
  view: "change-impact";
  requested: string[];
  relationKinds: string[];
  rows: ChangeImpactRow[];
}

export type GraphAnalysis =
  FanOutAnalysis | ChangeImpactAnalysis | ChurnAnalysis;

export function analyzeFanOut(input: GraphAnalysisInput): FanOutAnalysis {
  const report = base(input, "fan-out");
  if (input.bindings.availability !== "available") {
    return { ...report, state: "not_computed", rows: [] };
  }
  const owners = obligationOwners(input.assurance);
  const live = new Set(input.assurance.obligations.map(({ id }) => id));
  const rows = new Map<
    string,
    { live: Set<string>; unresolved: Set<string> }
  >();
  for (const binding of input.bindings.bindings) {
    const row = rows.get(binding.suite) ?? {
      live: new Set(),
      unresolved: new Set(),
    };
    if (live.has(binding.obligation)) row.live.add(binding.obligation);
    else {
      row.unresolved.add(binding.obligation);
      report.gaps.push({
        kind: "unresolved-binding",
        subject: `${binding.suite}:${binding.obligation}`,
        reason: `${binding.obligation} is absent from the accepted assurance export`,
      });
    }
    rows.set(binding.suite, row);
  }
  return finish({
    ...report,
    rows: [...rows.entries()]
      .sort(([left], [right]) => compare(left, right))
      .map(([suite, row]) => ({
        suite,
        obligations: sorted(row.live).map((obligation) => ({
          obligation,
          requirements: owners.get(obligation) ?? [],
        })),
        obligationCount: row.live.size,
        unresolvedBindings: sorted(row.unresolved),
      })),
  });
}

export function analyzeChurn(input: GraphAnalysisInput): ChurnAnalysis {
  const report = base(input, "churn");
  if (input.bindings.availability !== "available") {
    return { ...report, state: "not_computed", rows: [] };
  }
  const owners = obligationOwners(input.assurance);
  const live = new Set(input.assurance.obligations.map(({ id }) => id));
  const suites = new Map<string, Set<string>>();
  const events = new Map<string, Map<string, ChurnEvent>>();
  for (const binding of input.bindings.bindings) {
    if (!live.has(binding.obligation)) {
      if ((binding.affirmations?.length ?? 0) > 0) {
        report.gaps.push({
          kind: "unresolved-binding",
          subject: `${binding.suite}:${binding.obligation}`,
          reason: `affirmation history belongs to absent obligation ${binding.obligation}`,
        });
      }
      continue;
    }
    const boundSuites = suites.get(binding.obligation) ?? new Set<string>();
    boundSuites.add(binding.suite);
    suites.set(binding.obligation, boundSuites);
    const byKey =
      events.get(binding.obligation) ?? new Map<string, ChurnEvent>();
    for (const affirmation of binding.affirmations ?? []) {
      const key = [
        binding.obligation,
        affirmation.who,
        affirmation.commit,
        affirmation.note ?? "",
      ].join("\u0000");
      const existing = byKey.get(key);
      if (existing)
        existing.suites = sorted(new Set([...existing.suites, binding.suite]));
      else {
        byKey.set(key, {
          who: affirmation.who,
          commit: affirmation.commit,
          ...(affirmation.note === undefined ? {} : { note: affirmation.note }),
          suites: [binding.suite],
        });
      }
    }
    events.set(binding.obligation, byKey);
  }
  const rows = input.assurance.obligations.map(({ id }) => {
    const rowEvents = [...(events.get(id)?.values() ?? [])].sort(eventOrder);
    return {
      obligation: id,
      requirements: owners.get(id) ?? [],
      suites: sorted(suites.get(id) ?? new Set()),
      events: rowEvents,
      eventCount: rowEvents.length,
    };
  });
  rows.sort(
    (left, right) =>
      right.eventCount - left.eventCount ||
      compare(left.obligation, right.obligation),
  );
  return finish({ ...report, rows });
}

export function analyzeChangeImpact(
  input: GraphAnalysisInput,
  requested: string[],
  selectedRelations?: string[],
): ChangeImpactAnalysis {
  const report = base(input, "change-impact");
  const relationKinds = sorted(
    new Set(selectedRelations ?? DEFAULT_RELATION_KINDS),
  );
  const initial: ChangeImpactAnalysis = {
    ...report,
    requested: sorted(new Set(requested)),
    relationKinds,
    rows: [],
  };
  if (input.bindings.availability !== "available")
    return { ...initial, state: "not_computed" };

  const availableKinds = new Set(
    input.assurance.relation_kinds.map(({ kind }) => kind),
  );
  for (const kind of relationKinds.filter(
    (candidate) => !availableKinds.has(candidate),
  )) {
    initial.gaps.push({
      kind: "unknown-relation-kind",
      subject: kind,
      reason: `relationship kind ${kind} is absent from the accepted export vocabulary`,
    });
  }
  if (initial.gaps.some(({ kind }) => kind === "unknown-relation-kind")) {
    return {
      ...initial,
      state: "not_computed",
      gaps: uniqueGaps(initial.gaps),
    };
  }

  const artifactIds = new Set(input.assurance.artifacts.map(({ id }) => id));
  const validSeeds: string[] = [];
  for (const seed of initial.requested) {
    if (artifactIds.has(seed)) validSeeds.push(seed);
    else
      initial.gaps.push({
        kind: "unknown-requirement",
        subject: seed,
        reason: `${seed} is absent from the accepted assurance export`,
      });
  }
  const edges = selectedCorpusEdges(
    input.assurance,
    new Set(relationKinds),
    artifactIds,
    initial.gaps,
  );
  const paths = shortestReversePaths(validSeeds, edges);
  const owners = obligationOwners(input.assurance);
  const obligationsByOwner = new Map<string, AssuranceObligation[]>();
  for (const obligation of input.assurance.obligations) {
    for (const owner of owners.get(obligation.id) ?? []) {
      const group = obligationsByOwner.get(owner) ?? [];
      group.push(obligation);
      obligationsByOwner.set(owner, group);
    }
  }
  const bindingsByObligation = groupBindings(input.bindings.bindings);
  const rows = [...paths.entries()]
    .sort(([left], [right]) => compare(left, right))
    .map(([requirement, path]) => ({
      requirement,
      depth: path.edges.length,
      path,
      obligations: (obligationsByOwner.get(requirement) ?? [])
        .slice()
        .sort((left, right) => compare(left.id, right.id))
        .map((obligation) => ({
          obligation: obligation.id,
          requirements: owners.get(obligation.id) ?? [],
          bindings: (bindingsByObligation.get(obligation.id) ?? []).map(
            (binding) => ({
              suite: binding.suite,
              auditorVerdict: verdictFor(
                input.audit.report,
                obligation.id,
                initial.gaps,
              ),
            }),
          ),
        })),
    }));
  return finish({ ...initial, rows });
}

function base<V extends GraphReportBase["view"]>(
  input: GraphAnalysisInput,
  view: V,
): GraphReportBase & { view: V } {
  const gaps: GraphGap[] = [];
  if (input.bindings.availability === "absent") {
    gaps.push({
      kind: "absent-bindings-store",
      subject: "bindings.json",
      reason: input.bindings.reason,
    });
  } else if (input.bindings.availability === "unreadable") {
    gaps.push({
      kind: "unreadable-bindings-store",
      subject: "bindings.json",
      reason: input.bindings.reason,
    });
  } else if (input.bindings.bindings.length === 0) {
    gaps.push({
      kind: "empty-bindings-store",
      subject: "bindings.json",
      reason:
        "the bindings store is present and valid but contains no bindings",
    });
  }
  if (input.bindings.availability === "available") {
    const live = new Set(input.assurance.obligations.map(({ id }) => id));
    for (const binding of input.bindings.bindings) {
      if (!live.has(binding.obligation)) {
        gaps.push({
          kind: "unresolved-binding",
          subject: `${binding.suite}:${binding.obligation}`,
          reason: `${binding.obligation} is absent from the accepted assurance export`,
        });
      }
    }
  }
  for (const observation of input.assurance.relation_observations) {
    if (observation.availability === "unknown") {
      gaps.push({
        kind: "unknown-relation-availability",
        subject: observation.subject ?? observation.declaration,
        reason:
          observation.reason ??
          `availability of ${observation.declaration} is unknown`,
      });
    } else if (observation.availability === "missing") {
      gaps.push({
        kind: "missing-required-relation",
        subject: observation.subject ?? observation.declaration,
        reason:
          observation.reason ?? `${observation.declaration} is not satisfied`,
      });
    }
  }
  const owners = obligationOwners(input.assurance);
  for (const obligation of input.assurance.obligations) {
    if ((owners.get(obligation.id) ?? []).length === 0) {
      gaps.push({
        kind: "unresolved-obligation-owner",
        subject: obligation.id,
        reason: `${obligation.document} does not identify an accepted artifact`,
      });
    }
  }
  return {
    view,
    source: input.assurance.source,
    export: {
      format: input.assurance.format,
      format_version: input.assurance.format_version,
    },
    premises: input.premises,
    state: gaps.length === 0 ? "complete" : "incomplete",
    gaps: uniqueGaps(gaps),
  };
}

function finish<T extends GraphReportBase>(report: T): T {
  const gaps = uniqueGaps(report.gaps);
  return {
    ...report,
    state:
      report.state === "not_computed"
        ? "not_computed"
        : gaps.length === 0
          ? "complete"
          : "incomplete",
    gaps,
  };
}

function obligationOwners(exportValue: AssuranceExport): Map<string, string[]> {
  const artifactsByPath = new Map<string, string[]>();
  for (const artifact of exportValue.artifacts) {
    const group = artifactsByPath.get(artifact.locator.path) ?? [];
    group.push(artifact.id);
    artifactsByPath.set(artifact.locator.path, group);
  }
  return new Map(
    exportValue.obligations.map((obligation) => [
      obligation.id,
      sorted(artifactsByPath.get(obligation.document) ?? []),
    ]),
  );
}

function groupBindings(bindings: Binding[]): Map<string, Binding[]> {
  const groups = new Map<string, Map<string, Binding>>();
  for (const binding of bindings) {
    const group = groups.get(binding.obligation) ?? new Map<string, Binding>();
    if (!group.has(binding.suite)) group.set(binding.suite, binding);
    groups.set(binding.obligation, group);
  }
  return new Map(
    [...groups].map(([obligation, group]) => [
      obligation,
      [...group.values()].sort((left, right) =>
        compare(left.suite, right.suite),
      ),
    ]),
  );
}

function verdictFor(
  report: AuditReport,
  obligation: string,
  gaps: GraphGap[],
): AuditorVerdict {
  const verdict = {
    findings: report.findings.filter(
      (finding) => finding.obligation === obligation,
    ),
    healthy: report.healthy.filter((id) => id === obligation),
    unevaluated: report.unevaluated.filter(
      (check) => check.obligation === obligation,
    ),
  };
  if (
    verdict.findings.length === 0 &&
    verdict.healthy.length === 0 &&
    verdict.unevaluated.length === 0
  ) {
    gaps.push({
      kind: "missing-auditor-verdict",
      subject: obligation,
      reason: `${obligation} has no FR-032 finding, healthy result, or unevaluated check`,
    });
  }
  return verdict;
}

function selectedCorpusEdges(
  exportValue: AssuranceExport,
  selection: Set<string>,
  artifactIds: Set<string>,
  gaps: GraphGap[],
): AssuranceCorpusRelation[] {
  const edges: AssuranceCorpusRelation[] = [];
  for (const relation of exportValue.relations) {
    if (relation.kind !== "corpus" || !selection.has(relation.edge_type))
      continue;
    if (
      relation.resolution !== "resolved" ||
      !artifactIds.has(relation.source) ||
      !artifactIds.has(relation.target)
    ) {
      gaps.push({
        kind: "dangling-relation",
        subject: `${relation.source}:${relation.edge_type}:${relation.target}`,
        reason: "selected relationship cannot resolve both accepted artifacts",
      });
    } else edges.push(relation);
  }
  return edges.sort(corpusEdgeOrder);
}

function shortestReversePaths(
  seeds: string[],
  edges: AssuranceCorpusRelation[],
): Map<string, ImpactPath> {
  const dependents = new Map<string, AssuranceCorpusRelation[]>();
  for (const edge of edges) {
    const group = dependents.get(edge.target) ?? [];
    group.push(edge);
    dependents.set(edge.target, group);
  }
  for (const group of dependents.values()) group.sort(corpusEdgeOrder);
  const paths = new Map<string, ImpactPath>();
  const queue = new PathQueue();
  for (const seed of seeds.sort(compare)) {
    const candidate: ImpactPath = { seed, edges: [] };
    if (betterPath(candidate, paths.get(seed))) {
      paths.set(seed, candidate);
      queue.push({ node: seed, path: candidate });
    }
  }
  while (queue.length > 0) {
    const current = queue.pop()!;
    if (!samePath(paths.get(current.node), current.path)) continue;
    for (const edge of dependents.get(current.node) ?? []) {
      const candidate: ImpactPath = {
        seed: current.path.seed,
        edges: [
          ...current.path.edges,
          {
            source: edge.source,
            target: edge.target,
            relationship: edge.edge_type,
          },
        ],
      };
      if (betterPath(candidate, paths.get(edge.source))) {
        paths.set(edge.source, candidate);
        queue.push({ node: edge.source, path: candidate });
      }
    }
  }
  return paths;
}

interface QueuedPath {
  node: string;
  path: ImpactPath;
}

/** Binary min-heap: shared/cyclic closures do not repeatedly sort the frontier. */
class PathQueue {
  private readonly items: QueuedPath[] = [];

  get length(): number {
    return this.items.length;
  }

  push(item: QueuedPath): void {
    this.items.push(item);
    let index = this.items.length - 1;
    while (index > 0) {
      const parent = Math.floor((index - 1) / 2);
      if (queueOrder(this.items[parent], item) <= 0) break;
      this.items[index] = this.items[parent];
      index = parent;
    }
    this.items[index] = item;
  }

  pop(): QueuedPath | undefined {
    const first = this.items[0];
    const last = this.items.pop();
    if (first === undefined || last === undefined || this.items.length === 0) {
      return first;
    }
    let index = 0;
    while (true) {
      const left = index * 2 + 1;
      if (left >= this.items.length) break;
      const right = left + 1;
      const child =
        right < this.items.length &&
        queueOrder(this.items[right], this.items[left]) < 0
          ? right
          : left;
      if (queueOrder(this.items[child], last) >= 0) break;
      this.items[index] = this.items[child];
      index = child;
    }
    this.items[index] = last;
    return first;
  }
}

function betterPath(candidate: ImpactPath, current?: ImpactPath): boolean {
  if (!current) return true;
  return candidate.edges.length !== current.edges.length
    ? candidate.edges.length < current.edges.length
    : compare(pathKey(candidate), pathKey(current)) < 0;
}

function samePath(left: ImpactPath | undefined, right: ImpactPath): boolean {
  return left !== undefined && pathKey(left) === pathKey(right);
}

function pathKey(path: ImpactPath): string {
  return [
    path.seed,
    ...path.edges.flatMap(({ source, target, relationship }) => [
      source,
      target,
      relationship,
    ]),
  ].join("\u0000");
}

function queueOrder(left: QueuedPath, right: QueuedPath): number {
  return (
    left.path.edges.length - right.path.edges.length ||
    compare(pathKey(left.path), pathKey(right.path)) ||
    compare(left.node, right.node)
  );
}

function corpusEdgeOrder(
  left: AssuranceCorpusRelation,
  right: AssuranceCorpusRelation,
): number {
  return (
    compare(left.source, right.source) ||
    compare(left.target, right.target) ||
    compare(left.edge_type, right.edge_type)
  );
}

function eventOrder(left: ChurnEvent, right: ChurnEvent): number {
  return (
    compare(left.who, right.who) ||
    compare(left.commit, right.commit) ||
    compare(left.note ?? "", right.note ?? "")
  );
}

function uniqueGaps(gaps: GraphGap[]): GraphGap[] {
  const byKey = new Map<string, GraphGap>();
  for (const gap of gaps)
    byKey.set(`${gap.kind}\u0000${gap.subject}\u0000${gap.reason}`, gap);
  return [...byKey.values()].sort(
    (left, right) =>
      compare(left.kind, right.kind) ||
      compare(left.subject, right.subject) ||
      compare(left.reason, right.reason),
  );
}

function sorted(values: Iterable<string>): string[] {
  return [...values].sort(compare);
}

function compare(left: string, right: string): number {
  return left === right ? 0 : left < right ? -1 : 1;
}
