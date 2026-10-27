//! Every nested native pool observation must be representable without Candid metadata.
#[test]
fn census_has_complete_native_metadata() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::pool_census::Census, crate::dto::error::Error>,
    >()
    .unwrap();
}
