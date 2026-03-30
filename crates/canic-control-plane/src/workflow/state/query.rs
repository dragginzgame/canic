use crate::{dto::state::SubnetStateResponse, ops::storage::state::subnet::SubnetStateOps};

///
/// SubnetStateQuery
///

pub struct SubnetStateQuery;

impl SubnetStateQuery {
    /// Return the current root-owned subnet publication-store snapshot.
    #[must_use]
    pub fn snapshot() -> SubnetStateResponse {
        SubnetStateOps::snapshot_response()
    }
}
