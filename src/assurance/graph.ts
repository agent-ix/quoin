/**
 * The assurance case as a graph (FR-040).
 *
 * A **claim** is argued by explicit reasoning over sub-claims, and the leaves
 * are **evidence**. A pile of green obligations is not an argument; the
 * argument is the authored reasoning and its visible gaps.
 *
 * **This is strictly a view.** It collects nothing, computes no coverage and
 * writes nothing to the store. Where the case cannot be built, that is a finding
 * against the spec or the store — never something the view fills in.
 */

import type { Obligation } from "../quire/index.js";
import type { Finding } from "../auditor/index.js";
import type { BundleDocument } from "../completeness/index.js";

/** Internal node kinds for the legacy derived view; not an external format. */
export type NodeKind = "goal" | "strategy" | "solution";

/**
 * Whether a node's part of the argument holds.
 *
 * `open` is the point of the whole view. An undischarged obligation or a suspect
 * binding stays in the tree as an **undeveloped goal** rather than being
 * dropped: a case that silently narrows to only what it can prove reads as
 * complete, and that is the failure this program exists to catch.
 */
export type NodeStatus = "supported" | "open";

export interface CaseNode {
  id: string;
  kind: NodeKind;
  /** What the node asserts, in the spec's own words where it has them. */
  statement: string;
  status: NodeStatus;
  /** Why it is open — the auditor's finding, verbatim. Absent when supported. */
  because?: string;
  children: CaseNode[];
}

export interface AssuranceCase {
  /** Top-level claims, one tree each. */
  claims: CaseNode[];
  /**
   * Present exactly when `claims` is empty: why there is no case, naming the
   * claim types that were searched. The human renderer already explained this
   * (`render.ts`); the JSON payload carried no equivalent, so a machine
   * consumer could not tell "the assurance case is clean" from "nothing
   * matched, so nothing was argued" (#170).
   */
  reason?: string;
  /**
   * Requirements reachable from no claim.
   *
   * Not an error and not hidden: a bundle whose StRs do not reach its FRs has a
   * traceability gap, and the view's job is to make that visible rather than to
   * render a tidy case over the reachable half.
   */
  unreachable: string[];
  /** Documents whose frontmatter could not be read. */
  unreadable: Array<{ path: string; reason: string }>;
}

/** The edge verbs that mean "this document elaborates that one". */
const DOWNWARD_FROM_CHILD = new Set([
  "traces_to",
  "refines",
  "implements",
  "derives_from",
  "mitigates",
  "satisfies",
]);

export interface CaseInput {
  /** Frontmatter of every document in the bundle. */
  documents: BundleDocument[];
  /** Obligations, from `quire coverage --json`. */
  obligations: Obligation[];
  /** The auditor's findings, keyed by obligation elsewhere. */
  findings: Finding[];
  /**
   * Artifact types that are top-level claims.
   *
   * Defaults to `StR`. A safety or security bundle argues from a declared
   * hazard or threat instead, and that vocabulary is module data — so it is a
   * parameter here, never a constant.
   */
  claimTypes?: string[];
  unreadable?: Array<{ path: string; reason: string }>;
}

/**
 * Build the case.
 *
 * The decomposition is read from the documents' own `relationships`, in the
 * child→parent direction the corpus actually authors: an FR says it
 * `traces_to` an StR, not the other way round.
 */
export function buildCase(input: CaseInput): AssuranceCase {
  // Case-insensitive: `--claim-type str`, `STR` or `Hazard` used to match by
  // `===` against the authored `type:` and silently produce an empty case —
  // indistinguishable from a corpus that genuinely declares no claims (#170).
  const requested = input.claimTypes ?? ["StR"];
  const claimTypes = new Set(requested.map((t) => t.toLowerCase()));
  const byId = new Map<string, BundleDocument>();
  for (const doc of input.documents) {
    const id = idOf(doc);
    if (id) byId.set(id, doc);
  }

  // parent id → child ids, inverted from the edges each child declares.
  const children = new Map<string, string[]>();
  for (const doc of input.documents) {
    const id = idOf(doc);
    if (!id) continue;
    for (const parent of parentsOf(doc)) {
      if (!byId.has(parent)) continue;
      children.set(parent, [...(children.get(parent) ?? []), id]);
    }
  }

  const findingFor = new Map<string, Finding>();
  for (const finding of input.findings) {
    // First wins: the auditor already orders by severity, so the first finding
    // for an obligation is its most serious one, and that is the one a reader
    // needs on the node.
    if (!findingFor.has(finding.obligation)) {
      findingFor.set(finding.obligation, finding);
    }
  }

  const obligationsFor = new Map<string, Obligation[]>();
  for (const obligation of input.obligations) {
    const owner = requirementOf(obligation.id);
    obligationsFor.set(owner, [
      ...(obligationsFor.get(owner) ?? []),
      obligation,
    ]);
  }

  const claims: CaseNode[] = [];
  const reached = new Set<string>();
  for (const [id, doc] of byId) {
    if (!claimTypes.has(String(doc.frontmatter.type ?? "").toLowerCase()))
      continue;
    // A fresh `ancestors` set per claim. Sharing one across claims made a
    // requirement that refines TWO claims appear under only the first, while
    // the second reported "no sub-claim traces to this claim" — a statement
    // that was false, in an assurance case, about the very edge the author had
    // written. Cycle prevention and "already rendered somewhere" are different
    // questions and now have different sets.
    claims.push(
      nodeFor(
        id,
        byId,
        children,
        obligationsFor,
        findingFor,
        reached,
        new Set(),
      ),
    );
  }
  claims.sort((a, b) => a.id.localeCompare(b.id));

  const unreachable = [...byId.keys()]
    .filter((id) => !reached.has(id) && obligationsFor.has(id))
    .sort();

  const assurance: AssuranceCase = {
    claims,
    unreachable,
    unreadable: input.unreadable ?? [],
  };
  if (claims.length === 0) {
    // The human renderer's explanation, made machine-readable. Naming the
    // SEARCHED types (as the caller spelled them) is what lets a pipeline see
    // that `--claim-type hazard` over a hazard-free corpus argued nothing.
    assurance.reason =
      `no document declares itself a top-level claim ` +
      `(searched claim types: ${requested.join(", ")}); ` +
      `declare one or pass --claim-type`;
  }
  return assurance;
}

