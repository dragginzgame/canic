use crate::{
    cdk::{api::canister_self, mgmt::CanisterStatusResult},
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
pub fn icts_metadata() -> Vec<(String, String)> {
    vec![
        ("name".to_string(), icts_name()),
        ("version".to_string(), icts_version()),
        ("description".to_string(), icts_description()),
    ]
}

pub async fn icts_canister_status() -> Result<CanisterStatusResult, String> {
    workflow::canister::canister_status(canister_self())
        .await
        .map_err(|err| err.to_string())
}
