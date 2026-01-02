use crate::{
    cdk::api::canister_self,
    dto::canister::{CanisterMetadataView, CanisterStatusView},
    workflow,
};

#[must_use]
pub fn icts_name() -> String {
    env!("CARGO_PKG_NAME").to_string()
}

#[must_use]
pub fn icts_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[must_use]
pub fn icts_description() -> String {
    env!("CARGO_PKG_DESCRIPTION").to_string()
}

#[must_use]
/// ICTS standard: return types are fixed by the spec.
pub fn icts_metadata() -> CanisterMetadataView {
    CanisterMetadataView {
        name: icts_name(),
        version: icts_version(),
        description: icts_description(),
    }
}

/// ICTS standard: return types and string errors are fixed by the spec.
pub async fn icts_canister_status() -> Result<CanisterStatusView, String> {
    workflow::canister::canister_status(canister_self())
        .await
        .map_err(|err| err.to_string())
}
