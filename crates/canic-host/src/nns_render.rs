pub fn compact_text(value: &str, chars: usize) -> String {
    value.chars().take(chars).collect()
}

pub fn text_or_dash(value: Option<&str>) -> &str {
    value.filter(|text| !text.is_empty()).unwrap_or("-")
}

pub fn optional_f32_text(value: Option<f32>) -> String {
    value.map_or_else(|| "-".to_string(), |value| value.to_string())
}

pub fn optional_node_count_text(value: Option<u32>) -> String {
    value.map_or_else(|| "unknown".to_string(), |count| count.to_string())
}

pub const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
