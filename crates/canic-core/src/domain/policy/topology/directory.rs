use crate::{ids::CanisterRole, ops::config::ConfigOps};

/// Policy: is this role part of the app directory?
#[must_use]
pub fn is_app_directory_role(role: &CanisterRole) -> bool {
    let Ok(cfg) = ConfigOps::get() else {
        return false;
    };

    cfg.app_directory.contains(role)
}

/// Policy: is this role part of the current subnet directory?
#[must_use]
pub fn is_subnet_directory_role(role: &CanisterRole) -> bool {
    let Ok(subnet_cfg) = ConfigOps::current_subnet() else {
        return false;
    };

    subnet_cfg.subnet_directory.contains(role)
}
