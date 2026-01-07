use crate::{
    PublicError,
    cdk::api::canister_self,
    dto::{canister::CanisterStatusView, icts::CanisterMetadataView},
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
    pub fn metadata() -> CanisterMetadataView {
        CanisterMetadataView {
            name: Self::name(),
            version: Self::version(),
            description: Self::description(),
        }
    }

    /// ICTS standard: return types and string errors are fixed by the spec.
    pub async fn canister_status() -> Result<CanisterStatusView, PublicError> {
        MgmtWorkflow::canister_status_view(canister_self())
            .await
            .map_err(PublicError::from)
    }
}
