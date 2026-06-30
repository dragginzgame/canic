use canic_core::{
    cdk::{candid, types::Principal},
    dto::auth::{
        ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessStepV1,
        ChainKeyBatchWitnessV1, ChainKeyDelegationCertV1, ChainKeyKeyId, ChainKeyRootSignatureV1,
        DelegatedRoleGrant, DelegatedToken, DelegatedTokenClaims, DelegationAudience,
        DelegationCert, DelegationProof, IcCanisterSignatureProofV1,
        IcChainKeyBatchSignatureProofV1, IssuerProof, IssuerProofAlgorithm, IssuerProofBinding,
        RootProof,
    },
    ids::CanisterRole,
};
use criterion::Criterion;
use std::hint::black_box;

#[cfg(all(
    feature = "auth-root-canister-sig-create",
    feature = "auth-root-canister-sig-verify"
))]
const ROOT_SIG_DOMAIN: &[u8] = b"canic-root-delegation-cert";

fn bench_delegated_token_serialization(criterion: &mut Criterion) {
    let token = sample_delegated_token();
    let encoded = candid::encode_one(&token).expect("sample delegated token must encode");

    criterion.bench_function("delegated_token_candid_encode", |bench| {
        bench.iter(|| candid::encode_one(black_box(&token)).expect("token encodes"));
    });

    criterion.bench_function("delegated_token_candid_decode", |bench| {
        bench.iter(|| {
            candid::decode_one::<DelegatedToken>(black_box(&encoded)).expect("token decodes")
        });
    });

    criterion.bench_function("delegated_token_encoded_size_bytes", |bench| {
        bench.iter(|| black_box(encoded.len()));
    });
}

#[cfg(all(
    feature = "auth-root-canister-sig-create",
    feature = "auth-root-canister-sig-verify"
))]
fn bench_canister_signature_verification(criterion: &mut Criterion) {
    use ic_canister_sig_creation::CanisterSigPublicKey;
    use ic_certification::{HashTree, leaf};
    use serde::Serialize;
    use serde_bytes::Bytes;

    #[derive(Serialize)]
    struct CanisterSignatureCbor<'a> {
        certificate: &'a Bytes,
        tree: HashTree,
    }

    let payload_hash = [8; 32];
    let message = root_canister_sig_verification_message(payload_hash);
    let public_key_der = CanisterSigPublicKey::new(p(1), ROOT_SIG_DOMAIN.to_vec()).to_der();
    let certificate = self_describing_cbor(&serde_cbor::Value::Null);
    let signature = CanisterSignatureCbor {
        certificate: Bytes::new(&certificate),
        tree: leaf(Vec::<u8>::new()),
    };
    let signature_cbor = self_describing_cbor(&signature);
    let ic_root_public_key_raw = vec![9; 96];

    criterion.bench_function("root_canister_sig_verify_invalid_certificate", |bench| {
        bench.iter(|| {
            let result = ic_signature_verification::verify_canister_sig(
                black_box(&message),
                black_box(&signature_cbor),
                black_box(&public_key_der),
                black_box(&ic_root_public_key_raw),
            );
            black_box(result).expect_err("fixture intentionally has invalid certificate")
        });
    });
}

#[cfg(all(
    feature = "auth-root-canister-sig-create",
    feature = "auth-root-canister-sig-verify"
))]
fn root_canister_sig_verification_message(payload_hash: [u8; 32]) -> Vec<u8> {
    let mut message = Vec::with_capacity(1 + ROOT_SIG_DOMAIN.len() + payload_hash.len());
    message.push(u8::try_from(ROOT_SIG_DOMAIN.len()).expect("domain length fits in u8"));
    message.extend_from_slice(ROOT_SIG_DOMAIN);
    message.extend_from_slice(&payload_hash);
    message
}

#[cfg(all(
    feature = "auth-root-canister-sig-create",
    feature = "auth-root-canister-sig-verify"
))]
fn self_describing_cbor<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let mut encoded = vec![0xd9, 0xd9, 0xf7];
    encoded.extend(serde_cbor::to_vec(value).expect("fixture must encode"));
    encoded
}

