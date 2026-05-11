use crate::response_parse::{find_string_field, parse_candid_text_field};

pub(super) use crate::response_parse::parse_cycle_balance_response;

pub(super) fn parse_canic_metadata_version_response(output: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(output)
        .ok()
        .and_then(|value| find_string_field(&value, "canic_version"))
        .or_else(|| parse_candid_text_field(output, "canic_version"))
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
