// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Strict parser for the evidence store's canonical UTC timestamp subset.

/// Parsed timestamp components in chronological tuple order.
pub type TimestampKey = (u16, u8, u8, u8, u8, u8, u16);

/// Parse `YYYY-MM-DDTHH:MM:SS[.mmm]Z` without repairing caller input.
///
/// Years are 0001–9999, seconds are 00–59, and a fraction is exactly three
/// ASCII millisecond digits. The tuple compares by instant for every accepted
/// value.
pub fn parse_timestamp(value: &str) -> Option<TimestampKey> {
    let bytes = value.as_bytes();
    if bytes.len() != 20 && bytes.len() != 24 {
        return None;
    }
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || (bytes.len() == 20 && bytes[19] != b'Z')
        || (bytes.len() == 24 && (bytes[19] != b'.' || bytes[23] != b'Z'))
    {
        return None;
    }
    let year = digits(bytes, 0, 4)?;
    let month = u8::try_from(digits(bytes, 5, 2)?).ok()?;
    let day = u8::try_from(digits(bytes, 8, 2)?).ok()?;
    let hour = u8::try_from(digits(bytes, 11, 2)?).ok()?;
    let minute = u8::try_from(digits(bytes, 14, 2)?).ok()?;
    let second = u8::try_from(digits(bytes, 17, 2)?).ok()?;
    let millis = if bytes.len() == 24 {
        digits(bytes, 20, 3)?
    } else {
        0
    };
    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if day == 0 || day > month_days {
        return None;
    }
    Some((year, month, day, hour, minute, second, millis))
}

fn digits(bytes: &[u8], start: usize, len: usize) -> Option<u16> {
    let slice = bytes.get(start..start.checked_add(len)?)?;
    if !slice.iter().all(u8::is_ascii_digit) {
        return None;
    }
    slice.iter().try_fold(0_u16, |value, digit| {
        value.checked_mul(10)?.checked_add(u16::from(digit - b'0'))
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test assertion is the failure report"
)]
mod tests {
    use super::parse_timestamp;

    #[test]
    fn tc_plat_1794_canonical_utc_timestamps_parse_and_order_by_instant() {
        assert!(
            parse_timestamp("2026-11-22T10:00:00Z").unwrap()
                < parse_timestamp("2026-11-22T10:00:00.001Z").unwrap()
        );
        assert!(parse_timestamp("2024-02-29T23:59:59.940Z").is_some());
    }

    #[test]
    fn tc_plat_1794_noncanonical_or_invalid_timestamps_are_refused() {
        for invalid in [
            "",
            "zzzz",
            "2026-11-22T10:00:00+02:00",
            "2026-11-22t10:00:00Z",
            "2026-11-22T10:00:00z",
            "2026-02-29T10:00:00Z",
            "2024-02-30T10:00:00Z",
            "2026-11-22T23:59:60Z",
            "0000-01-01T00:00:00Z",
            "2026-11-22T10:00:00.Z",
            "2026-11-22T10:00:00.94Z",
            "2026-11-22T10:00:00.000000Z",
        ] {
            assert!(parse_timestamp(invalid).is_none(), "accepted {invalid}");
        }
    }
}
