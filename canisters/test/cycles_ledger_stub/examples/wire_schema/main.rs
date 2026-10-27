//! Emit native controls without reconstructing external Candid protocols.
use cycles_ledger_stub as _;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    canic::endpoint::schema::producer::run()
}
