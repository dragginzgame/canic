#![allow(clippy::unused_async)]
// Example: minimal root canister scaffold.
// Compile this example with `cargo build -p canic --example minimal_root`.

mod canister {
    use canic::prelude::*;

    // Set up a minimal root canister with default hooks.
    canic::start_root!();

    #[expect(clippy::unused_async)]
    async fn canic_setup() {}
    async fn canic_install() {}
    async fn canic_upgrade() {}

    // Minimal WASMS set required by the macro; empty in this example
    #[expect(dead_code)]
    pub static WASMS: &[(CanisterRole, &[u8])] = &[];

    #[canic_update]
    async fn ping() -> Result<String, canic::Error> {
        Ok("pong".to_string())
    }

    #[cfg(debug_assertions)]
    canic::export_candid!();
}

fn main() {
    println!("minimal_root example");
}
