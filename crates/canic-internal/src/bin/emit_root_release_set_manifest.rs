use canic_internal::release_set::{emit_root_release_set_manifest, workspace_root};
use std::env;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

// Emit the current build-produced ordinary root release-set manifest from `.dfx` artifacts.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let network = env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string());
    let manifest_path = emit_root_release_set_manifest(&workspace_root, &network)?;
    println!("{}", manifest_path.display());
    Ok(())
}
