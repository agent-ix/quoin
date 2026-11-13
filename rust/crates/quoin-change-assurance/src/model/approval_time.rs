// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The time an `approved` claim reports (FR-063, FR-065).
//!
//! # The writer's report, read as written
//!
//! The time is what Engineering Assurance's `approval-verify` result reported
//! (its FR-041) and the event's writer copied: the claim's own statement of
//! when the approval was made, with the source it names. Nothing here checks it
//! against a clock, a log or the event's timestamp, runs Git, or defaults it:
//! when the claim reports the time missing, the time is missing, and no other
//! instant stands in for it.
//!
//! Exactly one form is carried: `signed_at` with `signed_at_source`, or
//! `signed_at_missing`. The shape of the instant is read strictly
//! (`YYYY-MM-DDTHH:MM:SSZ`); whether it is plausible is not asked.

use quoin_store::JsonValue;

use std::fmt;

use crate::error::{ChangeAssuranceError, FieldFailure, Subject};
use crate::model::json::Fields;

/// The member names of the instant form, in wire order.
pub const KNOWN_MEMBERS: [&str; 2] = ["signed_at", "signed_at_source"];

/// The member name of the missing form.
pub const MISSING_MEMBER: &str = "signed_at_missing";

/// Where the reported instant came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TimeSource {
    /// The annotated tag's tagger date: the writer's own statement.
    ///
    /// The only source Engineering Assurance reports. A transparency-log
    /// source is refused here until an Engineering Assurance amendment
    /// produces one and this reader is amended with it.
    TaggerDate,
}

impl TimeSource {
    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TaggerDate => "tagger_date",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        (value == Self::TaggerDate.as_str()).then_some(Self::TaggerDate)
    }
}

/// Why the claim reports no time.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TimeMissing {
    /// The tag carried no readable tagger date.
    TaggerDateUnreadable,
}

impl TimeMissing {
    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TaggerDateUnreadable => "tagger_date_unreadable",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        (value == Self::TaggerDateUnreadable.as_str()).then_some(Self::TaggerDateUnreadable)
    }
}

/// A UTC instant of the form `YYYY-MM-DDTHH:MM:SSZ` that names a real calendar
/// second. Held as the text the claim gave, so it round-trips unchanged.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SignedAt(String);

impl SignedAt {
    /// Read an instant, refusing any other form and any date or time that does
    /// not exist.
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::Shape`] naming `field` when `value` is not a
    /// real instant of the stated form.
    pub fn parse(value: &str, subject: Subject, field: &str) -> Result<Self, ChangeAssuranceError> {
        if is_utc_instant(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(ChangeAssuranceError::shape(
                subject,
                FieldFailure::malformed(field),
            ))
        }
    }

    /// The instant as given.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SignedAt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The time a claim reports: an instant with its source, or the reason it is
/// missing. Never both, never neither.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimedTime {
    /// A UTC instant of the form `YYYY-MM-DDTHH:MM:SSZ`, as reported.
    Reported {
        /// The instant.
        instant: SignedAt,
        /// Where it came from.
        source: TimeSource,
    },
    /// No time was obtainable.
    Missing(TimeMissing),
}

impl ClaimedTime {
    /// The member names an object carrying this form must have, beyond the
    /// others of its row: the missing form when `signed_at_missing` is present,
    /// the instant form otherwise.
    #[must_use]
    pub fn members_for(fields: Fields<'_>) -> &'static [&'static str] {
        if fields.optional(MISSING_MEMBER).is_some() {
            &[MISSING_MEMBER]
        } else {
            &KNOWN_MEMBERS
        }
    }

    /// Read whichever form the object carries. The caller has already checked
    /// the object's member set with [`Self::members_for`].
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::Shape`] for a malformed instant, source or
    /// reason.
    pub fn read(fields: Fields<'_>, subject: Subject) -> Result<Self, ChangeAssuranceError> {
        let malformed =
            |name: &str| ChangeAssuranceError::shape(subject, FieldFailure::malformed(name));
        if fields.optional(MISSING_MEMBER).is_some() {
            return TimeMissing::parse(fields.text(MISSING_MEMBER)?)
                .map(Self::Missing)
                .ok_or_else(|| malformed(MISSING_MEMBER));
        }
        let instant = SignedAt::parse(fields.text("signed_at")?, subject, "signed_at")?;
        let source = TimeSource::parse(fields.text("signed_at_source")?)
            .ok_or_else(|| malformed("signed_at_source"))?;
        Ok(Self::Reported { instant, source })
    }

    /// The members this form adds to a JSON object, in wire order.
    #[must_use]
    pub fn members(&self) -> Vec<(&'static str, JsonValue)> {
        match self {
            Self::Reported { instant, source } => vec![
                ("signed_at", JsonValue::string(instant.as_str())),
                ("signed_at_source", JsonValue::string(source.as_str())),
            ],
            Self::Missing(reason) => {
                vec![(MISSING_MEMBER, JsonValue::string(reason.as_str()))]
            }
        }
    }
}

/// Whether `text` has the shape `YYYY-MM-DDTHH:MM:SSZ` and names a real second:
/// the day exists in the month (leap years counted), the hour is below 24 and
/// the minute and second below 60, so a leap second is refused. The instant is
/// not judged plausible.
fn is_utc_instant(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    let punctuation = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'Z'),
    ];
    if punctuation
        .iter()
        .any(|(at, expected)| bytes.get(*at) != Some(expected))
    {
        return false;
    }
    let number = |from: usize, to: usize| -> Option<u32> {
        let digits = bytes.get(from..to)?;
        if !digits.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(digits).ok()?.parse().ok()
    };
    let (Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(5, 7),
        number(8, 10),
        number(11, 13),
        number(14, 16),
        number(17, 19),
    ) else {
        return false;
    };
    let Some(year) = number(0, 4) else {
        return false;
    };
    (1..=days_in_month(year, month)).contains(&day) && hour < 24 && minute < 60 && second < 60
}

/// The days in `month` of `year`; zero for a month that does not exist.
const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => 0,
    }
}
