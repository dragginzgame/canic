use crate::{dto::env::EnvView, ops::runtime::env::EnvSnapshot};

///
/// EnvMapper
///

pub struct EnvMapper;

impl EnvMapper {
    #[must_use]
    pub fn view_to_snapshot(view: EnvView) -> EnvSnapshot {
        EnvSnapshot {
            prime_root_pid: view.prime_root_pid,
            subnet_role: view.subnet_role,
            subnet_pid: view.subnet_pid,
            root_pid: view.root_pid,
            canister_role: view.canister_role,
            parent_pid: view.parent_pid,
        }
    }

    #[must_use]
    pub fn snapshot_to_view(snapshot: EnvSnapshot) -> EnvView {
        EnvView {
            prime_root_pid: snapshot.prime_root_pid,
            subnet_role: snapshot.subnet_role,
            subnet_pid: snapshot.subnet_pid,
            root_pid: snapshot.root_pid,
            canister_role: snapshot.canister_role,
            parent_pid: snapshot.parent_pid,
        }
    }
}
