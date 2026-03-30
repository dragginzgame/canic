#[must_use]
pub fn to_title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper_next = true;

    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            if upper_next {
                out.push(ch.to_ascii_uppercase());
                upper_next = false;
            } else {
                out.push(ch.to_ascii_lowercase());
            }
        } else {
            if !out.ends_with(' ') && !out.is_empty() {
                out.push(' ');
            }
            upper_next = true;
        }
    }

    out.trim().to_string()
}
