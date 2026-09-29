//! Module: infra
//!
//! Responsibility: expose low-level platform adapters and infra-scoped failures.
//! Does not own: workflow orchestration, policy decisions, or storage mutation.
//! Boundary: ops calls infra for mechanical platform effects and raw transport.

pub mod ic;

use crate::InternalError;

impl From<ic::IcInfraError> for InternalError {
    fn from(err: ic::IcInfraError) -> Self {
        use crate::diagnostics::codes;

        if let ic::IcInfraError::CallFailed(ic_cdk::call::CallFailed::CallRejected(rejection)) =
            &err
        {
            return Self::public(codes::PLATFORM_UNAVAILABLE)
                .with_platform_rejection(&rejection.to_string());
        }
        let code = match err {
            ic::IcInfraError::CandidDecode(_) => codes::CODEC_INVALID,
            ic::IcInfraError::Candid(_) => codes::CODEC_FAILED,
            ic::IcInfraError::CallFailed(
                ic_cdk::call::CallFailed::InsufficientLiquidCycleBalance(_),
            ) => codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES,
            ic::IcInfraError::CallFailed(_) => codes::PLATFORM_UNAVAILABLE,
            ic::IcInfraError::EmbeddedReleaseBuild(_)
            | ic::IcInfraError::CyclesLedgerInfra(_)
            | ic::IcInfraError::IcpRefillInfra(_)
            | ic::IcInfraError::MgmtInfra(_)
            | ic::IcInfraError::NnsRegistryInfra(_) => codes::PLATFORM_FAILED,
        };
        Self::public(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{diagnostics::codes, dto::error::Error, ops::OpsError};
    use ic_cdk::call::{CallFailed, CallPerformFailed, InsufficientLiquidCycleBalance};

    #[test]
    fn liquid_cycle_admission_failure_preserves_its_typed_public_cause() {
        let failure = CallFailed::InsufficientLiquidCycleBalance(InsufficientLiquidCycleBalance {
            available: 50,
            required: 100,
        });
        let error = InternalError::from(OpsError::from(ic::IcInfraError::from(failure)));
        assert_eq!(
            error.public_code(),
            Some(codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES)
        );
        let public = Error::from(error);
        let decoded: Error = candid::decode_one(&candid::encode_one(public).unwrap()).unwrap();
        assert_eq!(
            decoded.code(),
            codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES.raw_code()
        );
    }

    #[test]
    fn platform_rejection_retains_bounded_utf8_without_changing_public_error() {
        let message = "Wasm module defined 50001 functions";
        let rejection = ic_cdk::call::CallRejected::with_rejection(5, message.to_owned());
        let expected = rejection.to_string();
        let error = InternalError::from(OpsError::from(ic::IcInfraError::from(
            CallFailed::CallRejected(rejection),
        )));
        assert_eq!(error.public_code(), Some(codes::PLATFORM_UNAVAILABLE));
        assert_eq!(error.platform_rejection(), Some(expected.as_str()));
        let public = Error::from(error);
        assert_eq!(
            public,
            Error::from(InternalError::public(codes::PLATFORM_UNAVAILABLE))
        );

        let message = "€".repeat(InternalError::MAX_PLATFORM_REJECTION_BYTES);
        let bounded = InternalError::unavailable().with_platform_rejection(&message);
        let retained = bounded.platform_rejection().unwrap();
        assert!(retained.len() <= InternalError::MAX_PLATFORM_REJECTION_BYTES);
        assert!(retained.len() > InternalError::MAX_PLATFORM_REJECTION_BYTES - 3);
        assert!(message.starts_with(retained));
    }

    #[test]
    fn unspecified_call_failure_is_not_attributed_to_funding() {
        let failure = CallFailed::CallPerformFailed(CallPerformFailed);
        let error = InternalError::from(ic::IcInfraError::from(failure));
        assert_eq!(error.public_code(), Some(codes::PLATFORM_UNAVAILABLE));
    }
}
