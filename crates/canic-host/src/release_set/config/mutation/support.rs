use crate::release_set::config::{AppConfigError, AppConfigNameField, AppConfigNameIssue};
use canic_core::bootstrap::compiled::{CanisterRoleNameIssue, validate_canister_role_name};

pub(super) fn admit_canister_role_name(role: &str) -> Result<(), AppConfigError> {
    validate_canister_role_name(role).map_err(|issue| AppConfigError::InvalidName {
        field: AppConfigNameField::Role,
        issue: map_canister_role_name_issue(issue),
        value: role.to_string(),
    })
}

const fn map_canister_role_name_issue(issue: CanisterRoleNameIssue) -> AppConfigNameIssue {
    match issue {
        CanisterRoleNameIssue::Empty => AppConfigNameIssue::Empty,
        CanisterRoleNameIssue::InvalidSnakeCase => AppConfigNameIssue::InvalidSnakeCase,
        CanisterRoleNameIssue::TooLong { max_bytes } => AppConfigNameIssue::TooLong { max_bytes },
    }
}

pub(super) fn validate_subnet_name(subnet: &str) -> Result<(), AppConfigError> {
    if subnet.is_empty() {
        return Err(AppConfigError::InvalidName {
            field: AppConfigNameField::Subnet,
            issue: AppConfigNameIssue::Empty,
            value: subnet.to_string(),
        });
    }
    if !subnet
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(AppConfigError::InvalidName {
            field: AppConfigNameField::Subnet,
            issue: AppConfigNameIssue::InvalidCharacters,
            value: subnet.to_string(),
        });
    }
    Ok(())
}

pub(super) fn validate_attach_kind(kind: &str) -> Result<(), AppConfigError> {
    if matches!(
        kind,
        "service" | "singleton" | "shard" | "replica" | "instance"
    ) {
        return Ok(());
    }

    Err(AppConfigError::InvalidKind {
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
