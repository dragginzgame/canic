//! Lossless projection of the production environment observation into the audit protocol.

impl From<canic::dto::env::EnvSnapshotResponse> for crate::protocol::Environment {
    fn from(value: canic::dto::env::EnvSnapshotResponse) -> Self {
        Self {
            fleet_subnet_root_pid: value.fleet_subnet_root_pid.map(Into::into),
            component_spec: value.component_spec,
            subnet_pid: value.subnet_pid.map(Into::into),
            root_pid: value.root_pid.map(Into::into),
            canister_role: value.canister_role,
            parent_pid: value.parent_pid.map(Into::into),
        }
    }
}
