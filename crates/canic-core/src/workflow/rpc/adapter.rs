use crate::{dto::rpc as dto, ops::rpc::request as ops};

///
/// RpcAdapter
///

pub struct RpcAdapter;

impl RpcAdapter {
    #[must_use]
    pub fn create_canister_parent_from_dto(
        parent: dto::CreateCanisterParent,
    ) -> ops::CreateCanisterParent {
        match parent {
            dto::CreateCanisterParent::Root => ops::CreateCanisterParent::Root,
            dto::CreateCanisterParent::ThisCanister => ops::CreateCanisterParent::ThisCanister,
            dto::CreateCanisterParent::Parent => ops::CreateCanisterParent::Parent,
            dto::CreateCanisterParent::Canister(pid) => ops::CreateCanisterParent::Canister(pid),
            dto::CreateCanisterParent::Directory(role) => {
                ops::CreateCanisterParent::Directory(role)
            }
        }
    }

    #[must_use]
    pub const fn create_canister_response_to_dto(
        res: ops::CreateCanisterResponse,
    ) -> dto::CreateCanisterResponse {
        dto::CreateCanisterResponse {
            new_canister_pid: res.new_canister_pid,
        }
    }

    #[must_use]
    pub const fn upgrade_canister_response_to_dto(
        _res: ops::UpgradeCanisterResponse,
    ) -> dto::UpgradeCanisterResponse {
        dto::UpgradeCanisterResponse {}
    }
}
