//! Project model observations at the native fixture endpoint boundary.

use crate::protocol::IndexStatus;
use canic::dto::placement::index::PlacementIndexStatusResponse;

impl From<PlacementIndexStatusResponse> for IndexStatus {
    fn from(value: PlacementIndexStatusResponse) -> Self {
        match value {
            PlacementIndexStatusResponse::Pending {
                owner_pid,
                created_at,
                provisional_pid,
            } => Self::Pending {
                owner_pid: owner_pid.into(),
                created_at,
                provisional_pid: provisional_pid.map(Into::into),
            },
            PlacementIndexStatusResponse::Bound {
                instance_pid,
                bound_at,
            } => Self::Bound {
                instance_pid: instance_pid.into(),
                bound_at,
            },
        }
    }
}
