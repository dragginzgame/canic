//! Exact reviewed identities and supplied custody stay intact in native observation frames.
use super::*;
use crate::dto::{
    pool_import::{PoolImportBootstrap, PoolImportIdentity},
    pool_import_read::Bootstrap,
};

#[test]
fn status_preserves_exact_sequence_and_plan_and_refuses_other_methods() {
    let identity = PoolImportIdentity {
        sequence: u64::MAX,
        plan_sha256: [7; 32],
    };
    let mut bytes =
        ic_wire::native::encode_request(status::METHOD_ID, &identity, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<PoolImportIdentity>(
            status::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .unwrap(),
        identity
    );
    assert!(matches!(
        ic_wire::native::decode_request::<PoolImportIdentity>(
            context::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    bytes.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<PoolImportIdentity>(
            status::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[test]
fn bootstrap_preserves_every_source_and_the_original_review() {
    let expected = PoolImportBootstrap {
        review_sha256: [8; 32],
        operator: ic_platform::principal::ExternalPrincipal::from_slice(&[1]),
        sources: vec![
            ic_platform::principal::ExternalPrincipal::from_slice(&[2]),
            ic_platform::principal::ExternalPrincipal::from_slice(&[3]),
        ],
    };
    let native = Bootstrap::from(expected.clone());
    assert_eq!(PoolImportBootstrap::from(native), expected);
}

#[cfg(feature = "wire-schema")]
#[test]
fn both_import_observation_graphs_have_native_metadata() {
    ic_wire::typescript::native_schema::<PoolImportIdentity>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::pool_import_read::Context, crate::dto::error::Error>,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::pool_census::ImportStatus, crate::dto::error::Error>,
    >()
    .unwrap();
}
