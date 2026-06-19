//! Module: config::schema::role
//!
//! Responsibility: define fleet and role declaration configuration shapes.
//! Does not own: topology attachment validation, package resolution, or runtime state.
//! Boundary: config schema re-exports these data shapes for validated models.

use crate::ids::CanisterRole;
use serde::{Deserialize, Serialize};
use std::fmt;

///
/// FleetRoleRefV1
///
/// Fleet-scoped role reference derived from config role declarations.
/// Owned by config schema and used by validation diagnostics and topology views.
///

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FleetRoleRefV1 {
    pub fleet: String,
    pub role: CanisterRole,
}

impl FleetRoleRefV1 {
    #[must_use]
    pub fn new(fleet: impl Into<String>, role: CanisterRole) -> Self {
        Self {
            fleet: fleet.into(),
            role,
        }
    }
}

impl fmt::Display for FleetRoleRefV1 {
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
