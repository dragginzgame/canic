use super::*;
use serde::de::DeserializeOwned;

fn assert_transport<T, U>(value: &T)
where
    T: CandidType + Serialize + DeserializeOwned + Eq + std::fmt::Debug,
    U: Serialize + DeserializeOwned,
{
    let bytes = ic_platform::candid::contract::encode_one(value).unwrap();
    let decoded: T = ic_platform::candid::decoder::decode_one(&bytes).unwrap();
    assert_eq!(&decoded, value);
    let mut canic_bytes = Vec::new();
    ciborium::into_writer(value, &mut canic_bytes).unwrap();
    let protocol: U = ciborium::from_reader(canic_bytes.as_slice()).unwrap();
    let mut protocol_bytes = Vec::new();
    ciborium::into_writer(&protocol, &mut protocol_bytes).unwrap();
    assert_eq!(canic_bytes, protocol_bytes);
}

#[test]
fn proof_transport_roundtrips_candid_and_preserves_protocol_cbor() {
    use ic_auth_protocol_types as protocol;
    assert_transport::<_, protocol::ChainKeyBatchHeaderV1>(&ChainKeyBatchHeaderV1 {
        schema_version: 1,
        root_canister_id: Principal::from_slice(&[1]),
        batch_id: [2; 32],
        proof_epoch: 3,
        registry_epoch: 4,
        registry_hash: [5; 32],
        tree_root: [6; 32],
        not_before_ns: 7,
        expires_at_ns: u64::MAX,
        algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
        key_id: ChainKeyKeyId {
            name: "key_1".into(),
        },
        derivation_path_hash: [8; 32],
        key_version: 9,
    });
    assert_transport::<_, protocol::ChainKeyBatchWitnessV1>(&ChainKeyBatchWitnessV1 {
        steps: vec![
            ChainKeyBatchWitnessStepV1::LeftSibling([10; 32]),
            ChainKeyBatchWitnessStepV1::RightSibling([11; 32]),
        ],
    });
    assert_transport::<_, protocol::ChainKeyRootSignatureV1>(&ChainKeyRootSignatureV1 {
        algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
        key_id: ChainKeyKeyId {
            name: "key_1".into(),
        },
        derivation_path: vec![vec![], vec![12, 13]],
        public_key: vec![14, 15],
        signature: vec![16, 17],
    });
    assert_transport::<_, protocol::IssuerProofAlgorithm>(
        &IssuerProofAlgorithm::IcCanisterSignatureV1,
    );
    assert_transport::<_, protocol::IssuerProofBinding>(
        &IssuerProofBinding::IcCanisterSignatureV1 {
            seed_hash: [18; 32],
        },
    );
    assert_transport::<_, protocol::IcCanisterSignatureProofV1>(&IcCanisterSignatureProofV1 {
        signature_cbor: vec![19, 20],
        public_key_der: vec![21, 22],
    });
}
