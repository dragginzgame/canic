use canic_contracts::{
    dto::auth::{
        ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessStepV1,
        ChainKeyBatchWitnessV1, ChainKeyDelegationCertV1, ChainKeyKeyId, ChainKeyRootSignatureV1,
        DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
        IcChainKeyBatchSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding, RootProof,
    },
    ids::{CanisterRole, CanonicalNetworkId, FleetId, FleetKey},
};
use canic_core::cdk::types::Principal;
use criterion::Criterion;
use std::hint::black_box;

fn fleet() -> FleetKey {
    FleetKey {
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_id: FleetId::from_generated_bytes([1; 32]),
    }
}

fn bench_delegation_proof_serialization(criterion: &mut Criterion) {
    let proof = DelegationProof {
        cert: sample_cert(),
        root_proof: sample_chain_key_root_proof(),
    };
    let encoded = ic_platform::candid::contract::encode_one(&proof)
        .expect("sample delegation proof must encode");

    criterion.bench_function("delegation_proof_candid_encode", |bench| {
        bench.iter(|| {
            ic_platform::candid::contract::encode_one(black_box(&proof)).expect("proof encodes")
        });
    });

    criterion.bench_function("delegation_proof_candid_decode", |bench| {
        bench.iter(|| {
            ic_platform::candid::decoder::decode_one::<DelegationProof>(black_box(&encoded))
                .expect("proof decodes")
        });
    });

    criterion.bench_function("delegation_proof_encoded_size_bytes", |bench| {
        bench.iter(|| black_box(encoded.len()));
    });
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
            audience: DelegationAudience::Fleet(fleet()),
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
        aud: DelegationAudience::Fleet(fleet()),
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
    bench_delegation_proof_serialization(&mut criterion);
    criterion.final_summary();
}
