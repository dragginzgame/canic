#[cfg(test)]
mod tests;

#[must_use]
pub fn find_field<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => map
            .get(field)
            .or_else(|| map.values().find_map(|value| find_field(value, field))),
        serde_json::Value::Array(values) => {
            values.iter().find_map(|value| find_field(value, field))
        }
        _ => None,
    }
}

#[must_use]
pub(crate) fn find_string_field(value: &serde_json::Value, field: &str) -> Option<String> {
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

#[must_use]
pub(crate) fn parse_cycle_balance_response(output: &str) -> Option<u128> {
    serde_json::from_str::<serde_json::Value>(output)
        .ok()
        .and_then(|value| find_field(&value, "Ok").and_then(parse_json_u128))
}

#[must_use]
pub fn parse_json_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(parse_u64_digits))
}

#[must_use]
pub fn parse_json_u128(value: &serde_json::Value) -> Option<u128> {
    value
        .as_u64()
        .map(u128::from)
        .or_else(|| value.as_str().and_then(parse_u128_digits))
}

#[must_use]
pub fn parse_u64_digits(text: &str) -> Option<u64> {
    number_digits(text).parse().ok()
}

#[must_use]
pub fn parse_u128_digits(text: &str) -> Option<u128> {
    number_digits(text).parse().ok()
}

fn number_digits(text: &str) -> String {
    text.chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit() || *ch == '_' || *ch == ',')
        .filter(char::is_ascii_digit)
        .collect()
}
