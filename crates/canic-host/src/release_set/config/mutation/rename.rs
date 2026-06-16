use super::{
    RenamedFleetRoleSource,
    support::{toml_assignment_key, toml_string_literal, validate_role_name},
};
use crate::release_set::config::model::RenamedFleetRole;
use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};
use std::{fs, path::Path};
use toml::Value as TomlValue;

pub(in crate::release_set) fn rename_fleet_role_source(
    config_source: &str,
    config_path: &Path,
    expected_fleet: &str,
    old_role: &str,
    new_role: &str,
) -> Result<RenamedFleetRoleSource, Box<dyn std::error::Error>> {
    let old_role = old_role.trim();
    let new_role = new_role.trim();
    validate_role_name(old_role)?;
    validate_role_name(new_role)?;
    if old_role == "root" || new_role == "root" {
        return Err("root role cannot be renamed through fleet role rename".into());
    }
    if old_role == new_role {
        return Err("old role and new role must differ".into());
    }

    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let actual_fleet = config
        .fleet_name()
        .ok_or_else(|| "missing required [fleet].name in canic.toml".to_string())?;
    if actual_fleet != expected_fleet {
        return Err(format!(
            "selected config declares fleet {actual_fleet:?}, not {expected_fleet:?}"
        )
        .into());
    }

    let old_id = CanisterRole::owned(old_role.to_string());
    let new_id = CanisterRole::owned(new_role.to_string());
    let declaration = config
        .roles
        .get(&old_id)
        .ok_or_else(|| format!("role {expected_fleet}.{old_role} is not declared"))?;
    if config.declares_role(&new_id) {
        return Err(format!("role {expected_fleet}.{new_role} is already declared").into());
    }

    let source = rename_config_role_references(config_source, old_role, new_role)?;
    parse_config_model(&source).map_err(|err| err.to_string())?;

    let (package_manifest, package_source, package_manifest_note) =
        config_path.parent().map_or_else(
            || (None, None, Some("config path has no parent".to_string())),
            |parent| {
                let manifest = parent.join(&declaration.package).join("Cargo.toml");
                match update_package_manifest_role(&manifest, expected_fleet, old_role, new_role) {
                    Ok(Some(updated)) => (Some(manifest), Some(updated), None),
                    Ok(None) => (
                        None,
                        None,
                        Some(format!(
                            "{} did not contain matching [package.metadata.canic] fleet/role metadata",
                            manifest.display()
                        )),
                    ),
                    Err(err) => (None, None, Some(err.to_string())),
                }
            },
        );

    Ok(RenamedFleetRoleSource {
        source,
        package_manifest: package_manifest.clone(),
        package_source,
        role: RenamedFleetRole {
            fleet: expected_fleet.to_string(),
            old_role: old_role.to_string(),
            new_role: new_role.to_string(),
            old_display: format!("{expected_fleet}.{old_role}"),
            new_display: format!("{expected_fleet}.{new_role}"),
            package_manifest,
            package_manifest_note,
        },
    })
}

fn rename_config_role_references(
    source: &str,
    old_role: &str,
    new_role: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let old_literal = toml_string_literal(old_role);
    let new_literal = toml_string_literal(new_role);
    let mut updated = Vec::new();

    for line in source.lines() {
        let mut line = rename_role_header(line, old_role, new_role)?;
        let trimmed = line.trim_start();
        if toml_assignment_key(trimmed) == Some("canister_role")
            || toml_assignment_key(trimmed) == Some("app_index")
        {
            line = line.replace(&old_literal, &new_literal);
        }
        updated.push(line);
    }

    let mut result = updated.join("\n");
    if source.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

fn rename_role_header(
    line: &str,
    old_role: &str,
    new_role: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') || trimmed.starts_with("[[") {
        return Ok(line.to_string());
    }

    let Some(prefix_len) = line.find('[') else {
        return Ok(line.to_string());
    };
    let inner = &trimmed[1..trimmed.len() - 1];
    let mut path = parse_toml_dotted_path(inner)?;
    let rename_roles_header = path.len() == 2 && path[0] == "roles" && path[1] == old_role;
    let rename_canister_header =
        path.len() >= 4 && path[0] == "subnets" && path[2] == "canisters" && path[3] == old_role;

    if rename_roles_header {
        path[1] = new_role.to_string();
    } else if rename_canister_header {
        path[3] = new_role.to_string();
    } else {
        return Ok(line.to_string());
    }

    Ok(format!(
        "{}[{}]",
        &line[..prefix_len],
        path.iter()
            .map(|part| toml_string_literal(part))
            .collect::<Vec<_>>()
            .join(".")
    ))
}

fn parse_toml_dotted_path(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars();
    let mut in_quote = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if !in_quote => in_quote = true,
            '"' if in_quote => in_quote = false,
            '\\' if in_quote => {
                let Some(escaped) = chars.next() else {
                    return Err("unterminated TOML escape in table header".into());
                };
                current.push(escaped);
            }
            '.' if !in_quote => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            ch => current.push(ch),
        }
    }

    if in_quote {
        return Err("unterminated quoted TOML table header".into());
    }
    parts.push(current.trim().to_string());
    Ok(parts)
}

fn update_package_manifest_role(
    manifest: &Path,
    expected_fleet: &str,
    old_role: &str,
    new_role: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if !manifest.is_file() {
        return Ok(None);
    }

    let source = fs::read_to_string(manifest)?;
    let metadata = toml::from_str::<TomlValue>(&source)?;
    let Some(canic_metadata) = metadata
        .get("package")
        .and_then(TomlValue::as_table)
        .and_then(|package| package.get("metadata"))
        .and_then(TomlValue::as_table)
        .and_then(|metadata| metadata.get("canic"))
        .and_then(TomlValue::as_table)
    else {
        return Ok(None);
    };
    if canic_metadata.get("fleet").and_then(TomlValue::as_str) != Some(expected_fleet)
        || canic_metadata.get("role").and_then(TomlValue::as_str) != Some(old_role)
    {
        return Ok(None);
    }

    Ok(Some(rename_package_metadata_role_source(
        &source, old_role, new_role,
    )))
}

fn rename_package_metadata_role_source(source: &str, old_role: &str, new_role: &str) -> String {
    let mut in_canic_metadata = false;
    let old_literal = toml_string_literal(old_role);
    let new_literal = toml_string_literal(new_role);
    let mut lines = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_canic_metadata = trimmed == "[package.metadata.canic]";
        }
        if in_canic_metadata && toml_assignment_key(line.trim_start()) == Some("role") {
            lines.push(line.replace(&old_literal, &new_literal));
        } else {
            lines.push(line.to_string());
        }
    }

    let mut result = lines.join("\n");
    if source.ends_with('\n') {
        result.push('\n');
    }
    result
}
