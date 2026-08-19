---
id: SR-009
title: "Adoption report — ISO 24495 plain-language tooling"
type: Review
analysis: adoption-analysis
scope: "GaZmagik/iso-24495, quire-rs lint and grammar surfaces, quire-cli lint, spec-artifacts-iso module"
review_set: full
---

## Summary

`iso-24495` is worth adopting as a source of language-quality rules, parser fixtures,
and audit workflow ideas. It should not become a second document engine in the Quoin
ecosystem.

Its executable engine is TypeScript run with Bun. It is a deterministic, hard-coded
plain-language audit over reader-visible Markdown blocks. It is not Rust, does not
understand Quire archetypes or extraction contracts, and does not provide Quire's
module-driven rule extension model.

The recommended architecture is:

```text
quire-rs Markdown parser and AST
        ↓
reader-prose block view
        ↓
typed language rules + module language profile
        ↓
existing Quire lint findings and corpus reports
        ↓
quire-cli, Quoin review skills, and ISO module authoring guidance
```

## What the external repository contains

The repository ships six agent skills. Parts 1–3 provide core, legal, and technical
writing guidance. Part 4 provides a provisional organisational implementation audit.
Part 5 provides provisional document-design guidance. The text-audit skill is an
explicitly invoked audit over selected Markdown or text paths.

The implementation worth studying is the shared Part 4/text-audit engine:

- a custom reader-oriented Markdown block parser;
- exclusions for front matter, fenced or indented code, tables, task markers, and
  alert labels;
- deterministic rules for sentence length and averages, paragraph density, headings,
  undefined acronyms, legalese, doublets, wordy phrases, complex words, double
  negatives, filler openings, link text, image alternative text, and prose lists;
- project-specific acronym configuration;
- stable `{rule, line, detail}` findings, per-rule totals, skipped-entry reporting,
  and a configuration hash;
- a substantial CommonMark-shaped fixture corpus and robustness tests.

