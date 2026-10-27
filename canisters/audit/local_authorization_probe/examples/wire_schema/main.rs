//! Native schema producer linked to the selected managed application.

use local_authorization_probe as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
