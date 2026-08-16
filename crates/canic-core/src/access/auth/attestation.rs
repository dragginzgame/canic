//! Module: access::auth::attestation
//!
//! Responsibility: decode and verify local-Subnet role attestations for endpoint access.
//! Does not own: proof issuance, root trust configuration, or endpoint dispatch.
//! Boundary: the access DSL calls this before invoking application endpoint code.

use crate::{
    access::AccessError,
    cdk::{
        candid::de::{DecoderConfig, IDLDeserialize},
        types::Principal,
    },
    dto::auth::SignedRoleAttestation,
    workflow::runtime::auth::RuntimeAuthWorkflow,
};
use ic_cdk::api::msg_arg_data;

const ROLE_ATTESTATION_DECODING_QUOTA: usize = 384 * 1024;
const ROLE_ATTESTATION_MAX_TYPE_LEN: usize = 16 * 1024;

pub(super) async fn is_attested_local_subnet(caller: Principal) -> Result<(), AccessError> {
    let attestation = role_attestation_from_args()?;
    if attestation.payload.subject != caller {
        return Err(AccessError::RoleAttestationSubjectMismatch);
    }

    RuntimeAuthWorkflow::verify_local_subnet_role_attestation(&attestation, 0)
        .await
        .map_err(AccessError::Internal)
}

fn role_attestation_from_args() -> Result<SignedRoleAttestation, AccessError> {
    let bytes = msg_arg_data();
    role_attestation_from_ingress_bytes(&bytes)
}

fn role_attestation_from_ingress_bytes(bytes: &[u8]) -> Result<SignedRoleAttestation, AccessError> {
    role_attestation_from_bytes(bytes).map_err(|_| AccessError::RoleAttestationMalformed)
}

fn role_attestation_from_bytes(bytes: &[u8]) -> Result<SignedRoleAttestation, String> {
    let mut config = DecoderConfig::new();
    config
        .set_decoding_quota(ROLE_ATTESTATION_DECODING_QUOTA)
        .set_max_type_len(ROLE_ATTESTATION_MAX_TYPE_LEN)
        .set_full_error_message(false);
    let mut decoder = IDLDeserialize::new_with_config(bytes, &config)
        .map_err(|err| format!("failed to decode ingress arguments: {err}"))?;
    decoder
        .get_value::<SignedRoleAttestation>()
        .map_err(|err| err.to_string())
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::role_attestation_from_ingress_bytes;
    use crate::{
        cdk::candid::{Principal, encode_args},
        dto::auth::{
            IcCanisterSignatureProofV1, RoleAttestation, RoleAttestationRootProof,
            SignedRoleAttestation,
        },
        ids::CanisterRole,
    };

    fn p(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    fn attestation() -> SignedRoleAttestation {
        SignedRoleAttestation {
            payload: RoleAttestation {
                subject: p(1),
                role: CanisterRole::new("project_hub"),
                subnet_id: Some(p(2)),
                audience: p(3),
                issued_at_ns: 10,
                expires_at_ns: 20,
                epoch: 4,
            },
            root_proof: RoleAttestationRootProof::IcCanisterSignatureV1(
                IcCanisterSignatureProofV1 {
                    signature_cbor: vec![4, 5, 6],
                    public_key_der: vec![7, 8, 9],
                },
            ),
        }
    }

    #[test]
    fn local_subnet_attestation_decode_allows_large_trailing_payload() {
        let attestation = attestation();
        let trailing = vec![7_u8; 128 * 1024];
        let bytes = encode_args((attestation.clone(), trailing)).expect("encode guarded call");

        let decoded = role_attestation_from_ingress_bytes(&bytes)
            .expect("only the bounded first attestation argument should be decoded");

        assert_eq!(decoded, attestation);
    }

    #[test]
    fn local_subnet_attestation_decode_rejects_oversized_proof() {
        let mut attestation = attestation();
        let RoleAttestationRootProof::IcCanisterSignatureV1(proof) = &mut attestation.root_proof;
        proof.signature_cbor = vec![0; 512 * 1024];
        let bytes = encode_args((attestation,)).expect("encode oversized guarded call");

        role_attestation_from_ingress_bytes(&bytes)
            .expect_err("decoder quota must reject an oversized attestation proof");
    }
}
