//! Produce native funding probe contracts without changing the external protocols.
use root_funding_probe as _;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
