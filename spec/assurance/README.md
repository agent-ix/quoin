---
type: index
title: "Quoin Assurance Artifacts"
description: "Index of quoin's own AssuranceProfile and MeasurementPlan instances."
---

# `spec/assurance/`

This directory holds **quoin's own** assurance artifacts: `AssuranceProfile`
(`AP-*.md`) and `MeasurementPlan` (`MP-*.md`) instances that state what quoin
measures about itself and how, not documentation about assurance in general.

The schemas and Markdown skeletons these files are validated against are not
defined here — they are defined and shipped by
[`agent-ix/engineering-assurance`](https://github.com/agent-ix/engineering-assurance),
installed as a Quire module (see that repo's README for the install command
and the current `AssuranceProfile`/`MeasurementPlan` skeletons). `quire
validate` checks the files in this directory against that installed module.

This is the artifact-authoring half of a two-repo split:

- `engineering-assurance` defines the artifact types, schemas, and skeletons —
  shared across every repository that adopts them.
- `spec/assurance/` (here) holds one project's — quoin's — actual instances of
  those types.

For the deeper question of whether quoin should *consume* engineering-assurance's
own evidence-accounting and measurement code (not just its schemas), see
`docs/engineering-assurance-adoption.md` in this repo, and the still-open cross-repo
gap tracked at
[`agent-ix/engineering-assurance#98`](https://github.com/agent-ix/engineering-assurance/issues/98).
That question is not resolved by this README.

## Contents

- [AP-201: Quoin finding quality and measurement reporting profile](./AP-201-finding-quality.md)
- [AP-202: Jev semantic lens evaluation profile](./AP-202-jev-lens-evaluation.md)
- [MP-201: Per-family finding precision](./MP-201-finding-precision.md)
- [MP-202: Per-family detection recall](./MP-202-finding-recall.md)
- [MP-203: Unadjudicated advisory finding count](./MP-203-unadjudicated-findings.md)
- [MP-204: Property span-grounding rate](./MP-204-span-grounding.md)
- [MP-205: Finding actionability rate](./MP-205-actionability.md)
- [MP-206: Cost per confirmed insight](./MP-206-confirmed-insight-cost.md)
- [MP-207: Benchmark silent-zero sentinel](./MP-207-silent-zero.md)
- [MP-208: Minting section-hit rate](./MP-208-minting-section-hit.md)
- [MP-209: Confirmed finding localisation rate](./MP-209-finding-localisation.md)
- [MP-210: Controlled-corpus detection recall](./MP-210-detection-recall.md)
- [MP-211: Controlled-corpus bounds gap count](./MP-211-corpus-gap-count.md)
- [MP-212: Finding actionability v2 rate](./MP-212-actionability-v2.md)
- [MP-213: Property span correctness](./MP-213-span-correctness.md)
- [MP-214: Safe property-span refusal](./MP-214-safe-span-refusal.md)
- [MP-215: Labeled property span outcomes](./MP-215-span-grounding-v2.md)
- [MP-216: Independent action-guidance correctness](./MP-216-guidance-correctness.md)
- [MP-217: Executable guidance repair success](./MP-217-guidance-repair-success.md)
- [MP-218: Executable diagnostic guidance yield](./MP-218-guidance-diagnostic-yield.md)
- [MP-219: Broad real-repository span grounding](./MP-219-span-breadth.md)
- [MP-220: CLI agent evaluation terminal-sentinel contract](./MP-220-cli-eval-sentinel-contract.md)
- [MP-222: Jev lens margin over the constant predictor](./MP-222-jev-constant-predictor-margin.md)
- [MP-223: Jev lens defect recall](./MP-223-jev-defect-recall.md)
- [MP-224: Jev lens no-defect answer recall](./MP-224-jev-no-defect-recall.md)
- [MP-225: Jev lens disagreement under repetition](./MP-225-jev-disagreement-under-repetition.md)
- [MP-226: Jev lens expected calibration error](./MP-226-jev-calibration-error.md)
- [MP-227: Jev lens cost, latency and throughput per pass](./MP-227-jev-cost-and-latency.md)
- [MP-228: Jev criterion-strength lens gate](./MP-228-jev-criterion-strength-gate.md)
- [MP-229: Jev EARS lens gate](./MP-229-jev-ears-gate.md)
- [MP-230: Jev gap-analysis lens gate](./MP-230-jev-gap-analysis-gate.md)
- [MP-231: Jev EARS engine/semantic delta rate](./MP-231-jev-ears-engine-semantic-delta.md)
- [MP-232: Gap-analysis lens agreement against mechanical mutation truth](./MP-232-jev-gap-analysis-mutation-truth-agreement.md)
- [MP-233: Gap-analysis battery agreement against targeted-mutation and static-check truth](./MP-233-jev-gap-analysis-battery-ground-truth.md)
- [MP-234: Gap-analysis battery's remaining four questions against agent-labelled truth on the shipped corpus](./MP-234-jev-gap-analysis-remaining-four-agent-labelled.md)
- [MP-240: Jev severity variants S1-S3 against S0 on corpus v2](./MP-240-jev-severity-variants.md)
- [MP-241: code_exceeds_requirement variants E1-E4 against the as-asked question on corpus v2](./MP-241-jev-code-exceeds-variants.md)
- [MP-242: Jev test_asserts_intent variants T1-T2 and the trace check TC on corpus v2](./MP-242-jev-test-intent-and-trace-variants.md)
- [MP-243: Jev criterion-soundness checklist variants K1-K2 on corpus v2](./MP-243-jev-criterion-soundness-checklist.md)
- [MP-250: Commands that fail on a bad argument say why and exit non-zero](./MP-250-nfr003-actionable-failures.md)
- [MP-251: Commands that fail on a bad argument never fail silently](./MP-251-nfr003-silent-failures.md)
- [MP-252: ix-flow failures reach the caller as a non-zero exit](./MP-252-nfr007-external-tool-failures.md)
- [MP-253: Corrupt manifests abort catalog assembly](./MP-253-nfr008-corrupt-manifests.md)
- [MP-254: Candidates without a manifest are skipped](./MP-254-nfr008-missing-manifests.md)
