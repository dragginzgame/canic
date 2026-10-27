//! Emit the exact native schema for the standalone allocation peer.
use sharding_root_stub as _;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
