---
name: spec-architecture-evaluation
description: Evaluate a repository's implemented architecture against its stated boundaries, quality goals, and operating scenarios, producing an evidence-backed SpecReview. Use for architecture reviews; do not use for general test-gap or code-style review.
---

# Architecture Evaluation

Evaluate the architecture that exists, including its specifications and code. Do not infer health from diagrams, test counts, or a single aggregate score.

## Evaluation

1. Establish the scope and named quality goals from the repository's architecture decisions, requirements, and current implementation.
2. Walk representative success, failure, recovery, and change scenarios across component boundaries. Follow data, control, and ownership; record the concrete files or artifacts that establish each claim.
3. Check whether responsibilities have one clear owner, dependencies point in the intended direction, failure and recovery cross boundaries coherently, and measurement or policy code is separated from presentation and orchestration.
4. For each problem, state what is wrong, where it occurs, why it matters in a named scenario, and the smallest credible repair. Distinguish a demonstrated defect from a risk that still needs measurement.
5. Prefer a small number of supported findings. Do not invent a numeric architecture score or treat an unavailable metric as zero.

## Output contract

Write one Quire-validated `SpecReview` under `reviews/YY-MM-DD-<slug>.md` with:

- `type: SpecReview`
- `analysis: architecture-evaluation`
- an exact `scope` and `review_set`
- a substantive `## Summary`
- a `## Findings` table with the module's required columns

Each finding must name evidence in `Refs` and contain enough location and causal detail to be actionable. If no defect is demonstrated, record a low-severity “No issues found” row and summarize the scenarios actually examined.

Validate the result with the repository's real `quire validate` command. This review is advisory evidence unless the repository separately declares it as a gate.
