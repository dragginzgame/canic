///
/// CYCLES
///

pub const KC: u128 = 1_000;
pub const MC: u128 = 1_000_000;
pub const BC: u128 = 1_000_000_000;
pub const TC: u128 = 1_000_000_000_000;
pub const QC: u128 = 1_000_000_000_000_000;

// format_cycles
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn format_cycles(cycles: u128) -> String {
    format!("{:.3} TC", cycles as f64 / TC as f64)
}

// parse_cycles
// helper function to parse string with multiplier suffix
pub fn parse_cycles(value: &str) -> Result<u128, String> {
    let mut num_str = String::new();
    let mut suffix_str = String::new();
    let mut seen_dot = false;

    for c in value.chars() {
        if c.is_ascii_digit() || (c == '.' && !seen_dot) {
            if c == '.' {
                seen_dot = true;
            }
            num_str.push(c);
        } else {
            suffix_str.push(c);
        }
    }

    let number: f64 = num_str.parse::<f64>().map_err(|e| e.to_string())?;

    let multiplier = match suffix_str.as_str() {
        "K" => 1_000_f64,
        "M" => 1_000_000_f64,
        "B" => 1_000_000_000_f64,
        "T" => 1_000_000_000_000_f64,
        "Q" => 1_000_000_000_000_000_f64,
        _ => 1_f64,
    };

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    Ok((number * multiplier) as u128)
}