function nodeFor(
  id: string,
  byId: Map<string, BundleDocument>,
  children: Map<string, string[]>,
  obligationsFor: Map<string, Obligation[]>,
  findingFor: Map<string, Finding>,
  reached: Set<string>,
  ancestors: Set<string>,
): CaseNode {
  reached.add(id);
  ancestors.add(id);
  const doc = byId.get(id);
  const title = String(doc?.frontmatter.title ?? id);

  // `ancestors`, not `reached`: a child already rendered under a DIFFERENT
  // claim must still appear here, and only a child on this path would be a
  // cycle.
  const sub = (children.get(id) ?? [])
    .filter((child) => !ancestors.has(child))
    .sort()
    .map((child) =>
      nodeFor(
        child,
        byId,
        children,
        obligationsFor,
        findingFor,
        reached,
        new Set(ancestors),
      ),
    );

  const evidence = (obligationsFor.get(id) ?? [])
    .slice()
    .sort((a, b) => a.id.localeCompare(b.id))
    .map((obligation) => solutionFor(obligation, findingFor));

  const parts = [...sub, ...evidence];
  const node: CaseNode = {
    id,
    kind: "goal",
    statement: title,
    // A goal with no sub-goals and no evidence is open, which a reader must
    // see. Rendering it as supported would assure a claim on the strength of
    // nobody having written anything.
    status:
      parts.length === 0 || parts.some((p) => p.status === "open")
        ? "open"
        : "supported",
    children: parts,
  };
  if (parts.length === 0) {
    node.because = "no sub-claim and no obligation traces to this claim";
  }
  return node;
}

function solutionFor(
  obligation: Obligation,
  findingFor: Map<string, Finding>,
): CaseNode {
  const finding = findingFor.get(obligation.id);
  return {
    id: obligation.id,
    kind: "solution",
    statement: obligation.statement,
    status: finding ? "open" : "supported",
    ...(finding ? { because: `${finding.kind}: ${finding.summary}` } : {}),
    children: [],
  };
}

/** The document's own id, from frontmatter. */
function idOf(doc: BundleDocument): string | null {
  const id = doc.frontmatter.id;
  return typeof id === "string" && id.length > 0 ? id : null;
}

/** Ids this document declares itself to elaborate. */
function parentsOf(doc: BundleDocument): string[] {
  const rels = doc.frontmatter.relationships;
  if (!Array.isArray(rels)) return [];
  const out: string[] = [];
  for (const entry of rels as Array<Record<string, unknown>>) {
    if (!entry || typeof entry !== "object") continue;
    if (!DOWNWARD_FROM_CHILD.has(String(entry.type ?? ""))) continue;
    const target = String(entry.target ?? "");
    // `ix://org/component/ID` or a bare id — take the last segment either way.
    const id = target.split("/").pop();
    if (id) out.push(id);
  }
  return out;
}

/**
 * The requirement an obligation belongs to.
 *
 * `FR-001-AC-3` → `FR-001`; `NFR-010-M-2` → `NFR-010`. Derived from the id
 * rather than from an edge because obligations are minted from a document's
 * own tables — the ownership is not in question and an edge for it would be a
 * second thing to keep in agreement.
 */
export function requirementOf(obligationId: string): string {
  const match = /^([A-Za-z]+-\d+)/.exec(obligationId);
  return match ? match[1] : obligationId;
}
