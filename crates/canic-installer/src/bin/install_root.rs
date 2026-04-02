use canic_installer::install_root::{InstallRootOptions, install_root};

// Run the generic local-root install entrypoint.
fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

// Execute the published local-root install flow against an already running replica.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    install_root(InstallRootOptions::from_env_and_args())
}
