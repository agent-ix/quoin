---
name: spec-review
description: Review requirements for quality, consistency, and completeness.
metadata:
  ix-flow-workflows: ./workflows
  ix-flow-invariant-provider:
    command: quoin
    args: [workflow-invariants]
    request-protocol: quoin.workflow-invariants/v1
    result-protocol: quoin.workflow-invariants-result/v1
    invariants:
      - review_selection_consistent
      - selected_analyses_covered
      - human_acceptance_required
---

# Review Requirements

If this plugin is not initialized or an Agent IX command fails, read [the quoin setup guide](https://github.com/agent-ix/quoin/blob/main/setup.md) for its prerequisites and local diagnosis.

Use this skill to validate requirements artefacts before implementation.

## Checklist

See **[references/checklist.md](references/checklist.md)** for the detailed quality gates.

## Choose the review set

Inspect applicable active `AssuranceProfile` documents using the installed catalog.
The current contract is `review_policy.mode` (`recommend` or `require`) and
`review_policy.operations` (`spec-review`, `code-review`, `gap-analysis`).
Record the actual profile paths and an applicability disposition, or explicitly
state why none applies. Resolve ambiguous applicability with the owner.

A required `spec-review` operation needs an accepted enforced review run.
The profile does not select individual analysis lenses. Requirements for
`code-review` or `gap-analysis` remain separate operations; a specification
review cannot mark those complete. Never author or interpret `review_selection`.

## Review scope and method selection

Review effort is proportional to the change. The agent measures the change,
selects the methods itself, and records the choice. It never stops to ask the
user which methods to run; ask only when the review would otherwise be
impossible (no diff or target can be determined, or profile applicability is
ambiguous). The user may still name methods up front; follow that instead.

### 1. Measure the change

Measure from the diff (`git diff --name-status <base>...HEAD` and the changed
hunks). Classify each touched file and requirement id by what changed:

- requirement text (statement, acceptance criteria rows, rule wording),
- frontmatter relationships only (links, `refines`, `depends`, and similar),
- test-case rows only,
- an ADR, shared type, shared format or other cross-cutting artifact.

Count the requirements touched and the cross-references the change adds or alters.

### 2. Scope

| Signal in the diff | Scope |
| --- | --- |
| Frontmatter links only; or test-case rows only; or a typo or non-normative edit | Small: the changed text and its directly linked artifacts |
| Wording or criteria of one requirement; up to about 3 requirements in one area; no ADR or shared type | Small: the changed text, the requirements it links to by frontmatter relationships, the tests and code criteria it constrains, the sibling requirement that must agree |
| New or restructured rule across routes or several requirements; 4 or more requirements in more than one area; new cross-references among them | Large: the wider set below, over every requirement touched and its neighbours |
| An ADR, shared type or shared format changes; any change that alters who owns a responsibility or a boundary | Large |

When signals disagree, take the larger scope. Small scope never reads the whole
spec tree; large scope reads every requirement touched plus its linked neighbours.

### 3. Select methods

Base (the Checklist, scoped to the changed text and its neighbours) always runs.
Add methods by what the change needs:

| What changed | Methods added to base |
| --- | --- |
| Frontmatter links only | dependency (links check only) |
| Test-case rows only | evidence on the touched criteria |
| Wording change in one requirement | ears-conformance and integrity, on that requirement |
| New criterion or rule in one requirement | ears-conformance, integrity, evidence on that requirement |
| New rule across routes or several requirements | integrity, dependency, object review (`spec-object-review`), failure-domain |
| Restructure, new shared type or format | the row above plus scope-boundary |
| New component, responsibility or interface boundary | scope-boundary, dependency, integrity |
| Volatile, novel or hard to estimate requirement | risk-complexity |
| ADR or security-relevant change (trust boundary, authz, secrets, input handling) | the matching rows above plus `spec-security-analysis` |
| Criteria that may be unfalsifiable | `spec-criterion-strength-analysis` on those criteria |

Rows combine: take the union. Run only the methods the union names. The seven
lenses are failure-domain, integrity, dependency, evidence, risk-complexity,
scope-boundary and ears-conformance (skills `spec-failure-domain-analysis`,
`spec-integrity-analysis`, `spec-dependency-analysis`, `spec-evidence-analysis`,
`spec-risk-complexity-analysis`, `spec-scope-boundary-analysis`,
`spec-ears-analysis`). Object review, security and criterion-strength are
additional direct methods, not enforced-run lenses.

### 4. Record the choice

Write the measured change, the scope, the selected methods and the reason for
each in the `## Summary` of every review document (the artifact header), for
example `Scope: small (1 requirement, wording only). Methods: base, ears-conformance,
integrity. Reason: wording change, no link or rule change.` Methods not run are
listed with why they were skipped.

In an enforced run, record `review_set: base` when only base runs, `all` only
when the table selects all seven lenses, otherwise `subset` with the chosen
lenses in `selected_analyses`. Direct-only methods go in the Summary and their
own documents, not in `selected_analyses`.

### 5. Re-review after fixes

After a fix round, check the fix diff only, plus any requirement the fix newly
touches (apply sections 1 to 4 to that diff). Do not repeat the full review
unless a rule changed; if a rule changed, review that rule and what links to it.

## Process

1.  Measure, scope and select (see above), then Automated Checks (always — part of `base`):
    -   Validate ID formats (US/FR/TC/AC/CON).
    -   Detect duplicates or gaps.
    -   Validation link integrity.
2.  Base Review (always):
    -   Read each artifact.
    -   Verify against the **Checklist**.
    -   Verify Test Coverage (6 Rules).
    -   Flag every hash / digest / pin / tracking-record antipattern as a P0,
        `high` finding with a FAIL verdict (see the
        **Hash / Digest / Pin Antipattern** section of the Checklist). The fix is removal;
        do not accept an existing use as a reason to keep it.
3.  Selected Analyses:
    -   Run each analysis selected in **Select methods**. Prefer running them **in parallel**.
    -   **Fetch the template from quoin** once with `quoin write . --types SpecReview`
        from the target repository — use its skeleton + schema as the contract.
    -   Write **one `SpecReview` document per analysis** to the repo-root
        `reviews/YYYY-MM-DD-<slug>-<analysis>.md` (plus `reviews/YYYY-MM-DD-<slug>-base.md`
        for the base checklist), the same root `gap-analysis` uses; never `spec/reviews/`.
        A repo that pins a different review root wins. Findings go in a validated
        `## Findings` table (`| ID | Severity | Summary | Refs |`, `FND-NNN` ids, Severity ∈
        `low`/`medium`/`high`).
4.  Validate + record:
    -   Run `quire validate --scope <repo> "spec/**/*.md"` and fix any errors.

> **`--scope` is the repository root, and must be passed explicitly.** The command derives **two roots** from it and never
> interchanges them: spec documents are read from `<repo>/spec` only, trace tags from
> the source tree at `<repo>` excluding `spec/`. A repo with no `spec/` exits with a
> diagnostic naming the missing document root rather than scanning the whole tree, and a
> matrix outside `spec/` (a fixture, a `plan/` copy) mints nothing. A relative glob
> resolves under `--scope` only in scoped mode (no `--module`); with `--module` it
> resolves against the process working directory, and an omitted `--scope` defaults to
> `.` — so a run launched from a parent directory validates the **wrong tree** and exits
> 0 for whatever it matched.

    -   Do not proceed to implementation until reviewed.

## Enforced runs (workflow mode)

This installed skill contains `workflows/review/def.yaml`. Start from the subject
repository, using the absolute directory containing this SKILL.md:

```bash
ix-flow run review --path <this-skill-directory> --id <run-id> --target <repo-relative-path> --json
```

Before advancing, use `ix-flow add-item <run-id> quoin_context --item '<json>'`
to record exactly one item with `id: quoin-launch-context`, absolute `repo_root`,
absolute `config_root` (normally ~/.ix), and absolute contained `targets`.
Alternatively `quoin review --path <this-skill-directory> --target <repo-relative-path>
--id <run-id> --json` starts the same definition and initializes that context.
Both commands require the installed ix-flow CLI >=0.3.1 and Quoin CLI >=0.29.1
on PATH. The plugin supplies the definitions; no archived provider is used.
Use the same --config-root and --state-dir for every ix-flow command.
See [workflow instructions](references/workflow.md). Record the `request` interview with `review_set`,
`selected_analyses`, `assurance_profiles`, and `profile_disposition`. Register
one `review_doc` item per analysis, including `base`, with its `id`, `analysis`,
and repository-relative `path`. The Rust provider opens and validates every
required document before validation and again before acceptance. Names or old
validation claims alone cannot pass. `all` always includes the seven lenses above; the agent chooses the set (see Review scope and method selection).

The final transition requires explicit human acknowledgement; never automatically
acknowledge it. A failed enforced run cannot be claimed complete from direct
analyses. Direct review remains available when no applicable policy requires an
enforced spec-review operation. The gate verifies artifact validity, not reasoning
quality. Existing documents can be registered in a new owned run and revalidated;
old external-provider runs are not silently migrated or retrospectively accepted.

## Common Issues

-   Vague Criteria: "Fast", "User-friendly". -> Make measurable.
-   Missing Errors: Only happy path. -> Add error modes.
-   No Coverage: AC without TC. -> Add tests.
-   Inconsistent IDs: `US-1` instead of `US-001`. -> Fix formatting.

## Output contract

The `SpecReview` this skill emits carries `analysis: base` — the generic review value — this skill runs the analysis lenses and writes one document per lens, so it is the default rather than a lens of its own.

Stated here because nothing checked it (quoin#202): a skill and its declared
output were unlinked, so drift between them was invisible until a document
failed validation in an unrelated session.
