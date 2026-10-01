//! Certificate projection rejects incomplete controller, module and target evidence.

use super::*;
use ic_certification::{HashTree, fork, labeled, leaf, pruned};

fn certificate(
    principal: Principal,
    controllers: &[Principal],
    module: Option<HashTree>,
) -> Certificate {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(controllers, &mut bytes).unwrap();
    let controllers = labeled(b"controllers", leaf(bytes));
    let fields = module.map_or_else(
        || controllers.clone(),
        |module| fork(controllers.clone(), module),
    );
    Certificate {
        tree: labeled(b"canister", labeled(principal.as_slice(), fields)),
        signature: Vec::new(),
        delegation: None,
    }
}

#[test]
fn certified_absence_is_distinct_from_unknown_or_malformed_module() {
    let principal = Principal::from_slice(&[1]);
    let owner = Principal::from_slice(&[2]);
    let empty = certificate(principal, &[owner], None);
    assert_eq!(
        project(&empty, b"test root", principal)
            .unwrap()
            .module_sha256(),
        None
    );
    let installed = certificate(
        principal,
        &[owner],
        Some(labeled(b"module_hash", leaf([3; 32]))),
    );
    assert_eq!(
        project(&installed, b"test root", principal)
            .unwrap()
            .module_sha256(),
        Some(hex_bytes([3; 32]).as_str())
    );
    for module in [pruned([0; 32]), labeled(b"module_hash", leaf([3; 31]))] {
        assert!(matches!(
            project(
                &certificate(principal, &[owner], Some(module)),
                b"test root",
                principal
            ),
            Err(CertifiedCustodyError::IncompleteCertificate { .. })
        ));
    }
}

#[test]
fn controller_evidence_rejects_duplicates_empty_values_and_wrong_target() {
    let principal = Principal::from_slice(&[1]);
    let owner = Principal::from_slice(&[2]);
    for owners in [Vec::new(), vec![owner, owner]] {
        assert!(matches!(
            project(
                &certificate(principal, &owners, None),
                b"test root",
                principal
            ),
            Err(CertifiedCustodyError::IncompleteCertificate { .. })
        ));
    }
    assert!(matches!(
        project(&certificate(principal, &[owner], None), b"test root", owner),
        Err(CertifiedCustodyError::IncompleteCertificate { .. })
    ));
    let additional = Principal::from_slice(&[3]);
    let observed = project(
        &certificate(principal, &[additional, owner], None),
        b"test root",
        principal,
    )
    .unwrap();
    assert_eq!(observed.controllers(), &[owner, additional]);
}
