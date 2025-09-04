// Minimal integration test scaffold to guard API drift across modules.

#[cfg(feature = "ic")]
#[test]
fn request_type_compiles() {
    use icu::ops::request::create_canister_request;
    let _f = create_canister_request::<()>;
}
