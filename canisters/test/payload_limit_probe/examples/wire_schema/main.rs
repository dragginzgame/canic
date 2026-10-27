//! Native schema producer linked to the selected framework probe.

use payload_limit_probe as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
