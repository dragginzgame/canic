use crate::{
    Error, ThisError,
    cdk::{
        env::nns::NNS_REGISTRY_CANISTER,
        spec::nns::{GetSubnetForCanisterRequest, GetSubnetForCanisterResponse},
        types::Principal,
    },
    infra::ic::nns::NnsInfraError,
    log,
    log::Topic,
    ops::ic::call::Call,
};

///
/// NnsRegistryInfraError
///

#[derive(Debug, ThisError)]
pub enum NnsRegistryInfraError {
    /// The response could not be decoded as expected
    #[error("failed to decode NNS registry response")]
    DecodeFailed,

    /// The registry explicitly rejected the request
    #[error("NNS registry rejected the request")]
    Rejected,
}

impl From<NnsRegistryInfraError> for Error {
    fn from(err: NnsRegistryInfraError) -> Self {
        NnsInfraError::from(err).into()
    }
}

///
/// Query the NNS registry for the subnet of *this* canister.
///
/// Infrastructure adapter:
/// - wraps legacy NNS API
/// - normalizes string errors
/// - never leaks protocol details
///

pub(crate) async fn get_subnet_for_canister(
    pid: Principal,
) -> Result<Option<Principal>, NnsRegistryInfraError> {
    let request = GetSubnetForCanisterRequest::new(pid);

    let result = Call::unbounded_wait(*NNS_REGISTRY_CANISTER, "get_subnet_for_canister")
        .with_arg(request)
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
            Err(NnsRegistryInfraError::Rejected)
        }
    }
}
