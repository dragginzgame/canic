#[must_use]
pub fn to_snake_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_is_lower_or_digit = false;

    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            let is_upper = ch.is_ascii_uppercase();
            if is_upper && prev_is_lower_or_digit && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_is_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            prev_is_lower_or_digit = false;
        }
    }

    while out.ends_with('_') {
        out.pop();
    }

    out
}

#[cfg(test)]
mod tests {
    use super::to_snake_case;

    #[test]
    fn converts_common_inputs() {
        let cases = [
            ("HelloWorld", "hello_world"),
            ("hello-world", "hello_world"),
            ("hello world", "hello_world"),
            ("helloWorld", "hello_world"),
            ("HELLO_WORLD", "hello_world"),
        ];

        for (input, expected) in cases {
            assert_eq!(to_snake_case(input), expected);
        }
    }
}
