use super::expand;
use quote::quote;

#[test]
fn refuses_ambiguous_contracts_and_session_signatures() {
    let handler = quote!(
        fn read() -> bool {
            true
        }
    );
    for (options, message) in [
        (quote!(contract = Read, method = 42), "contract supplies"),
        (quote!(limits = SMALL, limits = LARGE), "repeated"),
        (quote!(scope = "read"), "verified session first"),
    ] {
        let error = expand(false, options, handler.clone()).unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
    let error = expand(
        false,
        quote!(),
        quote!(
            async fn read() {}
        ),
    )
    .unwrap_err();
    assert!(error.to_string().contains("async endpoints require"));
}

#[test]
fn conventional_mount_checks_role_options_and_accepts_reordering() {
    use super::provisioning;
    let hub = quote!(ProvisioningRole::Hub, app::Row => app::Users, authority = "hub", pages = 32,
        payload = name, status = app::Status, generator = app::candidate,
        database = "hub.rows", write_scope = "signup", config = "hub.config");
    assert!(provisioning::expand(hub.clone()).is_ok());
    let shard = quote!(Child, app::Row => app::Users, payload = name, prefix = "app.",
        database = "app.rows", authority = "shard", pages = 32, parent_storage_key = "app.parent");
    assert!(provisioning::expand(shard.clone()).is_ok());
    for (options, expected) in [
        (quote!(#hub, pages = 8), "duplicate pages"),
        (
            quote!(#shard, write_scope = "signup"),
            "require a hub layout",
        ),
        (
            quote!(Hub, Row => Users, database = "rows"),
            "missing shard option: authority",
        ),
    ] {
        assert!(
            provisioning::expand(options)
                .unwrap_err()
                .to_string()
                .contains(expected)
        );
    }
}
