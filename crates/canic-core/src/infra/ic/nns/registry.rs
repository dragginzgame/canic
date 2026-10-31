//! Module: infra::ic::nns::registry
//!
//! Responsibility: query raw NNS registry topology methods.
//! Does not own: topology policy, subnet storage, or endpoint response mapping.
//! Boundary: ops topology uses this to discover canister subnet placement.

use crate::infra::ic::external_candid::decode_one;
use crate::{
    cdk::types::Principal,
    infra::ic::{IcInfraError, known::NNS_REGISTRY_CANISTER},
    log,
    log::Topic,
};
use ic_call::{Call, WaitPolicy};
use ic_platform::candid::contract::encode_one;
use ic_platform::nns_registry_types::{GetSubnetForCanisterRequest, GetSubnetForCanisterResponse};
use thiserror::Error as ThisError;

///
/// NnsRegistryInfraError
///
/// Raw NNS registry adapter failure.
/// Owned by NNS registry infra and converted into `IcInfraError`.
///

#[derive(Debug, ThisError)]
pub enum NnsRegistryInfraError {
    /// The registry explicitly rejected the request
    #[error("NNS registry rejected the request: {reason}")]
    Rejected { reason: String },
}

///
/// NnsRegistryInfra
///
/// Raw NNS registry adapter.
/// Owned by NNS infra and consumed by ops topology adapters.
///

pub struct NnsRegistryInfra;

impl NnsRegistryInfra {
    /// Quote the same subnet lookup before its owner reserves the paid call.
    pub fn subnet_lookup_call_cost(pid: Principal) -> Result<u128, IcInfraError> {
        Ok(Call::new(
            (*NNS_REGISTRY_CANISTER).into(),
            "get_subnet_for_canister",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(GetSubnetForCanisterRequest {
            principal: Some(pid),
        })?)
        .get_cost())
    }

    /// Query the NNS registry for the subnet of *this* canister.
    ///
    /// Infrastructure adapter:
    /// - normalizes string errors
    /// - never leaks protocol details
    pub async fn get_subnet_for_canister(
        pid: Principal,
    ) -> Result<Option<Principal>, IcInfraError> {
        let request = GetSubnetForCanisterRequest {
            principal: Some(pid),
        };

        let result = decode_one::<GetSubnetForCanisterResponse>(
            &(Call::new(
                (*NNS_REGISTRY_CANISTER).into(),
                "get_subnet_for_canister",
                WaitPolicy::Unbounded,
            )
            .take_raw_args(encode_one(request)?)
            .execute()
            .await?)
                .into_bytes(),
        )?;

        match result {
            Ok(payload) => Ok(payload.subnet_id),
            Err(msg) => {
                log!(
                    Topic::Topology,
                    Warn,
                    "NNS registry rejected get_subnet_for_canister: {msg}"
                );
                Err(NnsRegistryInfraError::Rejected { reason: msg }.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_rejection_preserves_its_typed_infra_cause() {
        let error: IcInfraError = NnsRegistryInfraError::Rejected {
            reason: "rejected".to_string(),
        }
        .into();

        assert!(matches!(
            error,
            IcInfraError::NnsRegistryInfra(
                NnsRegistryInfraError::Rejected { reason }
            ) if reason == "rejected"
        ));
    }
}
