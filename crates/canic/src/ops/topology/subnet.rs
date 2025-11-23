use crate::{memory::topology::SubnetCanisterRegistry, types::Principal};

pub use crate::dto::topology::subnet::CanisterChildrenPage;

///
/// CanisterChildrenOps
///

pub struct CanisterChildrenOps;

impl CanisterChildrenOps {
    /// Return a paginated view of the canister's direct children.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn page(subnet_id: Principal, offset: u64, limit: u64) -> CanisterChildrenPage {
        let all_children = SubnetCanisterRegistry::children(subnet_id);
        let total = all_children.len() as u64;

        let start = offset.min(total) as usize;
        let end = offset.saturating_add(limit).min(total) as usize;
        let children = all_children[start..end].to_vec();

        CanisterChildrenPage {
            total,
            offset,
            limit,
            children,
        }
    }
}
