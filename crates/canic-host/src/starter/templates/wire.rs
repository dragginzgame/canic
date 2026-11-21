//! Inspect the linked actor and its shared mounts.
use starter_app as _;

fn main() -> Result<(), String> {
    canic::inspection::print_manifest(&Default::default())
}
