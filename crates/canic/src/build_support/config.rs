use std::{fmt::Write as _, fs, path::Path};

/// Read a Canic config source, or generate a minimal standalone config when allowed.
#[must_use]
pub fn read_config_source_or_default(
    config_path: &Path,
    explicit_config: bool,
    default_role: Option<&str>,
) -> (String, bool) {
    match fs::read_to_string(config_path) {
        Ok(source) => (source, false),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let role = default_role
                .unwrap_or_else(|| panic!("Missing Canic config at {}", config_path.display()));

            assert!(
                !explicit_config,
                "Missing explicit Canic config at {}",
                config_path.display()
            );

            (standalone_config_source(role), true)
        }
        Err(err) => panic!("Failed to read {}: {err}", config_path.display()),
    }
}

/// Render the minimal topology needed by a standalone non-root canister.
#[must_use]
pub fn standalone_config_source(role: &str) -> String {
    assert!(
        !role.is_empty() && role != "root",
        "standalone Canic config requires a non-root role"
    );

    let role_key = toml_basic_string(role);

    format!(
        r#"controllers = []
app_index = []

[app]
init_mode = "enabled"

[app.whitelist]

[subnets.prime]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.{role_key}]
kind = "singleton"
"#
    )
}

// Escape a role name as a TOML basic string for quoted table keys.
fn toml_basic_string(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len() + 2);
    rendered.push('"');

    for ch in value.chars() {
        match ch {
            '"' => rendered.push_str("\\\""),
            '\\' => rendered.push_str("\\\\"),
            '\u{08}' => rendered.push_str("\\b"),
            '\t' => rendered.push_str("\\t"),
            '\n' => rendered.push_str("\\n"),
            '\u{0c}' => rendered.push_str("\\f"),
            '\r' => rendered.push_str("\\r"),
            ch if ch.is_control() => {
                let _ = write!(rendered, "\\u{:04X}", ch as u32);
            }
            ch => rendered.push(ch),
        }
    }

    rendered.push('"');
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::bootstrap::parse_config_model;

    #[test]
    fn standalone_config_source_parses_for_plain_role() {
        let source = standalone_config_source("sandbox_minimal");
        let cfg = parse_config_model(&source).expect("generated standalone config parses");

        let prime = cfg.subnets.get("prime").expect("prime subnet exists");

        assert!(prime.canisters.contains_key("root"));
        assert!(prime.canisters.contains_key("sandbox_minimal"));
    }

    #[test]
    fn standalone_config_source_quotes_role_keys() {
        let source = standalone_config_source("demo.role");
        let cfg = parse_config_model(&source).expect("generated standalone config parses");

        let prime = cfg.subnets.get("prime").expect("prime subnet exists");

        assert!(prime.canisters.contains_key("demo.role"));
    }

    #[test]
    #[should_panic(expected = "standalone Canic config requires a non-root role")]
    fn standalone_config_source_rejects_root_role() {
        let _ = standalone_config_source("root");
    }

    #[test]
    fn read_config_source_or_default_generates_when_implicit_file_is_missing() {
        let missing_path =
            std::env::temp_dir().join(format!("canic-missing-default-{}.toml", std::process::id()));
        let (source, generated) =
            read_config_source_or_default(missing_path.as_path(), false, Some("test"));

        assert!(generated);
        assert!(source.contains("[subnets.prime.canisters.\"test\"]"));
    }
}
