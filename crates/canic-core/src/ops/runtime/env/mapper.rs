use crate::{
    domain::policy::env::ValidatedEnv, dto::env::EnvSnapshotResponse,
    storage::stable::env::EnvRecord,
};

///
/// EnvRecordMapper
///
/// EnvRecord remains the canonical shape; this mapper exists to keep DTO
/// projections explicit and consistent with other record mappers.

pub struct EnvRecordMapper;

impl EnvRecordMapper {
    #[must_use]
    pub fn record_to_view(record: &EnvRecord) -> EnvSnapshotResponse {
        EnvSnapshotResponse {
            prime_root_pid: record.prime_root_pid,
            subnet_role: record.subnet_role.clone(),
            subnet_pid: record.subnet_pid,
            root_pid: record.root_pid,
            canister_role: record.canister_role.clone(),
            parent_pid: record.parent_pid,
        }
    }

    pub fn validated_to_record(validated: ValidatedEnv) -> EnvRecord {
        EnvRecord {
            prime_root_pid: Some(validated.prime_root_pid),
            subnet_role: Some(validated.subnet_role),
            subnet_pid: Some(validated.subnet_pid),
            root_pid: Some(validated.root_pid),
            canister_role: Some(validated.canister_role),
            parent_pid: Some(validated.parent_pid),
        }
    }
}
