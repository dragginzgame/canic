mod fixture {
    canic::canic_emit_canic_metadata_endpoints!();

    /// Return the macro-generated standards metadata for this test crate.
    pub fn metadata() -> canic::dto::standards::CanicStandardsResponse {
        canic_standards()
    }
}

// Standards metadata must describe the crate exporting the endpoint.
#[test]
fn canic_standards_uses_endpoint_crate_metadata() {
    let metadata = fixture::metadata();

    assert_eq!(metadata.name, env!("CARGO_PKG_NAME"));
    assert_eq!(metadata.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(metadata.description, env!("CARGO_PKG_DESCRIPTION"));
    assert_ne!(metadata.name, "canic-core");
}
