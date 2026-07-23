//! Module: infra::ic::nns::registry
//!
//! Responsibility: query raw NNS registry topology methods.
//! Does not own: topology policy, subnet storage, or endpoint response mapping.
//! Boundary: ops topology uses this to discover canister subnet placement.

use crate::{
    cdk::types::Principal,
    infra::ic::{IcInfraError, call::Call, known::NNS_REGISTRY_CANISTER},
    log,
    log::Topic,
};
use candid::CandidType;
use serde::Deserialize;
use thiserror::Error as ThisError;

///
/// GetSubnetForCanisterRequest
///
/// NNS registry request for canister subnet lookup.
/// Owned by NNS registry infra and sent to the registry canister.
///

#[derive(CandidType, Debug, Deserialize)]
pub struct GetSubnetForCanisterRequest {
    pub principal: Principal,
}

///
/// GetSubnetForCanisterPayload
///
/// Successful NNS registry subnet lookup payload.
/// Owned by NNS registry infra and decoded from registry responses.
///

#[derive(CandidType, Debug, Deserialize)]
pub struct GetSubnetForCanisterPayload {
    pub subnet_id: Option<Principal>,
}

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
    /// Query the NNS registry for the subnet of *this* canister.
    ///
    /// Infrastructure adapter:
    /// - normalizes string errors
    /// - never leaks protocol details
    pub async fn get_subnet_for_canister(
        pid: Principal,
    ) -> Result<Option<Principal>, IcInfraError> {
        let request = GetSubnetForCanisterRequest { principal: pid };

        let result = Call::unbounded_wait(*NNS_REGISTRY_CANISTER, "get_subnet_for_canister")
            .with_arg(request)?
            .execute()
            .await?
            .candid::<Result<GetSubnetForCanisterPayload, String>>()?;

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
