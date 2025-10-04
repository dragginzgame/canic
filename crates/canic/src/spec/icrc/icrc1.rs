use crate::spec::prelude::*;

///
/// Icrc1TransferArgs
///

#[derive(CandidType, Deserialize)]
pub struct Icrc1TransferArgs {
    pub from_subaccount: Option<Subaccount>,
    pub to: Account,
    pub amount: candid::Nat,
    pub fee: Option<candid::Nat>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}
