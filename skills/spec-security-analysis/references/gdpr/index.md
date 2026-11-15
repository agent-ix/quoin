# GDPR reference index

## Typed clause set

The evaluated GDPR material is limited to the exact set
[`eu/gdpr-2016-679@2016-05-04`](https://github.com/agent-ix/spec-clauses-eu-law/blob/main/spec_clauses_eu_law/clauses/eu-gdpr-2016-679-2016-05-04.json), only when that exact key is present in the source-profile evaluation. Its complete typed clause boundary is:

- Article 25(1)-(2)
- Article 30(1)(a)-(g), (2)(a)-(d), and (3)-(4), with applicability determined by the person-supplied Article 30 records-applicability value
- Article 32(1)(a)-(d), (2), and (4)
- Article 35(1), (2), (7)(a)-(d), (9), and (11), with applicability determined by person-supplied role and high-risk-processing values

Do not infer or supply those applicability values. The evaluation document is
the only source of clause outcomes; in generated artifacts, the exact set key
and clause id are the whole reference. Other GDPR clauses, including breach
notification, are not evaluated by this set. This index is the sole authority
for the typed boundary and applicability rules; [Chapter IV](ChIV-ControllerProcessor.md)
and other skill pages are navigation only.

## Untyped context

The following retained chapter summaries are untyped context only. They are not
clause sets, are not evaluated, and must not be reported as evaluated coverage:

- [Chapter II - Principles](ChII-Principles.md)
- [Chapter III - Data Subject Rights](ChIII-DataSubjectRights.md)
- [Chapter V - Data Transfers](ChV-DataTransfers.md)

These files remain context until future clause sets cover their topics. Do not
map controls from their summaries or infer obligations from them.
