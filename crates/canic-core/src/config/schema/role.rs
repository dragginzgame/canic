//! Module: config::schema::role
//!
//! Responsibility: define fleet and role declaration configuration shapes.
//! Does not own: topology attachment validation, package resolution, or runtime state.
//! Boundary: config schema re-exports these data shapes for validated models.

use crate::{ids::CanisterRole, shared_support::is_ascii_snake_case};
use serde::{Deserialize, Serialize};
use std::fmt;

///
/// CanisterRoleNameIssue
///
/// Typed reason a canister role cannot cross a configuration or deployment
/// identity boundary.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanisterRoleNameIssue {
    Empty,
    InvalidSnakeCase,
    TooLong { max_bytes: usize },
}

impl fmt::Display for CanisterRoleNameIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("must not be empty"),
            Self::InvalidSnakeCase => formatter.write_str(
                "must use lowercase snake_case beginning with an ASCII letter, with nonempty lowercase alphanumeric words separated by single '_' characters",
            ),
            Self::TooLong { max_bytes } => {
                write!(formatter, "must not exceed {max_bytes} bytes")
            }
        }
    }
}

/// Validate one canister role at configuration and deployment identity boundaries.
pub const fn validate_canister_role_name(role: &str) -> Result<(), CanisterRoleNameIssue> {
    let bytes = role.as_bytes();
    if bytes.is_empty() {
        return Err(CanisterRoleNameIssue::Empty);
    }
    if bytes.len() > super::NAME_MAX_BYTES {
        return Err(CanisterRoleNameIssue::TooLong {
            max_bytes: super::NAME_MAX_BYTES,
        });
    }
    if !is_ascii_snake_case(role) {
        return Err(CanisterRoleNameIssue::InvalidSnakeCase);
    }

    Ok(())
}

///
/// FleetRoleRef
///
/// Fleet-scoped role reference derived from config role declarations.
/// Owned by config schema and used by validation diagnostics and topology views.
///

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FleetRoleRef {
    pub fleet: String,
    pub role: CanisterRole,
}

impl FleetRoleRef {
    #[must_use]
    pub fn new(fleet: impl Into<String>, role: CanisterRole) -> Self {
        Self {
            fleet: fleet.into(),
            role,
        }
    }
}

impl fmt::Display for FleetRoleRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.fleet, self.role)
    }
}

///
/// RoleDeclaration
///
/// Declarative package-backed role entry from `canic.toml`.
/// Owned by config schema and validated before topology roles are trusted.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleDeclaration {
    pub kind: RoleDeclarationKind,

    /// Package path relative to the declaring canic.toml.
    pub package: String,
}

///
/// RoleDeclarationKind
///
/// Role declaration class used to distinguish root from regular canister roles.
/// Owned by config schema and consumed by topology validation.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleDeclarationKind {
    Root,
    Canister,
}
