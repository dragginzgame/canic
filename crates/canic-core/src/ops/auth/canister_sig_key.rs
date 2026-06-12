use crate::cdk::types::Principal;

const CANISTER_SIG_PK_DER_PREFIX_LENGTH: usize = 19;
const CANISTER_SIG_PK_DER_OID: &[u8; 14] =
    b"\x30\x0C\x06\x0A\x2B\x06\x01\x04\x01\x83\xB8\x43\x01\x02";

pub(super) fn parse_canister_sig_public_key_der(
    public_key_der: &[u8],
) -> Result<(Principal, Vec<u8>), String> {
    if public_key_der.len() < CANISTER_SIG_PK_DER_PREFIX_LENGTH + 1 {
        return Err("canister signature public key DER too short".to_string());
    }
    let oid_end = 2 + CANISTER_SIG_PK_DER_OID.len();
    if public_key_der.len() < oid_end || &public_key_der[2..oid_end] != CANISTER_SIG_PK_DER_OID {
        return Err("invalid canister signature public key OID".to_string());
    }

    let raw = &public_key_der[CANISTER_SIG_PK_DER_PREFIX_LENGTH..];
    let canister_id_len = usize::from(raw[0]);
    if raw.len() < 1 + canister_id_len {
        return Err("canister signature public key raw bytes too short".to_string());
    }
    let canister_id_end = 1 + canister_id_len;
    let canister_id = Principal::try_from_slice(&raw[1..canister_id_end])
        .map_err(|err| format!("invalid canister id in canister signature public key: {err}"))?;
    let seed = raw[canister_id_end..].to_vec();
    Ok((canister_id, seed))
}
