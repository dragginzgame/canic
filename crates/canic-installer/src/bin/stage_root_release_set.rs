use canic_installer::release_set::{
    dfx_root, load_root_release_set_manifest, resolve_artifact_root, resume_root_bootstrap,
    root_release_set_manifest_path, stage_root_release_set,
};
use std::env;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

// Stage the build-produced ordinary release manifest into root and resume bootstrap.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root_canister = env::args()
        .nth(1)
        .or_else(|| env::var("ROOT_CANISTER").ok())
        .unwrap_or_else(|| "root".to_string());
    let dfx_root = dfx_root()?;
    let network = env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string());
    let artifact_root = resolve_artifact_root(&dfx_root, &network)?;
    let manifest_path = root_release_set_manifest_path(&artifact_root)?;
    let manifest = load_root_release_set_manifest(&manifest_path)?;

    stage_root_release_set(&dfx_root, &root_canister, &manifest)?;
    resume_root_bootstrap(&root_canister)?;
    Ok(())
}
