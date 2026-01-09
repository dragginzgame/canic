use crate::{
    cdk::types::Principal,
    dto::{
        canister::CanisterRecordView,
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    ops::{
        ic::IcOps,
        runtime::env::EnvOps,
        storage::{
            CanisterRecord, children::CanisterChildrenOps, registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::view::paginate::paginate_vec,
};

///
/// CanisterChildrenQuery
///

pub struct CanisterChildrenQuery;

impl CanisterChildrenQuery {
    pub fn page(page: PageRequest) -> Page<CanisterRecordView> {
        let records = Self::records();

        let views: Vec<CanisterRecordView> = records
            .into_iter()
            .map(|(pid, record)| CanisterRecordView {
                pid,
                role: record.role,
                parent_pid: record.parent_pid,
                module_hash: record.module_hash,
                created_at: record.created_at,
            })
            .collect();

        paginate_vec(views, page)
    }

    #[must_use]
    pub fn find_first_by_role(role: &CanisterRole) -> Option<CanisterRecordView> {
        Self::records()
            .into_iter()
            .find(|(_, record)| &record.role == role)
            .map(|(pid, record)| CanisterRecordView {
                pid,
                role: record.role,
                parent_pid: record.parent_pid,
                module_hash: record.module_hash,
                created_at: record.created_at,
            })
    }

    fn records() -> Vec<(Principal, CanisterRecord)> {
        if EnvOps::is_root() {
            // Root derives children from the registry.
            SubnetRegistryOps::children(IcOps::canister_self())
        } else {
            // Non-root uses cached children from topology cascade.
            CanisterChildrenOps::snapshot().entries
        }
    }
}
