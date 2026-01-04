use crate::{
    PublicError,
    cdk::api::canister_self,
    dto::canister::{CanisterMetadataView, CanisterStatusView},
    ops::ic::mgmt::MgmtOps,
};

///
/// ICTS API
///

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
pub fn icts_metadata() -> CanisterMetadataView {
    CanisterMetadataView {
        name: icts_name(),
        version: icts_version(),
        description: icts_description(),
    }
}

/// ICTS standard: return types and string errors are fixed by the spec.
pub async fn icts_canister_status() -> Result<CanisterStatusView, PublicError> {
    MgmtOps::canister_status(canister_self())
        .await
        .map_err(PublicError::from)
}
