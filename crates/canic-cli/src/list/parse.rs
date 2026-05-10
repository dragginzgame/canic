pub(super) fn parse_canic_metadata_version_response(output: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(output)
        .ok()
        .and_then(|value| find_string_field(&value, "canic_version"))
        .or_else(|| parse_candid_text_field(output, "canic_version"))
}

fn find_string_field(value: &serde_json::Value, field: &str) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => map
            .get(field)
            .and_then(|value| value.as_str().map(ToString::to_string))
            .or_else(|| {
                map.values()
                    .find_map(|value| find_string_field(value, field))
            }),
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| find_string_field(value, field)),
        _ => None,
    }
}

fn parse_candid_text_field(output: &str, field: &str) -> Option<String> {
    let (_, after_field) = output.split_once(field)?;
    let (_, after_eq) = after_field.split_once('=')?;
    let after_quote = after_eq.trim_start().strip_prefix('"')?;
    let (value, _) = after_quote.split_once('"')?;
    Some(value.to_string())
}

pub(super) fn parse_cycle_balance_response(output: &str) -> Option<u128> {
    output
        .split_once('=')
        .map_or(output, |(_, cycles)| cycles)
        .lines()
        .find_map(parse_leading_integer)
}

fn parse_leading_integer(line: &str) -> Option<u128> {
    let digits = line
        .trim_start_matches(|ch: char| ch == '(' || ch.is_whitespace())
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '_' || *ch == ',')
        .filter(char::is_ascii_digit)
        .collect::<String>();
    (!digits.is_empty())
        .then_some(digits)
        .and_then(|digits| digits.parse::<u128>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_host::format::cycles_tc;

    // Ensure cycle balances parse from canic_cycle_balance command output.
    #[test]
    fn parses_cycle_balance_from_endpoint_output() {
        assert_eq!(
            parse_cycle_balance_response("(variant { 17_724 = 4_487_280_757_485 : nat })"),
            Some(4_487_280_757_485)
        );
        assert_eq!(
            parse_cycle_balance_response("(variant { 17_725 = record { code = 1 : nat } })"),
            None
        );
        assert_eq!(cycles_tc(12_345_678_900_000), "12.35 TC");
    }

    // Ensure metadata responses provide the Canic framework version for list output.
    #[test]
    fn parses_canic_version_from_metadata_output() {
        assert_eq!(
            parse_canic_metadata_version_response(
                r#"{"package_name":"app","canic_version":"0.33.6"}"#
            ),
            Some("0.33.6".to_string())
        );
        assert_eq!(
            parse_canic_metadata_version_response(
                r#"[{"package_name":"app","canic_version":"0.33.7"}]"#
            ),
            Some("0.33.7".to_string())
        );
        assert_eq!(
            parse_canic_metadata_version_response(
                r#"(record { package_name = "app"; canic_version = "0.33.8" })"#
            ),
            Some("0.33.8".to_string())
        );
        assert_eq!(parse_canic_metadata_version_response("{}"), None);
    }
}
