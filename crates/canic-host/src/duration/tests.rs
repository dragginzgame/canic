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
