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

const normalizedList = (value) =>
  Array.isArray(value) ? [...new Set(value.map(norm).filter(Boolean))] : [];

const sameSet = (left, right) =>
  left.length === right.length && left.every((value) => right.includes(value));

function findRequest(instance) {
  const items = instance.items?.operation_request ?? [];
  return (
    items.find((item) => item && item.interviewId === "request") ?? items[0]
  );
}

const reviewSelectionConsistent = ({ instance }) => {
  const request = findRequest(instance);
  const reviewSet = norm(request?.review_set);
  const selected = normalizedList(request?.selected_analyses);
  const profileMode = norm(request?.profile_selection_mode || "none");
  const profileAnalyses = normalizedList(request?.profile_analyses);
  const known = new Set(ALL_ANALYSES);
  // Recommendations are advisory: only selections that will actually run need
  // to be supported by the installed SpecReview schema. Required selections
  // must be supported in full before the workflow may advance.
  const analysesToValidate =
    profileMode === "require" ? [...selected, ...profileAnalyses] : selected;
  const unsupported = analysesToValidate.filter(
    (analysis) => !known.has(analysis),
  );
  if (unsupported.length > 0) {
    return {
      ok: false,
      code: "unsupported_selected_analysis",
      details: { unsupported: [...new Set(unsupported)].sort() },
    };
  }
  if (!["base", "all", "subset"].includes(reviewSet)) {
    return { ok: false, code: "invalid_review_set", details: { reviewSet } };
  }
  if (profileMode === "require") {
    if (reviewSet !== "subset") {
      return {
        ok: false,
        code: "required_profile_review_set",
        details: { reviewSet },
      };
    }
    if (!String(request?.assurance_profile ?? "").trim()) {
      return { ok: false, code: "required_profile_path_missing", details: {} };
    }
    if (profileAnalyses.length === 0 || !sameSet(selected, profileAnalyses)) {
      return {
        ok: false,
        code: "required_profile_selection_mismatch",
        details: { profileAnalyses, selected },
      };
    }
    return true;
  }
  if (reviewSet === "base" && selected.length > 0) {
    return { ok: false, code: "base_selects_analyses", details: { selected } };
  }
  if (
    reviewSet === "all" &&
    selected.length > 0 &&
    !sameSet(selected, ALL_ANALYSES)
  ) {
    return {
      ok: false,
      code: "all_set_incomplete",
      details: {
        selected,
        missing: ALL_ANALYSES.filter((item) => !selected.includes(item)),
      },
    };
  }
  if (reviewSet === "subset" && selected.length === 0) {
    return { ok: false, code: "subset_is_empty", details: {} };
  }
  return true;
};

// Hard check (final gate): every selected analysis must have a recorded
// `review_doc` (a rendered + quire-validated SpecReview doc on disk) before
// the run can be accepted. 'base' selects nothing, so it passes trivially;
// 'all' expands to the canonical seven when the agent did not echo the list.
const selectedAnalysesCovered = ({ instance }) => {
  const request = findRequest(instance);
  const reviewSet = norm(request?.review_set);
  const profileMode = norm(request?.profile_selection_mode || "none");
  let selected = normalizedList(request?.selected_analyses);
  if (profileMode === "require") {
    selected = normalizedList(request?.profile_analyses);
  } else if (reviewSet === "all") {
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
  review_selection_consistent: reviewSelectionConsistent,
  selected_analyses_covered: selectedAnalysesCovered,
};
