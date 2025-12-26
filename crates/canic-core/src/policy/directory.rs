use crate::{ids::CanisterRole, ops::config::ConfigOps};

/// Policy: is this role part of the app directory?
pub fn is_app_directory_role(role: &CanisterRole) -> bool {
    let cfg = ConfigOps::get();
    cfg.app_directory.contains(role)
}

/// Policy: is this role part of the current subnet directory?
pub fn is_subnet_directory_role(role: &CanisterRole) -> bool {
    let subnet_cfg = ConfigOps::current_subnet();
    subnet_cfg.subnet_directory.contains(role)
}
