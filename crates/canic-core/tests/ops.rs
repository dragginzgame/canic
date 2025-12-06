// Minimal integration test scaffold to guard API drift across modules.

#[cfg(feature = "ic")]
#[test]
fn request_type_compiles() {
    use canic_core::ops::request::create_canister_request;
    let _ = create_canister_request::<()>;
}
