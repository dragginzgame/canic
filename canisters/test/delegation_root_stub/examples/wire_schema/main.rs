//! Native schema producer for the actual installed Root fixture.

use delegation_root_stub as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
