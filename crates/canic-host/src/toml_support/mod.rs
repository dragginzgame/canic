//! Shared TOML field lookup and literal rendering for host-owned configuration tools.

use std::path::Path;
use toml::Value as TomlValue;

/// Read a required Cargo manifest string, retaining the field and source path in diagnostics.
pub fn manifest_string(
    manifest: &TomlValue,
    path: &[&str],
    manifest_path: &Path,
) -> Result<String, String> {
    let mut value = manifest;
    for segment in path {
        value = value
            .get(*segment)
            .ok_or_else(|| format!("missing {} in {}", path.join("."), manifest_path.display()))?;
    }
    value.as_str().map(ToString::to_string).ok_or_else(|| {
        format!(
            "{} must be a string in {}",
            path.join("."),
            manifest_path.display()
        )
    })
}

/// Escape a basic TOML string with the maintained configuration renderer's spelling.
#[must_use]
pub fn toml_string_literal(value: &str) -> String {
    let mut escaped = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}
