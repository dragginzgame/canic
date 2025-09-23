// Example: bootstrap the pieces that `icu_start_root!()` wires together.
// Run with `cargo run --example minimal_root`.

use icu::{
    Error,
    config::Config,
    memory::{
        canister::{CanisterRoot, CanisterState},
        subnet::SubnetRegistry,
    },
    state::wasm::WasmRegistry,
    types::{CanisterType, Principal},
};

const SHARD_WASM: &[u8] = b"\0asm\x01\0\0\0";
static WASMS: &[(CanisterType, &[u8])] = &[(CanisterType::new("demo_shard"), SHARD_WASM)];

fn main() -> Result<(), Error> {
    bootstrap_root_demo()
}

fn bootstrap_root_demo() -> Result<(), Error> {
    let root = principal(1);
    let config_toml = format!(
        r#"
controllers = ["{controller}"]

[canisters.demo_shard]
auto_create = true
uses_directory = true
"#,
        controller = root.to_text(),
    );

    Config::init_from_toml(&config_toml)?;

    let root_entry = SubnetRegistry::init_root(root);
    CanisterRoot::set(root);
    CanisterState::set_view(root_entry.into());

    WasmRegistry::import(WASMS);

    let shard_type = CanisterType::new("demo_shard");
    let shard_pid = principal(2);
    SubnetRegistry::create(shard_pid, &shard_type, root);
    let wasm = WasmRegistry::try_get(&shard_type)?;
    SubnetRegistry::install(shard_pid, wasm.module_hash())?;

    println!("root pid: {}", root.to_text());
    println!("registered types:");
    for view in SubnetRegistry::directory() {
        println!("  {ty} -> {pid}", ty = view.ty, pid = view.pid.to_text());
    }

    Ok(())
}

const fn principal(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}
