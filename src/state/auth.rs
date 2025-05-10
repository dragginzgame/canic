use candid::CandidType;
use derive_more::{Display, FromStr};
use serde::{Deserialize, Serialize};

///
/// Permission
///

#[derive(
    CandidType, Debug, Clone, Copy, Display, PartialEq, Eq, FromStr, Hash, Serialize, Deserialize,
)]
pub enum Permission {
    Admin,
    CrudDelete,
    CrudLoad,
    CrudSave,
    Player,
    RoleAdmin,
    Store,
}

///
/// Role
///

#[derive(
    CandidType, Debug, Display, Clone, Copy, PartialEq, Eq, FromStr, Hash, Serialize, Deserialize,
)]
pub enum Role {
    Operator,
    Admin,
    Player,
}

impl Role {
    // parent
    // defines role hierarchy
    #[must_use]
    pub const fn parent(&self) -> Option<Self> {
        match self {
            Self::Admin => Some(Self::Operator),
            Self::Player => Some(Self::Admin),
            Self::Operator => None,
        }
    }

    // permissions
    // defines permissions assigned to each role
    #[must_use]
    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            Self::Operator => vec![Permission::Store, Permission::RoleAdmin],
            Self::Admin => vec![
                Permission::Admin,
                Permission::CrudLoad,
                Permission::CrudSave,
                Permission::CrudDelete,
            ],
            Self::Player => vec![Permission::Player],
        }
    }

    // has_permission
    #[must_use]
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.all_permissions().contains(permission)
    }

    // all_permissions
    // get all permissions including inherited ones
    #[must_use]
    pub fn all_permissions(&self) -> Vec<Permission> {
        let mut perms = self.permissions();
        if let Some(parent) = self.parent() {
            perms.extend(parent.all_permissions());
        }

        perms
    }
}
