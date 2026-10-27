//! Native schema producer linked to the selected managed application.

use managed_lifecycle_probe as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
