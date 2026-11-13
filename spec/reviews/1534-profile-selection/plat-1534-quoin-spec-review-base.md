---
id: SR-20261113140009
title: "Base review of source profile selection (FR-180, FR-181, FR-182, NFR-037)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #870 (git diff origin/main...plat-1534-profile-selection-spec at 9e8997c3): spec/functional/FR-180, FR-181, FR-182, spec/non-functional/NFR-037, spec/functional/index.md, spec/non-functional/index.md, spec/spec.md"
---

# Base review of source profile selection (FR-180, FR-181, FR-182, NFR-037)

## Summary

Ticket: PLAT-1534. Checklist review of the four new artifacts and the index edits. `quire validate --scope .`
over the seven changed files exits 0. The repo-wide run fails only on FR-113 (`row-width`), which this PR does not
touch. The diff names no licensed pack and carries no standard text and no roadmap content.

The blocking defect is an id collision. The parallel PLAT-1426 lane (quoin PR #871, head 733d0dfb) adds FR-180 to
FR-183. This PR adds FR-180 to FR-182, so whichever PR merges second breaks the other. The same collision exists in
spec-objects-security: PR #48 (this ticket) adds FR-020 and FR-021, and PR #49 (PLAT-1426) adds FR-020 to FR-024.
The NFR ids do not collide (NFR-037 against NFR-040 here, NFR-002 against NFR-004 there). The brief's statement
that PLAT-1426 reserves FR-171 to FR-179 does not match PR #871, which uses FR-180 to FR-183.

AssuranceProfiles AP-201 and AP-202 are active with `review_policy.mode: require`. Their scope is finding scoring and
Jev lens evaluation, so they do not apply to source profiles.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-180, FR-181 and FR-182 collide with quoin PR #871 (PLAT-1426), which adds FR-180 to FR-183 under other titles. The companion spec-objects-security PR #48 collides the same way with PR #49 (FR-020, FR-021 against FR-020 to FR-024). Renumber one lane in both repos, with every `ix://` target and link updated. | spec/functional/FR-180-source-profile-selection.md:2; spec/functional/FR-181-source-profile-evaluation.md:2; spec/functional/FR-182-local-pack-joins-a-profile.md:2 |
| FND-002 | medium | FR-180-AC-7 says that with no selection file no clause set is evaluated. FR-182 appends user-layer `add` entries but does not say what happens when the user file exists and the project file does not. The two FRs disagree on that case. | spec/functional/FR-180-source-profile-selection.md:62; spec/functional/FR-182-local-pack-joins-a-profile.md:28 |
| FND-003 | medium | FR-182 says the project file sits "in a config root that is not published". Under FR-004 the project config root is `<cwd>/.ix`, inside the repository. Nothing stops a licensed pack's key going into a committed project `add`, and FR-182-AC-6 treats a key in both layers as `project`. The ticket requires that no licensed pack be named in a published file. Refuse (or at least report) a project-layer `add` key whose declaring module is a pack (FR-137), and add an AC for it. | spec/functional/FR-182-local-pack-joins-a-profile.md:16-21, :43 |
| FND-004 | medium | FR-180 does not type the values of the selection file's `context`. The engine context is string to string, and FR-020 requires string values in a profile. A YAML value such as `level: 2` is an integer, so one implementer would refuse it and another would stringify it. Define the type and add an AC. | spec/functional/FR-180-source-profile-selection.md:26-30 |
| FND-005 | medium | NFR-037-AC-3 tests "the report schema", but FR-181 gives the profile report no schema and no `schemaVersion`, only "The report is JSON". As written the AC has nothing to test. Name and define the report contract in FR-181. | spec/non-functional/NFR-037-source-profile-evaluation-is-deterministic-and-states-only.md:47; spec/functional/FR-181-source-profile-evaluation.md:31-34 |
| FND-006 | medium | Three FR-180 Behavior statements have no AC: reading profiles from module `profiles/` and from nowhere else (for example, a repo-local `profiles/p.yaml` is ignored); the SHALL NOT on inferring a profile; and the SHALL NOT on changing a version (a profile entry whose version no module declares). | spec/functional/FR-180-source-profile-selection.md:44-50 |
| FND-007 | medium | FR-180 reads profile files but never validates one: not against `source-profile-v1` (spec-objects-security FR-020), and not against a `name` that differs from the file stem. It has no refusal for a malformed profile, so nothing enforces FR-020 when a profile is used. Add a Behavior line and an AC that refuse a profile which fails the format. | spec/functional/FR-180-source-profile-selection.md:42-50 |
| FND-008 | low | The new index entries use `*` bullets. Both index files use `-`. | spec/functional/index.md:158-160; spec/non-functional/index.md:41 |

## Verdict

Not mergeable as is: FND-001 must be resolved with the PLAT-1426 lane before either PR lands. The design is sound
and decides each quoin-side scope item. Profiles are module data found under `profiles/`. A repo selects one profile
through `source-profile.yaml`. Every set is evaluated through the engine and a missing set is reported as
`unavailable`. A licensed pack joins through the user-layer `add`.

Examined (role examined): every Behavior statement and AC of FR-180, FR-181, FR-182 and NFR-037; the
index and spec.md entries. Context only: quoin FR-004, FR-006, FR-046, FR-137, FR-138; quire-rs
`clause-binding-v1` schema and `ClauseSet::evaluate`; quoin PR #871 file list.
