//! Native schema producer linked to the standalone receipt fixture.
use intent_authority as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
