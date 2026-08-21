import { specInvariants } from "../../../dist/index.js";

// Canonical "all" set — the seven analyses named in the spec-review skill.
const ALL_ANALYSES = [
  "failure-domain",
  "integrity",
  "dependency",
  "evidence",
  "risk-complexity",
  "scope-boundary",
  "ears-conformance",
];

const norm = (value) =>
  String(value ?? "")
    .trim()
    .toLowerCase()
    .replace(/^spec-/, "")
    .replace(/-analysis$/, "");

function findRequest(instance) {
  const items = instance.items?.operation_request ?? [];
  return (
    items.find((item) => item && item.interviewId === "request") ?? items[0]
  );
}

// Hard check (final gate): every selected analysis must have a recorded
// `review_doc` (a rendered + quire-validated SpecReview doc on disk) before
// the run can be accepted. 'base' selects nothing, so it passes trivially;
// 'all' expands to the canonical seven when the agent did not echo the list.
const selectedAnalysesCovered = ({ instance }) => {
  const request = findRequest(instance);
  const reviewSet = norm(request?.review_set);
  let selected = Array.isArray(request?.selected_analyses)
    ? request.selected_analyses.map(norm).filter(Boolean)
    : [];
  if (reviewSet === "all" && selected.length === 0) {
    selected = [...ALL_ANALYSES];
  }
  if (selected.length === 0) return true;

  const ran = new Set(
    (instance.items?.review_doc ?? [])
      .map((doc) => norm(doc?.analysis))
      .filter(Boolean),
  );
  const missing = selected.filter((analysis) => !ran.has(analysis));
  return (
    missing.length === 0 || {
      ok: false,
      code: "selected_analyses_not_run",
      details: { reviewSet, selected, missing },
    }
  );
};

export const invariants = {
  ...specInvariants,
  selected_analyses_covered: selectedAnalysesCovered,
};
