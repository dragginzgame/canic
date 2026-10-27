//! Module: component_initialization
//!
//! Responsibility: encode bounded target-bound application initialization for Root.
//! Does not own: application schemas, identity allocation, or network effects.
//! Boundary: the consumer prepares opaque application bytes after observing the allocated target.

use canic_contracts::dto::{
    component_initialization::RootComponentInitializationRequest as NativeRequest,
    component_registry::{
        MAX_COMPONENT_APPLICATION_INIT_BYTES, RootComponentInitializationRequest,
    },
};
use canic_contracts::protocol::component_initialization as wire;
use ic_platform::principal::ExternalPrincipal as Principal;
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
    Wire(#[from] ic_wire::Error),
}

/// Encode the native Root initialization input without interpreting application data.
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
    Ok(ic_wire::native::encode_request(
        wire::METHOD_ID,
        &NativeRequest::from(request.clone()),
        wire::REQUEST_LIMITS,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_contracts::dto::component_registry::ComponentApplicationInitialization;

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
        let decoded: NativeRequest =
            ic_wire::native::decode_request(wire::METHOD_ID, &bytes, wire::REQUEST_LIMITS).unwrap();
        let decoded = RootComponentInitializationRequest::from(decoded);
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

#[cfg(test)]
mod native_boundary_tests {
    use super::*;

    #[test]
    fn maximum_initializer_fits_and_rejects_wrong_method_and_trailing_bytes() {
        let request = RootComponentInitializationRequest {
            operation_id: [1; 32],
            initialization:
                canic_contracts::dto::component_registry::ComponentApplicationInitialization {
                    target_canister: Principal::from_slice(&[2; 29]),
                    arguments: vec![255; MAX_COMPONENT_APPLICATION_INIT_BYTES],
                },
        };
        let frame = encode_command(&request).unwrap();
        assert!(frame.len() <= wire::REQUEST_LIMITS.bytes);
        let decoded: NativeRequest =
            ic_wire::native::decode_request(wire::METHOD_ID, &frame, wire::REQUEST_LIMITS).unwrap();
        assert_eq!(RootComponentInitializationRequest::from(decoded), request);
        assert!(matches!(
            ic_wire::native::decode_request::<NativeRequest>(
                wire::METHOD_ID + 1,
                &frame,
                wire::REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Identity)
        ));
        let mut trailing = frame;
        trailing.push(0);
        assert!(matches!(
            ic_wire::native::decode_request::<NativeRequest>(
                wire::METHOD_ID,
                &trailing,
                wire::REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Trailing)
        ));
    }
}
