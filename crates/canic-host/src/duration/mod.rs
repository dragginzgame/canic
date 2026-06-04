use thiserror::Error as ThisError;

///
/// DurationParseError
///
#[derive(Debug, ThisError)]
pub enum DurationParseError {
    #[error("invalid duration {value:?}; use positive seconds or a value ending in s, m, h, or d")]
    Invalid { value: String },
}

pub fn parse_duration_seconds(value: &str) -> Result<u64, DurationParseError> {
    let (number, multiplier) = match value.as_bytes().last().copied() {
        Some(b's') => (&value[..value.len() - 1], 1),
        Some(b'm') => (&value[..value.len() - 1], 60),
        Some(b'h') => (&value[..value.len() - 1], 60 * 60),
        Some(b'd') => (&value[..value.len() - 1], 24 * 60 * 60),
        Some(b'0'..=b'9') => (value, 1),
        _ => return invalid_duration(value),
    };
    number
        .parse::<u64>()
        .ok()
        .and_then(|amount| amount.checked_mul(multiplier))
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| DurationParseError::Invalid {
            value: value.to_string(),
        })
}

fn invalid_duration(value: &str) -> Result<u64, DurationParseError> {
    Err(DurationParseError::Invalid {
        value: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_parser_accepts_units() {
        assert_eq!(parse_duration_seconds("7d").expect("days"), 604_800);
        assert_eq!(parse_duration_seconds("2h").expect("hours"), 7_200);
        assert_eq!(parse_duration_seconds("30m").expect("minutes"), 1_800);
        assert_eq!(parse_duration_seconds("90s").expect("seconds"), 90);
        assert_eq!(parse_duration_seconds("42").expect("bare"), 42);
    }

    #[test]
    fn duration_parser_rejects_zero_and_unknown_units() {
        assert!(matches!(
            parse_duration_seconds("0d"),
            Err(DurationParseError::Invalid { .. })
        ));
        assert!(matches!(
            parse_duration_seconds("1w"),
            Err(DurationParseError::Invalid { .. })
        ));
    }
}
