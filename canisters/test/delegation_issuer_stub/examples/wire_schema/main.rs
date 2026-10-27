//! Native schema producer linked to the selected managed application.

use delegation_issuer_stub as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
