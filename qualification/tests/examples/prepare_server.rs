//! Explicit server preparation, separate from offline test admission.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = canic_qualification::server_directory();
    let binary = ic_testkit::pic::server_assets::setup(&directory, || false)?;
    println!("{}", binary.display());
    Ok(())
}
