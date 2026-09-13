// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reading a timestamp, in the two grammars the store actually accepts.
//!
//! Two readers were written independently during the port (quoin#456) —
//! `trust::is_instant` and `assurance_records::is_iso_instant` — and they read
//! two DIFFERENT languages, each faithful to the TypeScript check it replaced:
//!
//! - **[`InstantGrammar::DateOrInstant`]** replaced `!Number.isNaN(Date.parse(v))`
//!   on a trust decision's `decidedAt`. It accepts a bare `YYYY-MM-DD` date,
//!   and it accepts `YYYY-MM-DDTHH:MM:SS` followed by anything at all: the
//!   retained check handed the string to V8's date parser, which takes a zone
//!   suffix, a fractional part, or nothing, and this reader is deliberately no
//!   narrower than the records already on disk.
//! - **[`InstantGrammar::ZonedInstant`]** replaced an anchored regular
//!   expression on an assurance record's `recordedAt` and window bounds:
//!   `\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})`. A zone
//!   is mandatory, a bare date is refused, and nothing may follow.
//!
//! Collapsing them into one grammar would be a behaviour change in a validator
//! — a record that reads today would stop reading, or one that is refused today
//! would be admitted — so both languages survive here BYTE FOR BYTE. What is
//! unified is the reader: one calendar-date prefix, one tail per grammar, and
//! one place to look when a timestamp question comes up (quoin#458).
//!
//! The grammar is a parameter rather than two functions because it is a
//! property of the FIELD being read, and a caller that picks the wrong one
//! should have to name the one it picked.

/// Which timestamp language a field is written in.
///
/// Named at every call site: `decidedAt` and `recordedAt` are both "a
/// timestamp" and they are not the same thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstantGrammar {
    /// A calendar date, or a date and a wall-clock time with any suffix.
    DateOrInstant,
    /// A full instant with a mandatory zone offset and nothing after it.
    ZonedInstant,
}

impl InstantGrammar {
    /// Every grammar, so a caller enumerating them cannot miss one.
    pub const ALL: &'static [Self] = &[Self::DateOrInstant, Self::ZonedInstant];

    /// The grammar's stable code, as a diagnostic or a schema would spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DateOrInstant => "date-or-instant",
            Self::ZonedInstant => "zoned-instant",
        }
    }

    /// Read a grammar back from its code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "date-or-instant" => Some(Self::DateOrInstant),
            "zoned-instant" => Some(Self::ZonedInstant),
            _ => None,
        }
    }

    /// How a refusal names what was expected.
    #[must_use]
    pub const fn expectation(self) -> &'static str {
        match self {
            Self::DateOrInstant => "must be an ISO-8601 timestamp",
            Self::ZonedInstant => "must be an ISO-8601 instant",
        }
    }
}

/// A timestamp that has been read, and the grammar it was read in.
///
/// A newtype rather than a `bool` return: a caller holding an `Instant` cannot
/// have skipped the check, and the two callers that go on to store the text
/// take it from here rather than from the unvalidated input they still have in
/// scope.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    text: String,
    grammar: InstantGrammar,
}

impl Instant {
    /// Read `value` in `grammar`, or report that it is not in that language.
    #[must_use]
    pub fn parse(value: &str, grammar: InstantGrammar) -> Option<Self> {
        accepts(value, grammar).then(|| Self {
            text: value.to_owned(),
            grammar,
        })
    }

    /// The timestamp exactly as it was written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The grammar it was read in.
    #[must_use]
    pub const fn grammar(&self) -> InstantGrammar {
        self.grammar
    }
}

impl std::fmt::Display for Instant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.text)
    }
}

impl From<Instant> for String {
    fn from(value: Instant) -> Self {
        value.text
    }
}

