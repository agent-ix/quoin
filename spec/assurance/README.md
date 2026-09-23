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
