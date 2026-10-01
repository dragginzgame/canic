//! Read-only fixture qualification before the complete test runner starts its suites.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    canic_testing_internal::embedded_root::verify(&workspace)
}
