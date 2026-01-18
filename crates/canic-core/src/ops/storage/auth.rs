use crate::{
    dto::auth::DelegationProof,
    storage::stable::auth::{DelegationState, DelegationStateData},
};

///
/// DelegationStateOps
///

pub struct DelegationStateOps;

impl DelegationStateOps {
    #[must_use]
    pub fn data() -> DelegationStateData {
        DelegationState::export()
    }

    pub fn import(data: DelegationStateData) {
        DelegationState::import(data);
    }

    #[must_use]
    pub fn proof() -> Option<DelegationProof> {
        DelegationState::get_proof()
    }

    pub fn set_proof(proof: DelegationProof) {
        DelegationState::set_proof(proof);
    }

    pub fn clear_proof() {
        DelegationState::clear_proof();
    }
}
