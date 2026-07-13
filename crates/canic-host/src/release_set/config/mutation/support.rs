use crate::release_set::config::{FleetConfigError, FleetConfigNameField, FleetConfigNameIssue};
use canic_core::bootstrap::compiled::{CanisterRoleNameIssue, validate_canister_role_name};

pub(super) fn admit_canister_role_name(role: &str) -> Result<(), FleetConfigError> {
    validate_canister_role_name(role).map_err(|issue| FleetConfigError::InvalidName {
        field: FleetConfigNameField::Role,
        issue: map_canister_role_name_issue(issue),
        value: role.to_string(),
    })
}

const fn map_canister_role_name_issue(issue: CanisterRoleNameIssue) -> FleetConfigNameIssue {
    match issue {
        CanisterRoleNameIssue::Empty => FleetConfigNameIssue::Empty,
        CanisterRoleNameIssue::InvalidSnakeCase => FleetConfigNameIssue::InvalidSnakeCase,
        CanisterRoleNameIssue::TooLong { max_bytes } => FleetConfigNameIssue::TooLong { max_bytes },
    }
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
