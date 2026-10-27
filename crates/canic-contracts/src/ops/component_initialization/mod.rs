//! Lossless transport projection; no application decoding or state mutation.
use crate::dto::{component_initialization as native, component_registry as stored};
impl From<stored::RootComponentInitializationRequest>
    for native::RootComponentInitializationRequest
{
    fn from(value: stored::RootComponentInitializationRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            initialization: value.initialization.into(),
        }
    }
}
impl From<native::RootComponentInitializationRequest>
    for stored::RootComponentInitializationRequest
{
    fn from(value: native::RootComponentInitializationRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            initialization: value.initialization.into(),
        }
    }
}

impl From<stored::ComponentApplicationInitialization>
    for native::ComponentApplicationInitialization
{
    fn from(value: stored::ComponentApplicationInitialization) -> Self {
        Self {
            target_canister: value.target_canister.into(),
            arguments: value.arguments,
        }
    }
}

impl From<native::ComponentApplicationInitialization>
    for stored::ComponentApplicationInitialization
{
    fn from(value: native::ComponentApplicationInitialization) -> Self {
        Self {
            target_canister: value.target_canister.into(),
            arguments: value.arguments,
        }
    }
}
