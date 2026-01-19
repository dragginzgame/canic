use crate::{
    cdk::types::Principal,
    config::schema::CanisterKind,
    dto::{
        canister::CanisterRecordView,
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{
        config::ConfigOps,
        storage::{CanisterRecord, children::CanisterChildrenOps},
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
            .map(|(pid, record)| Self::record_to_view(pid, record))
            .collect();

        paginate_vec(views, page)
    }

    /// Returns the per-parent node child for `role`, if present.
    /// Valid only for kind = Node.
    #[must_use]
    pub fn get_node_child(role: &CanisterRole) -> Option<CanisterRecordView> {
        let kind = match ConfigOps::current_subnet_canister(role) {
            Ok(cfg) => cfg.kind,
            Err(err) => {
                log!(
                    Topic::Topology,
                    Warn,
                    "get_node_child({role}) skipped: config lookup failed ({err})"
                );
                return None;
            }
        };

        if kind != CanisterKind::Node {
            log!(
                Topic::Topology,
                Error,
                "get_node_child({role}) invalid for kind={kind:?}"
            );
            return None;
        }

        Self::records()
            .into_iter()
            .find(|(_, record)| &record.role == role)
            .map(|(pid, record)| Self::record_to_view(pid, record))
    }

    /// Returns all children with the given role.
    /// Intended for worker or shard kinds.
    #[must_use]
    pub fn list_children_by_role(role: &CanisterRole) -> Vec<CanisterRecordView> {
        Self::records()
            .into_iter()
            .filter(|(_, record)| &record.role == role)
            .map(|(pid, record)| Self::record_to_view(pid, record))
            .collect()
    }

    fn records() -> Vec<(Principal, CanisterRecord)> {
        CanisterChildrenOps::records()
    }

    fn record_to_view(pid: Principal, record: CanisterRecord) -> CanisterRecordView {
        CanisterRecordView {
            pid,
            role: record.role,
            parent_pid: record.parent_pid,
            module_hash: record.module_hash,
            created_at: record.created_at,
        }
    }
}
