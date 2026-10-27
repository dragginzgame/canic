//! Native schema producer for the selected manual sandbox.

use canister_sandbox_blank as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
