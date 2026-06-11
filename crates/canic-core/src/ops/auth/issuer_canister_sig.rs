#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "issuer-proof token runtime lands after DTO foundation"
    )
)]

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum IssuerPayloadKind {
    DelegatedTokenClaims,
}

pub const fn issuer_sig_seed(kind: IssuerPayloadKind) -> &'static [u8] {
    match kind {
        IssuerPayloadKind::DelegatedTokenClaims => b"canic-issuer-delegated-token",
    }
}

pub const fn issuer_sig_domain(kind: IssuerPayloadKind) -> &'static [u8] {
    match kind {
        IssuerPayloadKind::DelegatedTokenClaims => b"canic-issuer-delegated-token",
    }
}

pub fn issuer_canister_sig_verification_message(
    kind: IssuerPayloadKind,
    payload_hash: [u8; 32],
) -> Vec<u8> {
    let domain = issuer_sig_domain(kind);
    let domain_len =
        u8::try_from(domain.len()).expect("issuer canister signature domain exceeds 255 bytes");

    let mut msg = Vec::with_capacity(1 + domain.len() + payload_hash.len());
    msg.push(domain_len);
    msg.extend_from_slice(domain);
    msg.extend_from_slice(&payload_hash);
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn issuer_canister_sig_verification_message_prefixes_domain_len() {
        let payload_hash = [7; 32];
        let msg = issuer_canister_sig_verification_message(
            IssuerPayloadKind::DelegatedTokenClaims,
            payload_hash,
        );
        let domain = issuer_sig_domain(IssuerPayloadKind::DelegatedTokenClaims);

        assert_eq!(msg[0], domain.len() as u8);
        assert_eq!(&msg[1..1 + domain.len()], domain);
        assert_eq!(&msg[1 + domain.len()..], payload_hash);
    }

    #[test]
    fn issuer_seed_hash_matches_binding_seed_hash_input() {
        let seed = issuer_sig_seed(IssuerPayloadKind::DelegatedTokenClaims);
        let seed_hash: [u8; 32] = Sha256::digest(seed).into();
        let expected: [u8; 32] = Sha256::digest(b"canic-issuer-delegated-token").into();

        assert_eq!(seed_hash, expected);
    }
}
