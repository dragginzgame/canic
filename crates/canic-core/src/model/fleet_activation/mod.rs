//! Module: model::fleet_activation
//!
//! Responsibility: validate the immutable identity established by fresh Fleet activation.
//! Does not own: Candid decoding, stable-record conversion, storage access, or lifecycle traps.
//! Boundary: workflows supply the embedded build identity before ops persists `Prepared`.

pub mod endpoint_mode;

use crate::ids::{FleetBinding, ReleaseBuildId};
use thiserror::Error as ThisError;

///
/// PreparedFleetActivation
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedFleetActivation {
    pub identity: FleetActivationIdentity,
}

///
/// FleetActivationIdentity
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationIdentity {
    pub fleet: FleetBinding,
    pub operation_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
}

///
/// RootInstallIdentity
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootInstallIdentity {
    pub fleet: FleetBinding,
    pub install_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
}

///
/// NonrootInstallIdentity
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NonrootInstallIdentity {
    pub fleet: FleetBinding,
    pub install_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
}

///
/// PrepareFleetActivationError
///

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum PrepareFleetActivationError {
    #[error(
        "install release-build identity {supplied} does not match embedded Wasm identity {embedded}"
    )]
    ReleaseBuildMismatch {
        supplied: ReleaseBuildId,
        embedded: ReleaseBuildId,
    },
}

/// Validate and normalize fresh root input into the sole internal activation identity.
pub fn prepare_root_install(
    input: RootInstallIdentity,
    embedded_release_build_id: ReleaseBuildId,
) -> Result<PreparedFleetActivation, PrepareFleetActivationError> {
    require_release_build_match(input.release_build_id, embedded_release_build_id)?;

    Ok(PreparedFleetActivation {
        identity: FleetActivationIdentity {
            fleet: input.fleet,
            operation_id: input.install_id,
            release_build_id: input.release_build_id,
        },
    })
}

/// Validate and normalize fresh non-root input into the sole internal activation identity.
pub fn prepare_nonroot_install(
    input: NonrootInstallIdentity,
    embedded_release_build_id: ReleaseBuildId,
) -> Result<PreparedFleetActivation, PrepareFleetActivationError> {
    require_release_build_match(input.release_build_id, embedded_release_build_id)?;

    Ok(PreparedFleetActivation {
        identity: FleetActivationIdentity {
            fleet: input.fleet,
            operation_id: input.install_id,
            release_build_id: input.release_build_id,
        },
    })
}

fn require_release_build_match(
    supplied: ReleaseBuildId,
    embedded: ReleaseBuildId,
) -> Result<(), PrepareFleetActivationError> {
    if supplied != embedded {
        return Err(PrepareFleetActivationError::ReleaseBuildMismatch { supplied, embedded });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{
        AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildNonce,
    };

    fn release_build(byte: u8) -> ReleaseBuildId {
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([byte; 32]))
    }

    fn input(release_build_id: ReleaseBuildId) -> RootInstallIdentity {
        RootInstallIdentity {
            fleet: FleetBinding {
                fleet: FleetKey {
                    network: CanonicalNetworkId::public_ic(),
                    fleet_id: FleetId::from_generated_bytes([2; 32]),
                },
                app: AppId::from("toko"),
            },
            install_id: [3; 32],
            release_build_id,
        }
    }

    fn nonroot_input(release_build_id: ReleaseBuildId) -> NonrootInstallIdentity {
        let root = input(release_build_id);
        NonrootInstallIdentity {
            fleet: root.fleet,
            install_id: root.install_id,
            release_build_id: root.release_build_id,
        }
    }

    #[test]
    fn root_install_normalizes_install_identity_only_after_build_match() {
        let release_build_id = release_build(5);
        let prepared =
            prepare_root_install(input(release_build_id), release_build_id).expect("prepare");

        assert_eq!(prepared.identity.operation_id, [3; 32]);
        assert_eq!(prepared.identity.release_build_id, release_build_id);
    }

    #[test]
    fn root_install_rejects_release_build_mismatch() {
        let supplied = release_build(6);
        let embedded = release_build(7);

        assert_eq!(
            prepare_root_install(input(supplied), embedded),
            Err(PrepareFleetActivationError::ReleaseBuildMismatch { supplied, embedded })
        );
    }

    #[test]
    fn nonroot_install_normalizes_the_same_exact_identity() {
        let release_build_id = release_build(8);
        let prepared = prepare_nonroot_install(nonroot_input(release_build_id), release_build_id)
            .expect("prepare");

        assert_eq!(prepared.identity.operation_id, [3; 32]);
        assert_eq!(prepared.identity.release_build_id, release_build_id);
    }

    #[test]
    fn nonroot_install_rejects_release_build_mismatch() {
        let supplied = release_build(9);
        let embedded = release_build(10);

        assert_eq!(
            prepare_nonroot_install(nonroot_input(supplied), embedded),
            Err(PrepareFleetActivationError::ReleaseBuildMismatch { supplied, embedded })
        );
    }
}
