# Assurance Artifact Selection

Use this reference only when the user requests assurance artifacts or the agreed change
explicitly needs an accountable assurance, architecture, or measurement decision. These
types come from an installed module; always obtain their live skeletons and schemas from
the single `quoin write` call made by the parent skill.

## Choose the smallest useful set

- `AssuranceProfile` records bounded scope, project-owned impact scenarios, selected
  practices, review selection, evidence expectations, independence, and exceptions.
  Author it when those choices need an accountable artifact—not merely because the
  repository supports the type.
- `ArchitectureDescription` records concerns, views, decisions, quality scenarios,
  conformance rules, and unresolved risks. Author it when design structure or trade-offs
  are part of the requested decision.
- `MeasurementPlan` defines one reproducible observation and its interpretation stage.
  Author it when a quality claim, performance expectation, or code-health observation
  will be compared or influence a decision.

An ordinary low-impact requirement change needs none of these unless the request says
otherwise. A profile may call for the other two, but its applicability and selected
scope must be established rather than inferred from the mere presence of the module.

## Link to existing intent

Use only relationship shapes admitted by the fetched schema. Link the artifact to the
exact requirements, decisions, or bounded system it qualifies. Do not duplicate the
requirement text inside the assurance artifact, manufacture missing graph edges, or use
an assurance link as evidence that the underlying claim is satisfied.

## Preserve the confirmation boundary

Validate the authored AP/AD/MP files with the rest of the requested scope, then stop at
the same confirmation boundary as ordinary specification authoring. Recommend an
applicable review set from the profile when present, but do not launch reviews,
measurements, or a guided workflow unless the request already includes them or the user
confirms the next step.
