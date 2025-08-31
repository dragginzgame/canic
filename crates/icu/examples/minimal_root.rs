// Example: minimal root canister scaffold.
// Compile with `--features ic` to include the canister module.

#[cfg(feature = "ic")]
mod canister {
    #![allow(unexpected_cfgs)]
    use icu::prelude::*;
    use icu::types::CanisterType;

    // Note: In a real canister crate, add this to build.rs:
    // fn main() { icu::icu_build!("../icu.toml"); }
    // Examples cannot run build scripts, so this example omits it.

    // Set up a minimal root canister with default hooks.
    icu_start_root!();

    const fn icu_setup() {}
    async fn icu_install() {}
    async fn icu_upgrade() {}

    // Minimal WASMS set required by the macro; empty in this example.
    pub static WASMS: &[(CanisterType, &[u8])] = &[];

    #[update]
    async fn ping() -> String {
        "pong".to_string()
    }

    export_candid!();
}

fn main() {
    println!("minimal_root example (use --features ic)");
}
