//! Native renewal frames preserve typed refusals and exact method identity.
use crate::dto::{error::Error, issuer_renewal::RootIssuerRenewalStatusResponse};
#[test]
fn renewal_wire_response_preserves_typed_remote_error() {
    let expected = Error::from_registered(crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED);
    let frame = ic_wire::native::encode_response(
        super::METHOD_ID,
        &Err::<RootIssuerRenewalStatusResponse, _>(expected),
        super::RESPONSE_LIMITS,
    )
    .unwrap();
    let decoded: Result<RootIssuerRenewalStatusResponse, Error> =
        ic_wire::native::decode_response(super::METHOD_ID, &frame, super::RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded, Err(expected));
}

#[cfg(feature = "wire-schema")]
#[test]
fn renewal_graph_is_complete() {
    ic_wire::typescript::native_schema::<crate::dto::issuer_renewal::RootIssuerRenewalStatusRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<RootIssuerRenewalStatusResponse, Error>>().unwrap();
}