/// The shared calendar-date prefix, then the grammar's own tail.
fn accepts(value: &str, grammar: InstantGrammar) -> bool {
    let bytes = value.as_bytes();
    let digits_at = |index: usize, count: usize| {
        (index..index + count).all(|position| bytes.get(position).is_some_and(u8::is_ascii_digit))
    };
    let byte_at = |index: usize, byte: u8| bytes.get(index) == Some(&byte);

    // `YYYY-MM-DD`, which both grammars open with. Out-of-range indices read as
    // absent rather than panicking, so a short input simply fails here.
    if !(digits_at(0, 4)
        && byte_at(4, b'-')
        && digits_at(5, 2)
        && byte_at(7, b'-')
        && digits_at(8, 2))
    {
        return false;
    }

    match grammar {
        InstantGrammar::DateOrInstant => {
            if bytes.len() == 10 {
                return true;
            }
            // `THH:MM:SS`, and then whatever the producer appended. The
            // retained V8 parse accepted a zone, a fractional part or neither,
            // and narrowing that here would refuse records already on disk.
            byte_at(10, b'T')
                && digits_at(11, 2)
                && byte_at(13, b':')
                && digits_at(14, 2)
                && byte_at(16, b':')
                && digits_at(17, 2)
        }
        InstantGrammar::ZonedInstant => {
            if !(byte_at(10, b'T')
                && digits_at(11, 2)
                && byte_at(13, b':')
                && digits_at(14, 2)
                && byte_at(16, b':')
                && digits_at(17, 2))
            {
                return false;
            }
            let mut index = 19;
            // `(\.\d+)?` — at least one digit if the dot is there at all.
            if byte_at(index, b'.') {
                index += 1;
                let start = index;
                while digits_at(index, 1) {
                    index += 1;
                }
                if index == start {
                    return false;
                }
            }
            // `(Z|[+-]\d{2}:\d{2})`, anchored: the zone ends the string.
            if byte_at(index, b'Z') {
                return index + 1 == bytes.len();
            }
            if byte_at(index, b'+') || byte_at(index, b'-') {
                return digits_at(index + 1, 2)
                    && byte_at(index + 3, b':')
                    && digits_at(index + 4, 2)
                    && index + 6 == bytes.len();
            }
            false
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{Instant, InstantGrammar};

    /// Every grammar survives the round trip through its own code.
    ///
    /// Provenance: quoin#458
    #[test]
    fn tc_458_300_every_grammar_round_trips_through_its_code() {
        assert_eq!(
            InstantGrammar::ALL.len(),
            2,
            "a grammar was added without a case here"
        );
        for grammar in InstantGrammar::ALL {
            assert_eq!(
                InstantGrammar::from_code(grammar.as_str()),
                Some(*grammar),
                "{} did not round trip",
                grammar.as_str()
            );
        }
        assert_eq!(InstantGrammar::from_code("iso8601"), None);
    }

    /// The date-or-instant language, exactly as `trust::is_instant` read it.
    ///
    /// Trace: FR-093-AC-1
    /// Provenance: quoin#456, quoin#458
    #[test]
    fn tc_458_301_the_date_or_instant_grammar_takes_a_bare_date_and_any_suffix() {
        for accepted in [
            "2026-01-01",
            "2026-01-01T00:00:00",
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00.123456Z",
            "2026-01-01T00:00:00+05:30",
            // The retained `Date.parse` accepted trailing text on a
            // well-formed prefix, so this reader must too.
            "2026-01-01T00:00:00 (Pacific)",
        ] {
            assert!(
                Instant::parse(accepted, InstantGrammar::DateOrInstant).is_some(),
                "refused {accepted}"
            );
        }
        for refused in [
            "",
            "2026-01",
            "2026-01-0",
            "2026-01-01 00:00:00",
            "2026-01-01T00:00",
            "20260101T000000Z",
            "yesterday",
        ] {
            assert!(
                Instant::parse(refused, InstantGrammar::DateOrInstant).is_none(),
                "accepted {refused}"
            );
        }
    }

    /// The zoned language, exactly as `assurance_records::is_iso_instant` read it.
    ///
    /// Trace: FR-048-AC-3
    /// Provenance: quoin#456, quoin#458
    #[test]
    fn tc_458_302_the_zoned_instant_grammar_requires_a_zone_and_ends_at_it() {
        for accepted in [
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00.1Z",
            "2026-01-01T00:00:00.123456789Z",
            "2026-01-01T00:00:00+05:30",
            "2026-01-01T00:00:00-08:00",
            "2026-01-01T00:00:00.5-08:00",
        ] {
            assert!(
                Instant::parse(accepted, InstantGrammar::ZonedInstant).is_some(),
                "refused {accepted}"
            );
        }
        for refused in [
            "2026-01-01",
            "2026-01-01T00:00:00",
            "2026-01-01T00:00:00.Z",
            "2026-01-01T00:00:00Z ",
            "2026-01-01T00:00:00+0530",
            "2026-01-01T00:00:00+05:300",
            "2026-01-01T00:00:00 (Pacific)",
        ] {
            assert!(
                Instant::parse(refused, InstantGrammar::ZonedInstant).is_none(),
                "accepted {refused}"
            );
        }
    }

    /// The two languages are different, and the difference is the point.
    ///
    /// Anti-tautology: if a future edit collapsed them, this fails rather than
    /// the two tests above both continuing to pass against one grammar.
    ///
    /// Trace: FR-048-AC-3, FR-093-AC-1
    /// Provenance: quoin#458
    #[test]
    fn tc_458_303_the_two_grammars_disagree_in_both_directions() {
        let date_only = "2026-01-01";
        assert!(Instant::parse(date_only, InstantGrammar::DateOrInstant).is_some());
        assert!(Instant::parse(date_only, InstantGrammar::ZonedInstant).is_none());

        let unzoned = "2026-01-01T12:30:45";
        assert!(Instant::parse(unzoned, InstantGrammar::DateOrInstant).is_some());
        assert!(Instant::parse(unzoned, InstantGrammar::ZonedInstant).is_none());
    }

    /// What was read is what was written, and it remembers which language.
    ///
    /// Trace: FR-048-AC-3
    /// Provenance: quoin#458
    #[test]
    fn tc_458_304_a_read_instant_carries_its_own_text_and_grammar() {
        let read = Instant::parse("2026-01-01T00:00:00Z", InstantGrammar::ZonedInstant).unwrap();
        assert_eq!(read.as_str(), "2026-01-01T00:00:00Z");
        assert_eq!(read.grammar(), InstantGrammar::ZonedInstant);
        assert_eq!(String::from(read), "2026-01-01T00:00:00Z");
    }
}
