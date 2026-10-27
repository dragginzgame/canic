//! Exercise the canonical Root public-key command against real PocketIC management.
//!
//! Cover controller authorization, deterministic replay and derivation-path binding.

use super::*;
use canic::dto::auth::RootChainKeyPublicKeyRequest;
use canic_contracts::protocol::issuer_control::{self, public_key};

pub(super) fn assert_controller_key_discovery(pic: &PocketIc, root: Principal) {
    let key = |caller, path: Vec<Vec<u8>>| -> Result<Vec<u8>, Error> {
        let request = RootChainKeyPublicKeyRequest {
            key_id: "key_1".into(),
            derivation_path: path,
        };
        let bytes = ic_wire::native::encode_request(
            public_key::METHOD_ID,
            &request,
            issuer_control::REQUEST_LIMITS,
        )
        .unwrap();
        let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
            pic.update_call(root, caller, public_key::METHOD, bytes)
        })
        .unwrap()
        .unwrap();
        ic_wire::native::decode_response(
            public_key::METHOD_ID,
            &reply,
            issuer_control::RESPONSE_LIMITS,
        )
        .unwrap()
    };
    let outsider = Principal::self_authenticating(b"uncontrolled public-key requester");
    let Err(rejected) = key(outsider, vec![b"canic".to_vec()]) else {
        panic!("public-key discovery must require a Root controller")
    };
    assert_eq!(
        rejected.code(),
        Error::from(canic_core::access::AccessError::ControllerRequired).code()
    );
    let path = vec![b"canic".to_vec(), b"delegation".to_vec()];
    let first = key(Principal::anonymous(), path.clone()).unwrap();
    let source = format!(
        r#"
[app]
name = "offline_key"
[roles.root]
kind = "root"
[auth.delegated_tokens]
enabled = true
build_network = "local"
root_canister_id = "{root}"
[auth.delegated_tokens.chain_key_root_proof]
public_key_derivation = "pocketic"
key_id = "key_1"
derivation_path_hex = ["63616e6963", "64656c65676174696f6e"]
key_version = 1
min_accepted_key_version = 1
min_accepted_proof_epoch = 1
min_accepted_registry_epoch = 1
valid_from_ns = 1
accept_until_ns = 18446744073709551615
max_revocation_latency_ns = 60000000000
"#
    );
    let config = canic_core::bootstrap::parse_config_model(&source).unwrap();
    let derived = config
        .auth
        .delegated_tokens
        .chain_key_root_proof
        .public_key_hex
        .unwrap();
    assert_eq!(
        canic_core::cdk::utils::hash::decode_hex(&derived).unwrap(),
        first,
        "offline build derivation must match the management canister"
    );
    assert_eq!(first.len(), 33, "compressed secp256k1 public key");
    assert!(matches!(first[0], 2 | 3));
    let replay = key(Principal::anonymous(), path).unwrap();
    assert_eq!(first, replay);
    let other_path = key(Principal::anonymous(), vec![b"other".to_vec()]).unwrap();
    assert_ne!(first, other_path);
}