fn sample_delegated_token() -> DelegatedToken {
    let cert = sample_cert();
    let claims = DelegatedTokenClaims {
        subject: p(9),
        issuer_pid: cert.issuer_pid,
        cert_hash: [8; 32],
        issued_at_ns: 120_000_000_000,
        expires_at_ns: 180_000_000_000,
        aud: DelegationAudience::Project("test".to_string()),
        grants: vec![
            grant("project_hub", &["read", "upload"]),
            grant("project_instance", &["read"]),
            grant("user_shard", &["session"]),
        ],
        nonce: [7; 16],
        ext: Some(b"user_id=42;session=primary".to_vec()),
    };

    DelegatedToken {
        claims,
        proof: DelegationProof {
            cert,
            root_proof: sample_chain_key_root_proof(),
        },
        issuer_proof: IssuerProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: vec![3; 256],
            public_key_der: vec![4; 96],
        }),
    }
}

fn sample_chain_key_root_proof() -> RootProof {
    let key_id = ChainKeyKeyId {
        name: "key_1".to_string(),
    };
    RootProof::IcChainKeyBatchSignatureV1(IcChainKeyBatchSignatureProofV1 {
        header: ChainKeyBatchHeaderV1 {
            schema_version: 1,
            root_canister_id: p(1),
            batch_id: [10; 32],
            proof_epoch: 11,
            registry_epoch: 12,
            registry_hash: [13; 32],
            tree_root: [14; 32],
            not_before_ns: 100_000_000_000,
            expires_at_ns: 700_000_000_000,
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: key_id.clone(),
            derivation_path_hash: [15; 32],
            key_version: 1,
        },
        delegation_cert: ChainKeyDelegationCertV1 {
            root_canister_id: p(1),
            issuer_canister_id: p(2),
            proof_epoch: 11,
            issuer_proof_algorithm: IssuerProofAlgorithm::IcCanisterSignatureV1,
            issuer_proof_binding_hash: [7; 32],
            issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [4; 32] },
            max_token_ttl_ns: 120_000_000_000,
            audience: DelegationAudience::Project("test".to_string()),
            grants: vec![
                grant("project_hub", &["read", "upload"]),
                grant("project_instance", &["read"]),
                grant("user_shard", &["session"]),
            ],
            not_before_ns: 100_000_000_000,
            expires_at_ns: 700_000_000_000,
            registry_epoch: 12,
            registry_hash: [13; 32],
        },
        issuer_witness: ChainKeyBatchWitnessV1 {
            steps: vec![
                ChainKeyBatchWitnessStepV1::LeftSibling([16; 32]),
                ChainKeyBatchWitnessStepV1::RightSibling([17; 32]),
            ],
        },
        signature: ChainKeyRootSignatureV1 {
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id,
            derivation_path: vec![b"canic".to_vec(), b"delegation".to_vec()],
            public_key: vec![18; 33],
            signature: vec![19; 64],
        },
    })
}

fn sample_cert() -> DelegationCert {
    DelegationCert {
        root_pid: p(1),
        issuer_pid: p(2),
        issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
        issuer_proof_binding_hash: [7; 32],
        issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [4; 32] },
        issued_at_ns: 100_000_000_000,
        not_before_ns: 100_000_000_000,
        expires_at_ns: 700_000_000_000,
        max_token_ttl_ns: 120_000_000_000,
        aud: DelegationAudience::Project("test".to_string()),
        grants: vec![
            grant("project_hub", &["read", "upload"]),
            grant("project_instance", &["read", "write"]),
            grant("user_shard", &["session"]),
        ],
    }
}

fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
    DelegatedRoleGrant {
        target: CanisterRole::owned(role.to_string()),
        scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
    }
}

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn main() {
    let mut criterion = Criterion::default();
    bench_delegated_token_serialization(&mut criterion);
    #[cfg(all(
        feature = "auth-root-canister-sig-create",
        feature = "auth-root-canister-sig-verify"
    ))]
    bench_canister_signature_verification(&mut criterion);
    criterion.final_summary();
}
