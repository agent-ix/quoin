---
name: quoin-review
description: Quoin-owned review workflow with native validation and explicit human acceptance.
metadata:
  ix-flow-workflows: ./workflows
  ix-flow-invariant-provider:
    command: __QUOIN_EXECUTABLE__
    args: [workflow-invariants]
    request-protocol: quoin.workflow-invariants/v1
    result-protocol: quoin.workflow-invariants-result/v1
    invariants:
      - review_selection_consistent
      - selected_analyses_covered
      - human_acceptance_required
---

# Quoin specification review

Quoin owns this definition and its Rust invariant provider. ix-flow owns state,
transitions and acknowledgements. Start through `quoin review`; it records one
`quoin_context` with absolute repository/config roots and target paths.

1. Inspect the applicable active AssuranceProfile documents using the installed
   catalog. Record applicable paths and an explicit applicability disposition.
   Follow `review_policy.mode` and `review_policy.operations`; a profile does not
   select specification lenses. Resolve ambiguous profile applicability with the
   owner. Never invent `review_selection` or claim code-review/gap-analysis is
   fulfilled by specification review.
2. Obtain the user's base/all/subset selection. Record `request` interview answers
   with `review_set`, `selected_analyses`, `assurance_profiles` (possibly empty),
   and `profile_disposition`. Base is always required. The all set is:
   __QUOIN_LENSES__.
3. Advance to analyses_run, perform the base checklist and selected lenses, then
   obtain each live artifact contract with `quoin write <repo> --types SpecReview`.
   Author one SpecReview per required analysis with a valid Findings table.
4. Register every document using `ix-flow add-item <id> review_doc --item
   '{"id":"base-review","analysis":"base","path":"spec/reviews/base.md"}'`.
   Paths are resolved under the recorded repository root, never the runtime directory.
5. Advance through reviews_rendered and validated. Quoin opens and validates the
   actual documents on the validation and acceptance transitions. A recorded name
   or an old claimed validation result cannot satisfy the gate.
6. Advance toward accepted, present the actual results and wait for explicit human
   approval. Use `ix-flow ack` only after that approval, then advance again.

Pass the same --config-root and --state-dir returned at launch to every ix-flow
command. `ix-flow status` and `resume` report progress and pending actions.
Never automatically acknowledge acceptance, suppress a failed invariant, or claim
an enforced run completed from direct analyses. The gate verifies artifact validity,
not the quality of the reasoning inside an artifact.
