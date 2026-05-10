mod fixture {
    canic::canic_emit_canic_metadata_endpoints!();
}

// Canic metadata must describe the crate exporting the endpoint.
#[test]
fn canic_metadata_uses_endpoint_crate_metadata() {
    let metadata = canic::__internal::core::api::metadata::CanicMetadataApi::metadata_for(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
        canic::VERSION,
        7,
    );

    assert_eq!(metadata.package_name, env!("CARGO_PKG_NAME"));
    assert_eq!(metadata.package_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(metadata.package_description, env!("CARGO_PKG_DESCRIPTION"));
    assert_eq!(metadata.canic_version, canic::VERSION);
    assert_ne!(metadata.package_name, "canic-core");
    assert_eq!(metadata.canister_version, 7);
}
