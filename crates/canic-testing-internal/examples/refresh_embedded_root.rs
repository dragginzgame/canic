//! Explicit local refresh; ordinary tests only verify the embedded peer.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    canic_testing_internal::embedded_root::refresh(&workspace)
}
