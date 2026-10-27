//! Native schema producer linked to this exact consumer.

use blob_service_consumer as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
