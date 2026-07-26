use crate::release_set::config::{AppConfigError, AppConfigNameField, AppConfigNameIssue};
use canic_core::bootstrap::compiled::{CanisterRoleNameIssue, validate_canister_role_name};
use canic_core::ids::{ComponentSpecId, ComponentSpecIdParseError};

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

pub(super) fn validate_component_spec_id(component_spec: &str) -> Result<(), AppConfigError> {
    component_spec
        .parse::<ComponentSpecId>()
        .map(|_| ())
        .map_err(|error| AppConfigError::InvalidName {
            field: AppConfigNameField::ComponentSpec,
            issue: match error {
                ComponentSpecIdParseError::Empty => AppConfigNameIssue::Empty,
                ComponentSpecIdParseError::TooLong { max_bytes, .. } => {
                    AppConfigNameIssue::TooLong { max_bytes }
                }
                ComponentSpecIdParseError::InvalidCharacters => {
                    AppConfigNameIssue::InvalidCharacters
                }
            },
            value: component_spec.to_string(),
        })
}

pub(super) fn validate_attach_kind(kind: &str) -> Result<(), AppConfigError> {
    if matches!(kind, "singleton" | "shard" | "replica" | "instance") {
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
