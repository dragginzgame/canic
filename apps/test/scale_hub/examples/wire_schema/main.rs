//! Native schema producer linked to the selected managed application.

use canister_scale_hub as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
