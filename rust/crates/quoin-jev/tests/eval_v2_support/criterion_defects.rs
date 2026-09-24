// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The five criterion-soundness defects, defined once (PLAT-1025, PLAT-1031).
//!
//! Corpus v2's labelling rules for these keys, the two label passes that
//! wrote the R-row truth, the by-construction criterion mutants, and the K1/K2
//! soundness variants all read this one table. Before it existed the corpus
//! header and the variants stated the checks in different words, so the
//! variants were scored against labels written to another definition (PR #624
//! review). The text is the `defect` / `clear` wording of the K variants'
//! `CHECKS`, with `untestable`'s aspiration-verb list widened to the one the
//! labellers were given.

/// One soundness check: its truth key, and when it is and is not present.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CriterionDefect {
    /// The truth key, e.g. `vague_term`.
    pub(crate) key: &'static str,
    /// When the answer is `yes`: the defect is present.
    pub(crate) defect: &'static str,
    /// When the answer is `no`: the criterion is clear of it.
    pub(crate) clear: &'static str,
}

impl CriterionDefect {
    /// The labelling rule as the corpus header states it: the defect text,
    /// then the clear text.
    pub(crate) fn rule(&self) -> String {
        format!("{} {}", self.defect, self.clear)
    }
}

/// The five checks, in `keys::KEYS` order.
pub(crate) const CRITERION_DEFECTS: [CriterionDefect; 5] = [
    CriterionDefect {
        key: "vague_term",
        defect: "Yes: some word or phrase that decides pass or fail has no fixed meaning, so two \
            careful reviewers could disagree about whether the same observed behaviour passes. \
            Examples: appropriate, adequate, reasonable, robust, user-friendly, intuitive, \
            gracefully, properly, clean, as needed, where possible, if feasible, generally. \
            'Correctly' or 'valid' with no stated standard of correct or valid is vague. A speed \
            or size word used without a number (fast, quickly, large, small) is vague here and \
            is also a missing threshold; both checks may say yes.",
        clear: "No: every word that decides pass or fail has one reading. Named values, named \
            states, named status or error identifiers, exact counts, and plain verbs such as \
            'is refused', 'is written', 'exits with status 2' are not vague. A term with a fixed \
            technical meaning (idempotent, UTF-8, sorted ascending) is not vague. A term defined \
            elsewhere in the same requirement, or by a named standard, schema or format, is not \
            vague: 'a valid Quire export' is not vague when a schema defines the export, and 'an \
            RFC 3339 timestamp' is not vague. A vague word that does not decide pass or fail, \
            for example in a clause that only explains the purpose, does not count.",
    },
    CriterionDefect {
        key: "no_measurable_threshold",
        defect: "Yes: it talks about an amount or a level but leaves the number out, for example \
            'responds quickly', 'handles large files', 'most requests succeed', 'uses little \
            memory', 'within a short time', 'retries several times', 'high availability'. Also \
            yes when a number is given without a unit or scope and the unit changes the verdict \
            ('latency under 5').",
        clear: "No, in either of two cases. (1) It states the needed number with its unit or an \
            exact value: 'within 200 ms', 'at most 4096 bytes', 'exactly one row', 'exit status \
            2', 'three attempts'. 'All', 'every', 'each', 'none' and 'at least one' are exact \
            quantities. (2) It asserts no quantity or level at all: a criterion about presence, \
            equality, order, format or refusal ('the field is present', 'the request is \
            refused', 'keys are sorted') needs no threshold, so this check is no.",
    },
    CriterionDefect {
        key: "untestable",
        defect: "Yes: no observation, inspection or analysis could count against it. That covers \
            a pure intention or goal ('the system aims to be secure', 'is designed with \
            extensibility in mind'), a tautology ('the output is valid output'), a claim about \
            motive, intent or unobservable internal state, a claim about all future inputs with \
            no checkable instance, and a criterion that only says a feature exists ('supports \
            export') with no behaviour attached. An aspiration verb (strive to, aim to, try to, \
            intend to, endeavour to, seek to, be designed to) governing an otherwise observable \
            outcome also makes it unfalsifiable: 'the service strives to respond within 200 ms' \
            still holds after a 300 ms response.",
        clear: "No: at least one concrete behaviour, output, file, status or message could be \
            observed that would make it false, even if checking it is laborious or needs a \
            special setup. A vague word alone does not make a criterion unfalsifiable when the \
            rest names an observable outcome; that is the vague-term defect. A missing number \
            alone does not make it unfalsifiable; that is the missing-threshold defect.",
    },
    CriterionDefect {
        key: "compound",
        defect: "Yes: two or more different behaviours, effects or required conditions are \
            joined by 'and', 'or', 'as well as', a list or several sentences, and each could be \
            verified, and fail, on its own: 'the file is written and an audit entry is \
            recorded'; 'the order is cancelled, the card is refunded and the customer is \
            emailed'. Also yes for 'X or Y' where X and Y are different behaviours and the \
            criterion does not say which one is required.",
        clear: "No: it requires one outcome. Parts that together describe one observed response \
            to one event count as one outcome: a status with its error identifier, a message \
            with its fields, one value described by several of its own attributes. Several \
            conditions on the trigger ('when A and B both hold, X happens') do not make it \
            compound. A list of inputs that must each get the same single outcome ('empty, null \
            and blank names are each rejected') is one outcome and is not compound.",
    },
    CriterionDefect {
        key: "missing_trigger",
        defect: "Yes: the response clearly depends on a situation the criterion never names, \
            for example 'an error is returned' (on what?), 'the cache is cleared' (when?), 'the \
            user is notified' (of what, when?), 'it retries three times' (after what?). Also \
            yes when the only trigger is a reference ('then', 'it', 'the event', 'in that case') \
            to something the criterion itself does not state, even if the requirement statement \
            mentions an event.",
        clear: "No, in either of two cases. (1) The criterion names its trigger or condition, \
            with When, While, If or Where, or by naming the input or situation it applies to \
            ('a 5000-byte upload is refused', 'on startup, ...', 'for an empty list, ...', \
            'given a missing file, ...'). A trigger that is named but vague is still named; \
            vagueness is the vague-term defect. (2) It genuinely holds at all times and needs \
            no trigger ('every exported file ends with a newline', 'the binary makes no network \
            connection').",
    },
];

/// The check for `key`, if it is one of the five.
pub(crate) fn criterion_defect(key: &str) -> Option<&'static CriterionDefect> {
    CRITERION_DEFECTS.iter().find(|check| check.key == key)
}
