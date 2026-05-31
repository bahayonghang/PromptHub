//! Timestamp utilities for the Storage_Engine (Requirement 4.9).
//!
//! Timestamps are stored in SQLite as integer epoch milliseconds in UTC and
//! returned to the Frontend as ISO_8601 strings in the exact millisecond format
//! `YYYY-MM-DDTHH:mm:ss.sssZ` (UTC, `Z` suffix, three-digit milliseconds). These
//! helpers are the single conversion point so every row-to-domain mapping
//! produces a consistent, well-formed timestamp.
//!
//! Some of these helpers are only consumed by later service tasks (and the unit
//! tests below), so the module is allowed to carry currently-unused functions.
#![allow(dead_code)]

use chrono::{DateTime, Utc};

use crate::error::AppError;

/// The `chrono` format string producing `YYYY-MM-DDTHH:mm:ss.sssZ`.
///
/// `%.3f` emits a leading dot followed by exactly three fractional-second
/// digits, and the literal `Z` marks UTC.
const ISO_8601_MILLIS: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

/// Returns the current UNIX time in whole milliseconds (UTC).
pub fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// Formats an epoch-millisecond timestamp (UTC) as an ISO_8601 string of the form
/// `YYYY-MM-DDTHH:mm:ss.sssZ`.
///
/// Values are produced by [`now_millis`] and read back from the database, so they
/// are always within the representable range. As a defensive fallback, a value
/// outside that range formats as the UNIX epoch rather than panicking.
pub fn millis_to_iso8601(millis: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(millis)
        .unwrap_or_else(|| {
            DateTime::<Utc>::from_timestamp_millis(0).expect("UNIX epoch is representable")
        })
        .format(ISO_8601_MILLIS)
        .to_string()
}

/// Parses an ISO_8601 timestamp string into epoch milliseconds (UTC).
///
/// Accepts any RFC 3339 / ISO_8601 string (including the canonical
/// `YYYY-MM-DDTHH:mm:ss.sssZ` form and offset-bearing variants), normalizing the
/// result to UTC milliseconds. Returns a [`AppError::parse`] error when the input
/// is not a valid timestamp.
pub fn iso8601_to_millis(value: &str) -> Result<i64, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
        .map_err(|e| AppError::parse(format!("invalid ISO_8601 timestamp `{value}`: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    #[test]
    fn epoch_formats_to_canonical_iso8601() {
        assert_eq!(millis_to_iso8601(0), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn format_shape_is_well_formed() {
        let iso = millis_to_iso8601(1_700_000_000_123);
        assert!(
            iso.ends_with('Z'),
            "must end with the UTC `Z` marker: {iso}"
        );
        assert!(
            iso.contains('.'),
            "must contain a millisecond separator: {iso}"
        );
        assert_eq!(
            iso.len(),
            24,
            "canonical millisecond form is 24 chars: {iso}"
        );
        // Exactly three fractional digits between '.' and 'Z'.
        let frac = &iso[iso.find('.').unwrap() + 1..iso.len() - 1];
        assert_eq!(frac.len(), 3, "expected 3 millisecond digits, got `{frac}`");
    }

    #[test]
    fn round_trips_a_known_millis_value() {
        let millis = 1_700_000_000_123_i64;
        let iso = millis_to_iso8601(millis);
        let back = iso8601_to_millis(&iso).unwrap();
        assert_eq!(back, millis);
    }

    #[test]
    fn round_trips_now_millis() {
        let millis = now_millis();
        let iso = millis_to_iso8601(millis);
        let back = iso8601_to_millis(&iso).unwrap();
        assert_eq!(back, millis);
    }

    #[test]
    fn round_trips_negative_pre_epoch_value() {
        // Timestamps before 1970 are negative milliseconds; the conversion must
        // remain lossless across the epoch boundary.
        let millis = -1_000_i64; // 1969-12-31T23:59:59.000Z
        let iso = millis_to_iso8601(millis);
        assert_eq!(iso, "1969-12-31T23:59:59.000Z");
        assert_eq!(iso8601_to_millis(&iso).unwrap(), millis);
    }

    #[test]
    fn parse_accepts_offset_form_and_normalizes_to_utc() {
        // 12:00:00 at +02:00 is 10:00:00 UTC.
        let millis = iso8601_to_millis("2024-01-01T12:00:00.000+02:00").unwrap();
        assert_eq!(
            millis,
            iso8601_to_millis("2024-01-01T10:00:00.000Z").unwrap()
        );
    }

    #[test]
    fn parse_rejects_malformed_input() {
        let err = iso8601_to_millis("not-a-timestamp").unwrap_err();
        assert_eq!(err.code, ErrorCode::Parse);
    }
}
