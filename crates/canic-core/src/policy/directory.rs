use crate::{ids::CanisterRole, ops::config::ConfigOps};

pub fn is_app_directory_role(role: &CanisterRole) -> bool {
    let cfg = ConfigOps::get();
    cfg.app_directory.contains(role)
}

pub fn is_subnet_directory_role(role: &CanisterRole) -> bool {
    let subnet_cfg = ConfigOps::current_subnet();
    subnet_cfg.subnet_directory.contains(role)
}
