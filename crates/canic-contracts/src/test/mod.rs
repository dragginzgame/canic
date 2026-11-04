//! Passive byte-codec fixtures; these values confer no runtime authority.

pub mod support;

#[test]
fn dto_principal_retains_external_identity_and_wire_bytes() {
    use crate::dto::prelude::Principal;
    use ic_platform_types::principal::ExternalPrincipal;

    let external = ExternalPrincipal::from_slice(&[42; 29]);
    let dto: Principal = external;
    let owned: ic_platform_types::Principal = external.into();
    let bytes = crate::protocol::authority::encode(&dto).unwrap();
    assert_eq!(bytes, crate::protocol::authority::encode(&owned).unwrap());
    assert_eq!(
        crate::protocol::authority::decode::<Principal>(&bytes).unwrap(),
        external
    );
}
