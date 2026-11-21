//! Inspect the linked actor and its shared mounts.
use ex_template as _;

fn main() -> Result<(), String> {
    canic::inspection::print_manifest(&Default::default())
}
