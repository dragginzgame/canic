use crate::{
    ThisError,
    cdk::{
        env::nns::NNS_REGISTRY_CANISTER,
        spec::nns::{GetSubnetForCanisterRequest, GetSubnetForCanisterResponse},
    },
    infra::{ic::nns::NnsInfraError, prelude::*},
};

///
/// NnsRegistryInfraError
///

#[derive(Debug, ThisError)]
pub enum NnsRegistryInfraError {
    /// The registry explicitly rejected the request
    #[error("NNS registry rejected the request: {reason}")]
    Rejected { reason: String },
}

impl From<NnsRegistryInfraError> for InfraError {
    fn from(err: NnsRegistryInfraError) -> Self {
        NnsInfraError::from(err).into()
    }
}

///
/// NnsRegistryInfra
///

pub struct NnsRegistryInfra;

impl NnsRegistryInfra {
    /// Query the NNS registry for the subnet of *this* canister.
    ///
    /// Infrastructure adapter:
    /// - normalizes string errors
    /// - never leaks protocol details
    pub async fn get_subnet_for_canister(pid: Principal) -> Result<Option<Principal>, InfraError> {
        let request = GetSubnetForCanisterRequest { principal: pid };

        let result = Call::unbounded_wait(*NNS_REGISTRY_CANISTER, "get_subnet_for_canister")
            .try_with_arg(request)?
            .execute()
            .await?
            .candid::<GetSubnetForCanisterResponse>()?;

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
