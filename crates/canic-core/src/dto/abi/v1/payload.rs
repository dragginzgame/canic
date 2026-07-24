use crate::dto::{
    env::EnvBootstrapArgs,
    prelude::*,
    topology::{FleetDirectoryInput, SubnetDirectoryInput},
};
use crate::ids::{FleetBinding, ReleaseBuildId};

//
// CanisterInitPayload
//

#[derive(CandidType, Debug, Deserialize)]
pub struct CanisterInitPayload {
    pub fleet: FleetBinding,
    pub install_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
    pub env: EnvBootstrapArgs,
    pub fleet_directory: FleetDirectoryInput,
    pub subnet_directory: SubnetDirectoryInput,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::topology::IndexEntryInput,
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, FleetId, FleetKey, ReleaseBuildNonce,
            SubnetSlotId,
        },
    };

    #[test]
    fn managed_nonroot_init_payload_roundtrips_the_exact_fleet_identity_and_directories() {
        let fleet = FleetBinding {
            fleet: FleetKey {
                network: CanonicalNetworkId::public_ic(),
                fleet_id: FleetId::from_generated_bytes([1; 32]),
            },
            app: AppId::from("toko"),
        };
        let release_build_id =
            ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([2; 32]));
        let principal = Principal::from_slice(&[3; 29]);
        let payload = CanisterInitPayload {
            fleet: fleet.clone(),
            install_id: [4; 32],
            release_build_id,
            env: EnvBootstrapArgs {
                prime_root_pid: Some(principal),
                subnet_role: Some(SubnetSlotId::DEFAULT),
                subnet_pid: Some(principal),
                root_pid: Some(principal),
                canister_role: Some(CanisterRole::new("app")),
                parent_pid: Some(principal),
            },
            fleet_directory: FleetDirectoryInput(vec![IndexEntryInput {
                role: CanisterRole::new("app"),
                pid: principal,
            }]),
            subnet_directory: SubnetDirectoryInput(Vec::new()),
        };

        let bytes = candid::encode_one(&payload).expect("encode managed non-root init payload");
        let decoded: CanisterInitPayload =
            candid::decode_one(&bytes).expect("decode managed non-root init payload");

        assert_eq!(decoded.fleet, fleet);
        assert_eq!(decoded.install_id, [4; 32]);
        assert_eq!(decoded.release_build_id, release_build_id);
        assert_eq!(decoded.fleet_directory.0.len(), 1);
        assert!(decoded.subnet_directory.0.is_empty());
    }
}
