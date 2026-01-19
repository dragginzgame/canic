mod root;

use canic::{
    Error,
    cdk::{types::Principal, utils::time::now_secs},
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof},
    protocol,
};
use canic_internal::canister;
use hex::encode as hex_encode;
use ic_certification::{Certificate, HashTree, LookupResult};
use root::harness::setup_root;
use serde::Deserialize;
use serde_bytes::ByteBuf;

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

#[test]
#[allow(clippy::too_many_lines)]
fn delegated_token_flow() {
    if std::env::var("CANIC_CERTIFIED_QUERIES").as_deref() != Ok("1") {
        eprintln!("skipping delegation flow: certified queries unavailable");
        return;
    }

    let setup = setup_root();

    let auth_hub_pid = setup
        .subnet_directory
        .get(&canister::AUTH_HUB)
        .copied()
        .expect("auth_hub must exist in subnet directory");

    let test_pid = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist in subnet directory");

    let tenant = p(7);
    let audiences = vec!["login".to_string()];
    let scopes = vec!["read".to_string()];

    let provisioned: Result<Result<(Principal, DelegationCert), Error>, Error> =
        setup.pic.update_call(
            auth_hub_pid,
            "provision_auth_shard",
            (tenant, audiences.clone(), scopes.clone(), 3600_u64),
        );

    let (shard_pid, cert) = provisioned
        .expect("provision_auth_shard transport failed")
        .expect("provision_auth_shard application failed");

    let mut proof = None;
    let mut last_error = None;
    for _ in 0..10 {
        // Allow certified_data to be certified after prepare before requesting the signature.
        setup.pic.certify_time();

        let issued: Result<Result<DelegationProof, Error>, Error> = setup.pic.query_call_as(
            setup.root_id,
            auth_hub_pid,
            protocol::CANIC_DELEGATION_GET,
            (cert.clone(),),
        );

        match issued {
            Ok(Ok(found)) => {
                proof = Some(found);
                break;
            }
            Ok(Err(err))
                if err
                    .message
                    .contains("certified_data doesn't match sig tree digest") =>
            {
                last_error = Some(err);
            }
            Ok(Err(err)) => {
                panic!("canic_delegation_get application failed: {err:?}");
            }
            Err(err) => {
                panic!("canic_delegation_get transport failed: {err:?}");
            }
        }
    }

    let proof = proof.unwrap_or_else(|| {
        panic!("canic_delegation_get retries exhausted: {last_error:?}");
    });

    let finalized: Result<Result<(), Error>, Error> = setup.pic.update_call(
        auth_hub_pid,
        "finalize_auth_shard",
        (shard_pid, proof.clone()),
    );

    match finalized {
        Ok(Ok(())) => {}
        Ok(Err(err))
            if err
                .message
                .contains("certified_data doesn't match sig tree digest") =>
        {
            let debug = debug_delegation_signature(&proof, setup.root_id)
                .unwrap_or_else(|| "signature debug unavailable".to_string());
            panic!("finalize_auth_shard application failed: {err:?}\n{debug}");
        }
        Ok(Err(err)) => {
            panic!("finalize_auth_shard application failed: {err:?}");
        }
        Err(err) => {
            panic!("finalize_auth_shard transport failed: {err:?}");
        }
    }

    let set_proof: Result<Result<(), Error>, Error> = setup.pic.update_call_as(
        test_pid,
        setup.root_id,
        "test_set_delegation_proof",
        (proof,),
    );

    set_proof
        .expect("test_set_delegation_proof transport failed")
        .expect("test_set_delegation_proof application failed");

    let now = now_secs();
    let claims = DelegatedTokenClaims {
        sub: p(9),
        aud: audiences[0].clone(),
        scopes,
        iat: now,
        exp: now + 60,
        nonce: None,
    };

    let minted: Result<Result<DelegatedToken, Error>, Error> =
        setup
            .pic
            .update_call(shard_pid, "auth_shard_mint_token", (claims,));

    let token = minted
        .expect("auth_shard_mint_token transport failed")
        .expect("auth_shard_mint_token application failed");

    let verify: Result<Result<(), Error>, Error> =
        setup
            .pic
            .update_call(test_pid, "test_verify_delegated_token", (token,));

    verify
        .expect("test_verify_delegated_token transport failed")
        .expect("test_verify_delegated_token application failed");
}

#[derive(Deserialize)]
struct CanisterSigDebug {
    certificate: ByteBuf,
    tree: HashTree,
}

fn debug_delegation_signature(proof: &DelegationProof, root_id: Principal) -> Option<String> {
    let sig: CanisterSigDebug = serde_cbor::from_slice(&proof.cert_sig).ok()?;
    let sig_digest = sig.tree.digest();

    let cert: Certificate = serde_cbor::from_slice(sig.certificate.as_ref()).ok()?;
    let cert_path = [b"canister", root_id.as_slice(), b"certified_data"];
    let cert_data = match cert.tree.lookup_path(cert_path) {
        LookupResult::Found(bytes) => Some(bytes.to_vec()),
        other => {
            return Some(format!(
                "signature debug: sig_tree_digest=0x{}, certified_data_lookup={other:?}",
                hex_encode(sig_digest.as_ref())
            ));
        }
    };

    let cert_data = cert_data?;
    Some(format!(
        "signature debug: sig_tree_digest=0x{}, cert_certified_data=0x{}",
        hex_encode(sig_digest.as_ref()),
        hex_encode(cert_data.as_slice())
    ))
}
