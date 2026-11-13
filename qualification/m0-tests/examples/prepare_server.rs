//! Explicit server preparation, separate from offline test admission.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root")
        .join("target/m0/server");
    let binary = ic_testkit::pic::server_assets::setup(&directory, || false)?;
    println!("{}", binary.display());
    Ok(())
}
