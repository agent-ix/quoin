---
id: SR-20261111200817
title: "Spec review of FR-135 (requirements mapping matrix view), quoin PR 814"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@6ed10483; spec/functional/FR-135-requirements-mapping-matrix-view.md, spec/spec.md, spec/functional/index.md, spec/matrix.md, rust/crates/quoin-cli/src/discharge.rs (help text)"
review_set: subset
---

# Spec review of FR-135

## Summary

Ticket: PLAT-307. This is a new FR with no prior spec-review. Sub-analyses run inline:
integrity (structure, ids, matrix rows), EARS (statement form), scope-boundary (claims
about an external standard), and failure-domain (adverse cases). `quire validate` on the
file exits 0. Its only warning is the unrelated `semantic.unused-import` of the module.
Ids are clean (see the gap analysis). The statement is EARS-ubiquitous ("Quoin SHALL
render ..."). The Constraints section (no persistence, no embedded standard text, no
score) is testable and tested. The Compliance vocabulary keeps disposition, expiry,
undischarged and unresolved apart from Compliant, which matches FR-046.

The ticket text (Linear PLAT-307, treated as data) asks for a renderer that "fills the
NPR 7150.2D Appendix C Requirements Mapping Matrix shape", with columns attributed to
"the LaRC v1.5 templates", and "Tailoring columns from N/A-with-rationale dispositions".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Overclaim about an external standard. The Description says the columns are those "the NPR 7150.2D Appendix C matrix asks for". That is unverified in the repo (the ticket attributes the columns to LaRC templates, not the NPR). The Compliance and Applicability values are Quoin's own vocabulary, not the NPR's. The spec must say both. | spec/functional/FR-135-requirements-mapping-matrix-view.md:15-25 |
| FND-002 | medium | The ticket's N/A tailoring is dropped silently. A `Not applicable` row renders `-` in both Tailoring columns, though the binding report given to `build_requirements_mapping` carries the binder's reasons for the not-binding outcome. A disposition recorded against a not-binding clause appears only as an unused fact. The spec says nothing about either. | spec/functional/FR-135-requirements-mapping-matrix-view.md:41-60 |
| FND-003 | medium | The escaping rule names only `\|` and line breaks, and AC-5's "byte-identical across two renders" cannot fail for a pure function. The spec should require that all interpolated text (cells, title, unused-fact list) renders as literal text: no Markdown, links, images or HTML. AC-5 should name the adverse inputs. | spec/functional/FR-135-requirements-mapping-matrix-view.md:66-68, 85 |
| FND-004 | medium | No AC states the honesty guarantee the Dependencies line relies on: a stored disposition with an empty or whitespace rationale refuses the operation as a bad request, and no Tailored row ever lacks a rationale. | spec/functional/FR-135-requirements-mapping-matrix-view.md:81, 86, 93-94 |
| FND-005 | low | `Not compliant - attestation not current` also covers a third FR-046 reason, "attestation expiry is not after attestation" (malformed window). The bullet names only expired or not yet valid. | spec/functional/FR-135-requirements-mapping-matrix-view.md:56-58 |

## Verdict

**MERGEABLE AFTER FND-001..FND-004 ARE FIXED.** Recommended text:

FND-001, Description: "Quoin SHALL render a requirements-mapping-matrix view of the
FR-133 discharge report: one row per clause of a live `clause-binding-v1` report, with
the columns Req#, Applicability, Compliance, Evidence, Tailoring Description and
Tailoring Rationale. The column headings follow the shape projects use for a compliance
matrix, for example center templates for the NPR 7150.2D Appendix C requirements
mapping matrix. The Applicability and Compliance values are Quoin's own vocabulary,
defined below, and are not quoted from any standard. Quoin supplies no standard's
requirement list, numbering or applicability; they come from the clause set and the
binding report. A project that submits the view to a standard maps the values itself."
Keep the CLI help wording, but use "a" rather than "the": "Render a requirements
mapping matrix view: one row per clause ...". Retitle the ticket with "an RMM-shaped
view of the discharge map" rather than "fill the NPR 7150.2D Appendix C matrix".

FND-002, minimal change: FR-046 has no N/A disposition kind (decisions are
accepted_risk, temporary_exception and delegated), so the N/A rationale that exists is
the binder's derivation. For a `Not applicable` row, set Tailoring Description to
`Not applicable per binding derivation` and Tailoring Rationale to the clause's
`reasons[].message` joined with "; ". These come from the binding report already passed
in, so rmm.rs needs no store read. When the binder gave no reason, the rationale is
`no reason given by the binding report`, never blank. Add a spec sentence that a
disposition recorded against a not-binding clause is not tailoring and stays in Unused
facts, because no disposition kind means "not applicable". Add it to AC-3 and TC-3202.

FND-003: "Every string taken from the binding report or a stored fact renders as
literal text: a pipe, a backslash, a line break, Markdown emphasis, link or image
syntax, and HTML render as the characters given and cannot add a row, a cell, a block,
a link, an image or an element." AC-5: "... a cell, the title or an unused-fact entry
containing `\|`, a line break, `![x](y)`, `[t](u)` or `<img>` renders as literal text
within its row or line."

FND-004: add to AC-6: "a stored disposition with an empty or whitespace-only rationale
refuses the operation as a bad request and renders no matrix".
