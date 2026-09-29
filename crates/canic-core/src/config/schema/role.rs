//! Module: config::schema::role
//!
//! Responsibility: define fleet and role declaration configuration shapes.
//! Does not own: topology attachment validation, package resolution, or runtime state.
//! Boundary: config schema re-exports these data shapes for validated models.

use crate::{
    ids::{AppId, CanisterRole},
    shared_support::is_ascii_snake_case,
};
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
/// AppRoleRef
///
/// App-scoped role reference derived from config role declarations.
/// Owned by config schema and used by validation diagnostics and topology views.
///

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AppRoleRef {
    pub app: AppId,
    pub role: CanisterRole,
}

impl AppRoleRef {
    #[must_use]
    pub const fn new(app: AppId, role: CanisterRole) -> Self {
        Self { app, role }
    }
}

impl fmt::Display for AppRoleRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.app, self.role)
    }
}

///
/// RoleDeclaration
///
/// Declarative role entry from `canic.toml`.
/// Owned by config schema and validated before topology roles are trusted.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleDeclaration {
    pub kind: RoleDeclarationKind,

    /// Application package path relative to canic.toml; absent for canonical Root.
    pub package: Option<String>,

    /// Enroll every managed instance of this role in Fleet admission convergence.
    #[serde(default)]
    pub fleet_admission: bool,

    /// Optional observation surfaces compiled into this role's endpoints.
    #[serde(default)]
    pub observability: RoleObservabilityConfig,
}

///
/// RoleObservabilityConfig
///
/// Build-time selection of optional observation providers. Operational health,
/// readiness, binding and cycle balance remain available for every managed role.
///

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent build-time provider switches, not mutually exclusive runtime states"
)]
pub struct RoleObservabilityConfig {
    pub diagnostics: bool,
    pub history: bool,
    pub logs: bool,
    pub metrics: bool,
}

impl Default for RoleObservabilityConfig {
    fn default() -> Self {
        Self {
            diagnostics: true,
            history: true,
            logs: true,
            metrics: true,
        }
    }
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
