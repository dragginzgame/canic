use canic_installer::release_set::{config_path, configured_install_targets, workspace_root};
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

// Print the local install target set: root plus the ordinary roles for its subnet.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let config_path = std::env::args_os()
        .nth(1)
        .map_or_else(|| config_path(&workspace_root), PathBuf::from);

    for role in configured_install_targets(&config_path, "root")? {
        println!("{role}");
    }

    Ok(())
}
