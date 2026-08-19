/**
 * The assurance-case view (FR-040).
 *
 * ADR-0011 L1: read-only over the evidence store and the spec. It collects
 * nothing and writes nothing.
 */

export {
  buildCase,
  requirementOf,
  type AssuranceCase,
  type CaseInput,
  type CaseNode,
  type NodeKind,
  type NodeStatus,
} from "./graph.js";
export { renderCase } from "./render.js";
