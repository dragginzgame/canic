use crate::{
    config::schema::CanisterKind,
    dto::{
        canister::CanisterInfo,
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{config::ConfigOps, storage::children::CanisterChildrenOps},
    workflow::view::paginate::paginate_vec,
};

///
/// CanisterChildrenQuery
///

pub struct CanisterChildrenQuery;

impl CanisterChildrenQuery {
    pub fn page(page: PageRequest) -> Page<CanisterInfo> {
        let entries = CanisterChildrenOps::infos();

        paginate_vec(entries, page)
    }

    /// Returns the per-parent node child for `role`, if present.
    /// Valid only for kind = Node.
    #[must_use]
    pub fn get_node_child(role: &CanisterRole) -> Option<CanisterInfo> {
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

        CanisterChildrenOps::infos()
            .into_iter()
            .find(|entry| &entry.role == role)
    }

    /// Returns all children with the given role.
    /// Intended for worker or shard kinds.
    #[must_use]
    pub fn list_children_by_role(role: &CanisterRole) -> Vec<CanisterInfo> {
        CanisterChildrenOps::infos()
            .into_iter()
            .filter(|entry| &entry.role == role)
            .collect()
    }
}
