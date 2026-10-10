//! Module: component_initialization
//!
//! Responsibility: encode bounded target-bound application initialization for Root.
//! Does not own: application schemas, identity allocation, or network effects.
//! Boundary: the consumer prepares inner Candid after observing the allocated target.

use candid::Principal;
use canic_contracts::dto::{
    component_registry::{
        MAX_COMPONENT_APPLICATION_INIT_BYTES, RootComponentInitializationRequest,
    },
    wire::projection::component_initialization::RootCommand,
};
use thiserror::Error;

/// Typed preparation refusal before any Root command is sent.

#[derive(Debug, Error)]
pub enum ComponentInitializationError {
    #[error("initialization requires a nonzero operation and an allocated target")]
    Identity,
    #[error(
        "application initializer must contain 1..={MAX_COMPONENT_APPLICATION_INIT_BYTES} bytes"
    )]
    Bounds,
    #[error(transparent)]
    Candid(#[from] candid::Error),
}

/// Encode the production `canic_root_command` input without interpreting application data.
/// Retain these bytes for an exact retry; Root freezes the request before installation.
pub fn encode_command(
    request: &RootComponentInitializationRequest,
) -> Result<Vec<u8>, ComponentInitializationError> {
    if request.operation_id == [0; 32]
        || [Principal::anonymous(), Principal::management_canister()]
            .contains(&request.initialization.target_canister)
    {
        return Err(ComponentInitializationError::Identity);
    }
    let bytes = &request.initialization.arguments;
    if bytes.is_empty() || bytes.len() > MAX_COMPONENT_APPLICATION_INIT_BYTES {
        return Err(ComponentInitializationError::Bounds);
    }
    Ok(candid::encode_one(
        RootCommand::BindComponentInitialization(request),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_contracts::dto::component_registry::ComponentApplicationInitialization;

    use canic_contracts::dto::wire::root_command::RootCommand;

    fn request() -> RootComponentInitializationRequest {
        RootComponentInitializationRequest {
            operation_id: [1; 32],
            initialization: ComponentApplicationInitialization {
                target_canister: Principal::from_slice(&[2; 29]),
                arguments: vec![0, 1, 255],
            },
        }
    }

    #[test]
    fn command_preserves_exact_target_and_application_bytes() {
        let request = request();
        let bytes = encode_command(&request).unwrap();
        let RootCommand::BindComponentInitialization(decoded) = candid::decode_one(&bytes).unwrap()
        else {
            panic!("initialization command selector");
        };
        assert_eq!(decoded, request);
        assert_eq!(encode_command(&request).unwrap(), bytes);
    }

    #[test]
    fn command_refuses_unallocated_identities_and_unbounded_bytes() {
        let mut request = request();
        request.initialization.target_canister = Principal::anonymous();
        assert!(matches!(
            encode_command(&request),
            Err(ComponentInitializationError::Identity)
        ));
        request = self::request();
        request.initialization.arguments = vec![0; MAX_COMPONENT_APPLICATION_INIT_BYTES + 1];
        assert!(matches!(
            encode_command(&request),
            Err(ComponentInitializationError::Bounds)
        ));
    }
}
