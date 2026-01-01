use crate::{
    cdk::types::Principal,
    dto::pool::{CanisterPoolEntryView, CanisterPoolView},
    ops::storage::pool::PoolOps,
    workflow::pool::adapter::{canister_pool_entry_to_view, canister_pool_to_view},
};

pub fn pool_entry_view(pid: Principal) -> Option<CanisterPoolEntryView> {
    PoolOps::get(pid).map(|entry| canister_pool_entry_to_view(&entry.header, &entry.state))
}

pub fn pool_list_view() -> CanisterPoolView {
    let data = PoolOps::export();
    canister_pool_to_view(data)
}
