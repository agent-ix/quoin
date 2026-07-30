---
id: US-010
title: "Author specs for my own organization"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# US-010: Author specs for my own organization

## Story

**As a** developer adopting quoin in a repository my own organization owns
**I want** quoin to know which organization that is and put it in the specs it
helps me author
**So that** my artifacts carry my organization's identity instead of a value
copied from whoever wrote the templates.

This story expresses the adopter's perspective in informal language and does not
prescribe where the organization is read from or how it is passed along.

## Context

Every root spec artifact must declare an owning organization — it is a required
field, and the cross-repository `ix://` references authors write are qualified by
it. But quoin currently offers no way to tell it which organization the author
belongs to: there is no flag, no environment variable, and no configuration key
for one. The only path is for the author (or the agent acting for them) to type a
value into frontmatter by hand.

That leaves the value to guesswork, and the material quoin hands an author points
two different ways at once. Its own root-spec template leaves a fill-in-the-blank
placeholder, while the catalog skeleton the `write` command points at carries a
real organization name that happens to be the one quoin's own authors work in. An
adopter outside that organization gets whichever of the two the agent copied —
either a placeholder left in a finished document, or, worse, another
organization's name sitting in their specs looking entirely legitimate.

The repository itself usually already knows the answer: its remote points at the
organization that owns it. A sibling tool in the ecosystem already reads exactly
that, and treats a repository it cannot identify as unidentified rather than
guessing, so that two same-named repositories in different organizations never
collide.

## Acceptance Examples (Illustrative)

These examples clarify the adopter's expectations. They are illustrative only —
not test cases and not verification criteria.

### US-010-EX-1: The repository answers for itself

- **Given** a developer in a repository whose remote belongs to their organization
- **When** they ask quoin for the authoring contract for a root spec
- **Then** quoin tells them their organization, and they never type it themselves

### US-010-EX-2: Overriding what the repository says

- **Given** a developer whose repository remote does not reflect the organization
  the spec belongs to
- **When** they state the organization explicitly for that invocation
- **Then** what they stated is used in preference to the remote

### US-010-EX-3: Nothing to go on

- **Given** a developer in a directory with no remote to read an organization from
- **When** they ask quoin for the authoring contract
- **Then** quoin says it could not determine the organization and tells them how
  to supply one, rather than choosing a value on their behalf

## Constraints (Contextual)

Adopters noted that quoin is expected to work in a fresh checkout with nothing
installed beyond quoin itself, so reading the organization should not depend on
other tooling being present. This context is not binding and may be refined during
requirements analysis.

## Dependencies (Contextual)

Relationships observed during discovery. Upstream: the standalone-CLI stakeholder
need ([StR-001](../stakeholder/StR-001-standalone-cli.md)). Downstream: a likely
functional requirement for resolving the organization, and a change to the
configuration surface to carry it. These are potential relationships, not formal
traceability.

## Notes (Informative)

Open question raised in discovery: the organization an author declares in
frontmatter and the organization a downstream consumer uses when it mints
references are, today, independent of each other. Whether the declared value
should become the authoritative one is captured here for later analysis; it
introduces no requirement.

## Traceability (Informative)

This user story traces to the stakeholder need for a standalone CLI that any
author can run, in any organization, without editing what quoin ships. Links may
be updated as understanding evolves.
