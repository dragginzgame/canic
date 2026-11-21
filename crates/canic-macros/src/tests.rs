use super::{Mode, expand};
use quote::quote;

#[test]
fn refuses_ambiguous_contracts_and_invalid_signatures() {
    let handler = quote!(
        fn read() -> bool {
            true
        }
    );
    for (options, message) in [
        (quote!(roles = &[], role_contract = Read), "roles cannot"),
        (quote!(contract = Read, roles = &[]), "roles cannot"),
        (quote!(roles = &[], roles = ANY), "repeated"),
        (quote!(method = 42), "unknown"),
        (quote!(contract = Read, method = 42), "unknown"),
        (quote!(contract = Read, limits = SMALL), "contract supplies"),
        (quote!(limits = SMALL, limits = LARGE), "repeated"),
    ] {
        let error = expand(Mode::Query, options, handler.clone()).unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
    let error = expand(
        Mode::Query,
        quote!(),
        quote!(
            async fn read() {}
        ),
    )
    .unwrap_err();
    assert!(error.to_string().contains("async endpoints require"));
}

#[test]
fn scoped_handler_reserves_auth_for_admitted_identity() {
    let error = expand(
        Mode::Query,
        quote!(scope = "read"),
        quote!(
            fn read(auth: String) {}
        ),
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("reserved for the admitted identity")
    );
}
