use canic_host::response_parse::{find_string_field, parse_candid_text_field};

pub(super) use canic_host::response_parse::parse_cycle_balance_response;

pub(super) fn parse_canic_metadata_version_response(output: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(output)
        .ok()
        .and_then(|value| find_string_field(&value, "canic_version"))
        .or_else(|| parse_candid_text_field(output, "canic_version"))
}

#[cfg(test)]
mod tests;
