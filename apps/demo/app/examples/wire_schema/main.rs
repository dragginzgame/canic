//! Native schema producer linked to the selected managed application.

use demo_fleet_app as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
