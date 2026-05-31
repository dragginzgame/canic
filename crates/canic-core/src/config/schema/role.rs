use crate::ids::CanisterRole;
use serde::{Deserialize, Serialize};
use std::fmt;

///
/// FleetRoleRefV1
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
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleDeclarationKind {
    Root,
    Canister,
}
