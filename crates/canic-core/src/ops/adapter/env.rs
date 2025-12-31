use crate::{dto::env::EnvView, storage::memory::env::EnvData};

#[must_use]
pub fn env_data_from_view(view: EnvView) -> EnvData {
    EnvData {
        prime_root_pid: view.prime_root_pid,
        subnet_role: view.subnet_role,
        subnet_pid: view.subnet_pid,
        root_pid: view.root_pid,
        canister_role: view.canister_role,
        parent_pid: view.parent_pid,
    }
}

#[must_use]
pub fn env_data_to_view(data: EnvData) -> EnvView {
    EnvView {
        prime_root_pid: data.prime_root_pid,
        subnet_role: data.subnet_role,
        subnet_pid: data.subnet_pid,
        root_pid: data.root_pid,
        canister_role: data.canister_role,
        parent_pid: data.parent_pid,
    }
}
