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

        let code = match err {
            ic::IcInfraError::CandidDecode(_) => codes::CODEC_INVALID,
            ic::IcInfraError::Candid(_) => codes::CODEC_FAILED,
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
