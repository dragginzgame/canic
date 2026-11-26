//! Inspect the linked actor and its shared mounts.
use example_user_directory_hub as _;

fn main() -> Result<(), String> {
    canic::inspection::print_manifest(&Default::default())
}
