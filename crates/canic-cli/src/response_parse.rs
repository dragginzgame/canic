pub const RECORD_MARKER: &str = "record {";

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

pub fn find_string_field(value: &serde_json::Value, field: &str) -> Option<String> {
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

pub fn parse_candid_text_field(output: &str, field: &str) -> Option<String> {
    let after_eq = field_value_after_equals(output, field)?;
    let after_quote = after_eq.trim_start().strip_prefix('"')?;
    let (value, _) = after_quote.split_once('"')?;
    Some(value.to_string())
}

pub fn parse_cycle_balance_response(output: &str) -> Option<u128> {
    output
        .split_once('=')
        .map_or(output, |(_, cycles)| cycles)
        .lines()
        .find_map(parse_leading_u128_digits)
}

pub fn parse_json_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(parse_u64_digits))
}

pub fn parse_json_u128(value: &serde_json::Value) -> Option<u128> {
    value
        .as_u64()
        .map(u128::from)
        .or_else(|| value.as_str().and_then(parse_u128_digits))
}

pub fn field_value_after_equals<'a>(text: &'a str, field: &str) -> Option<&'a str> {
    let (_, after_field) = text.split_once(field)?;
    let (_, after_eq) = after_field.split_once('=')?;
    Some(after_eq.trim_start())
}

pub fn text_after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let (_, after_marker) = text.split_once(marker)?;
    Some(after_marker.trim_start())
}

pub fn parse_u64_digits(text: &str) -> Option<u64> {
    number_digits(text).parse().ok()
}

pub fn parse_u128_digits(text: &str) -> Option<u128> {
    number_digits(text).parse().ok()
}

pub fn parse_leading_u128_digits(text: &str) -> Option<u128> {
    leading_number_digits(text).parse().ok()
}

pub fn quoted_strings(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remaining = text;
    while let Some((_, after_open)) = remaining.split_once('"') {
        let Some((value, after_close)) = after_open.split_once('"') else {
            break;
        };
        values.push(value.to_string());
        remaining = after_close;
    }
    values
}

pub fn candid_record_blocks(text: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut index = 0;
    while let Some(relative_start) = text[index..].find(RECORD_MARKER) {
        let start = index + relative_start;
        let mut depth = 1_u32;
        let mut cursor = start + RECORD_MARKER.len();
        let bytes = text.as_bytes();
        while cursor < text.len() {
            match bytes[cursor] {
                b'{' => depth = depth.saturating_add(1),
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let end = cursor + 1;
                        blocks.push(&text[start..end]);
                        index = start + RECORD_MARKER.len();
                        break;
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
        if depth != 0 {
            break;
        }
    }
    blocks
}

fn number_digits(text: &str) -> String {
    text.chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit() || *ch == '_' || *ch == ',')
        .filter(char::is_ascii_digit)
        .collect()
}

fn leading_number_digits(text: &str) -> String {
    text.trim_start_matches(|ch: char| ch == '(' || ch.is_whitespace())
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '_' || *ch == ',')
        .filter(char::is_ascii_digit)
        .collect()
}
