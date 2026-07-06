use canic_core::dto::state::BootstrapStatusResponse;
use serde_json::Value;

pub(in crate::install_root) type BootstrapStatusSnapshot = BootstrapStatusResponse;

pub(in crate::install_root) fn parse_bootstrap_status_value(
    data: &Value,
) -> Option<BootstrapStatusSnapshot> {
    serde_json::from_value::<BootstrapStatusResponse>(data.clone())
        .ok()
        .or_else(|| {
            data.get("Ok")
                .cloned()
                .and_then(|ok| serde_json::from_value::<BootstrapStatusResponse>(ok).ok())
        })
}
