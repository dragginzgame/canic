use super::snake::to_snake_case;

#[must_use]
pub fn to_constant_case(s: &str) -> String {
    let snake = to_snake_case(s);
    snake
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect()
}