The project itself labels its quantitative thresholds as proxies rather than ISO
clauses. Parts 4 and 5 are provisional drafts, and the repository makes no ISO
conformance claim. See the [upstream README](https://github.com/GaZmagik/iso-24495)
and the [Part 4 skill](https://raw.githubusercontent.com/GaZmagik/iso-24495/main/skills/iso-24495-4/SKILL.md).

## Current integration points

Quire already has the right boundaries:

| Existing surface                | Evidence                                              | Integration use                                                         |
| ------------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------------- |
| Markdown parser and section AST | `quire-rs/src/parser/`, `quire-rs/src/ast.rs`         | Supply structural context and source locations.                         |
| Declarative advisory lint       | `quire-rs/src/lint.rs`                                | Add typed prose rules without changing validation semantics.            |
| Requirement grammar             | `quire-rs/src/grammar/`                               | Keep EARS and requirement-quality checks separate from reader prose.    |
| Module registry                 | `quire-rs/src/registry.rs`                            | Merge language profiles, vocabulary, and severity once per registry.    |
| CLI lint command                | `quire-cli/src/commands/lint.rs`                      | Expose per-document findings through the existing stderr/JSON contract. |
| ISO module manifest             | `spec-artifacts-iso/spec_artifacts_iso/manifest.yaml` | Declare artifact scope, vocabulary, thresholds, and default severity.   |
| Quoin workflows                 | `quoin/skills/`                                       | Apply authoring and review guidance; aggregate corpus findings.         |

The ISO module already declares advisory lint rules for verification-method values,
forbidden user-story sections, and normative wording. Its manifest also carries
`lexicon`, `grammar_severity`, `property_idioms`, `observable_verbs`,
`vacuous_predicates`, and `ambiguity_terms`. A language profile is a natural extension
of this existing model.

## Adoption matrix

| Feature                                           | Decision                                                         | Owner                   | Priority |
| ------------------------------------------------- | ---------------------------------------------------------------- | ----------------------- | -------- |
| Reader-prose block view                           | Adopt the design and representative fixtures; implement in Rust  | `quire-rs`              | P1       |
| Sentence, paragraph, and heading rules            | Adopt as warning-level typed rules                               | `quire-rs` + ISO module | P1       |
| Acronym and terminology handling                  | Adopt; merge with module lexicon and project glossary            | `quire-rs` + modules    | P1       |
| Wordy phrases, doublets, filler, double negatives | Adopt selectively after corpus precision sampling                | `quire-rs`              | P2       |
| Link text and image alternative text              | Adopt for rendered/public documents                              | `quire-rs` + Quoin      | P2       |
| Document-design checks                            | Adopt as Quoin authoring/review guidance first                   | `quoin`                 | P2       |
| Part 4 evidence sweep and maturity model          | Adapt for Quoin organisational review, not core Quire validation | `quoin`                 | P3       |
| External TypeScript engine                        | Do not embed or invoke as a Quire dependency                     | —                       | —        |
| Fixed ISO 24495 thresholds                        | Do not make global or normative                                  | —                       | —        |
| Legal profile's banned `shall` rule               | Do not apply to ISO requirement artifacts                        | —                       | —        |

## Proposed Quire language profile

The profile should be typed data, not a collection of arbitrary regular expressions.
The existing generic `lint_rules` list can remain for local table and section rules.
Language rules need a separate manifest block because they operate on prose blocks,
headings, links, and vocabulary rather than one section/table locator.

Illustrative shape:

```yaml
language_profile:
  locale: en
  rules:
    sentence_length:
      max_words: 30
      severity: warning
    sentence_average:
      min_sentences: 10
      max_words: 20
      severity: warning
    heading_skip:
      severity: warning
    undefined_acronym:
      severity: warning
  acronyms: [API, CLI, EARS, ISO, NFR]
```

The exact key can change. The important properties are typed rule identifiers,
explicit thresholds, module merge semantics, and severity keys that follow the
existing `off`/`warning`/`error` model.

The engine should expose a narrow API similar to:

```text
language_lint(document, profile, scope) -> Vec<LintFinding>
```

Each finding should carry at least:

```text
rule, severity, line, column?, section?, message, replacement_hint?
```

`replacement_hint` should remain optional and advisory. The first version should not
rewrite prose automatically.

## Scope rules for ISO artifacts

The external engine audits general reader prose. ISO artifacts need more precise
scope:

- audit `Description`, `Statement`, `Stakeholder Need`, `Rationale`, explanatory
  sections, and prose-bearing acceptance-criteria cells;
- audit headings for depth and skips, but preserve artifact identifiers and exact
  section names;
- preserve front matter, code blocks, commands, logs, IDs, verification-method cells,
  and exact protocol or API names;
- treat table cells as optional prose targets rather than blindly excluding every
  table, because acceptance criteria are often authored there;
- use the project glossary and module vocabulary to suppress known technical terms;
- keep `shall` valid for ISO/IEC/IEEE 29148 normative requirements;
- keep legalese rules opt-in and scoped to legal/compliance document types;
- keep English-only rules disabled or explicitly reported for other locales.

This prevents a language rule from contradicting the ISO module's existing
`fr-shall-language` and `nfr-shall-language` advisories.

## Division of responsibility

### `quire-rs`

Implement the reader-prose view, typed rule evaluators, source locations, and
deterministic findings. Compile module vocabulary and rule configuration once in the
registry, as the current grammar and lexicon matchers do.

Do not put plain-language heuristics into the EARS grammar. Requirement grammar asks
whether a normative statement is checkable. Language lint asks whether a reader can
find, understand, and act on it. The findings have different meanings and should
remain independently configurable.

### `quire-cli`

Keep `quire lint <DOC> --module <PATH>` as the per-document entry point. Prose rules
should appear through the same diagnostic stream and JSON format, with warning-level
defaults. A later corpus mode can aggregate the same findings without introducing a
second parser or output schema.

Structural validation must remain independent. A language finding must never make a
document unextractable or alter writeback behavior.

### `spec-artifacts-iso`

Declare the first language profile for FR, NFR, StR, and US. Start with rules that
have clear artifact value: heading skips, sentence-length advisories, undefined
acronyms, and selected wordy phrases. Keep all new rules at `warning` until a corpus
baseline and sampled precision review support promotion.

Use the existing module lexicon and glossary machinery rather than introducing an
unrelated acronym file format. If acronym definitions need richer metadata, add a
typed vocabulary registry rather than a list of exceptions.

### `quoin`

Add language quality to `spec-review` and authoring guidance. The review workflow
should report findings with file, line, rule, and reader impact. It should distinguish
mechanical proxy findings from semantic review and never call zero findings “ISO
compliant.”

Adapt the Part 4 evidence and trend concepts for repository-level review: record the
profile version, configuration hash, corpus denominator, finding totals, and trend
over time. Do not make organisational maturity a hidden side effect of `quire lint`.

## Validation and rollout gates

The local Quire guidance already requires measurement before changing a check. Apply
that discipline here:

1. Port the reader-prose fixtures and add Rust parser parity tests.
2. Run the initial rules against the local specification corpus.
3. Sample flagged documents and classify each finding as rule error or real corpus
   debt.
4. Record the configuration hash, denominator, counts, and precision split.
5. Ship findings as warnings.
6. Unify the corpus or tune the rule for semantic reasons, never merely to reduce
   the count.
7. Promote a rule only after explicit user or module-owner sign-off.

The external project's deterministic output, skipped-entry reporting, and append-only
trend state are useful models. Quire should express them through its existing typed
diagnostics and corpus reports rather than copying the external JSON format wholesale.

## Recommended first slice

The first implementation should be one vertical slice:

1. Add a Rust reader-prose block view with code/front matter/list/quote handling.
2. Add `sentence_length`, `heading_skip`, and `undefined_acronym` typed rules.
3. Add the ISO module's language profile with warning severity and project acronyms.
4. Run it through `quire lint` and JSON diagnostics.
5. Add a Quoin review adapter that groups findings by document and rule.
6. Measure the corpus before adding legalese, complex-word, or automated rewrite rules.

This slice proves the extension boundary and reporting contract. It does not commit
the ecosystem to every heuristic in `iso-24495`.

## Final recommendation

Adopt `iso-24495` as a design and fixture source for a Quire-native language-lint
pack. Adopt its reader-aware parsing boundary, deterministic audit contract,
project vocabulary, explicit limitations, and evidence-based rollout. Keep the
implementation in Rust, the policy in modules, and the workflow/reporting in Quoin.

Do not install the external Bun engine inside Quire, do not treat its thresholds as
ISO requirements, and do not apply its legal-writing rules to ISO requirement prose.
