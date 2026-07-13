use crate::release_set::config::{FleetConfigError, FleetConfigNameField, FleetConfigNameIssue};

pub(super) fn validate_role_name(role: &str) -> Result<(), FleetConfigError> {
    if role.is_empty() {
        return Err(FleetConfigError::InvalidName {
            field: FleetConfigNameField::Role,
            issue: FleetConfigNameIssue::Empty,
            value: role.to_string(),
        });
    }
    if !role
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(FleetConfigError::InvalidName {
            field: FleetConfigNameField::Role,
            issue: FleetConfigNameIssue::InvalidCharacters,
            value: role.to_string(),
        });
    }
    Ok(())
}

pub(super) fn validate_subnet_name(subnet: &str) -> Result<(), FleetConfigError> {
    if subnet.is_empty() {
        return Err(FleetConfigError::InvalidName {
            field: FleetConfigNameField::Subnet,
            issue: FleetConfigNameIssue::Empty,
            value: subnet.to_string(),
        });
    }
    if !subnet
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(FleetConfigError::InvalidName {
            field: FleetConfigNameField::Subnet,
            issue: FleetConfigNameIssue::InvalidCharacters,
            value: subnet.to_string(),
        });
    }
    Ok(())
}

pub(super) fn validate_attach_kind(kind: &str) -> Result<(), FleetConfigError> {
    if matches!(
        kind,
        "service" | "singleton" | "shard" | "replica" | "instance"
    ) {
        return Ok(());
    }

    Err(FleetConfigError::InvalidKind {
        kind: kind.to_string(),
    })
}

pub(super) fn toml_assignment_key(line: &str) -> Option<&str> {
    let (key, _) = line.split_once('=')?;
    Some(key.trim())
}

pub(super) fn toml_string_literal(value: &str) -> String {
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
