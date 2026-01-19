use crate::{
    cdk::api::canister_self,
    dto::{canister::CanisterStatusResponse, error::Error, icts::CanisterMetadataResponse},
    workflow::ic::mgmt::MgmtWorkflow,
};

///
/// IctsApi
///

pub struct IctsApi;

impl IctsApi {
    #[must_use]
    pub fn name() -> String {
        env!("CARGO_PKG_NAME").to_string()
    }

    #[must_use]
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[must_use]
    pub fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").to_string()
    }

    #[must_use]
    pub fn metadata() -> CanisterMetadataResponse {
        CanisterMetadataResponse {
            name: Self::name(),
            version: Self::version(),
            description: Self::description(),
        }
    }

    /// ICTS standard: return types and string errors are fixed by the spec.
    pub async fn canister_status() -> Result<CanisterStatusResponse, Error> {
        MgmtWorkflow::canister_status(canister_self())
            .await
            .map_err(Error::from)
    }
}
