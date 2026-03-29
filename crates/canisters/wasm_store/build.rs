use std::{
    collections::BTreeSet,
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

macro_rules! collect_release_roles {
    ($cfg:expr) => {{
        let mut roles = BTreeSet::new();

        for subnet in $cfg.subnets.values() {
            for (role, canister_cfg) in &subnet.canisters {
                if canister_cfg.kind.to_string() == "root" {
                    continue;
                }

                roles.insert(role.as_str().to_string());
            }
        }

        roles.into_iter().collect::<Vec<_>>()
    }};
}

// Resolve the workspace root so generated code can address built `.dfx` artifacts.
fn workspace_root(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .ancestors()
        .find(|dir| {
            let cargo_toml = dir.join("Cargo.toml");
            cargo_toml.is_file()
                && fs::read_to_string(&cargo_toml)
                    .is_ok_and(|contents| contents.contains("[workspace]"))
        })
        .map(Path::to_path_buf)
        .expect("workspace root must contain Cargo.toml with [workspace]")
}

// Select the built artifact namespace used for generated embedded release input.
fn dfx_network_dir() -> &'static str {
    match env::var("DFX_NETWORK") {
        Ok(value) if value == "ic" => "ic",
        Ok(value) if value == "local" => "local",
        Ok(value) => panic!("unsupported DFX_NETWORK '{value}'; expected 'local' or 'ic'"),
        Err(_) => "local",
    }
}

// Generate the embedded release table seeded by the local wasm_store canister.
fn write_embedded_release_set(roles: &[String], manifest_dir: &Path, out_dir: &Path) {
    let repo_root = workspace_root(manifest_dir);
    let network_dir = dfx_network_dir();
    let mut body = String::from(
        "pub static EMBEDDED_RELEASE_SET: &[(canic::ids::CanisterRole, &[u8])] = &[\n",
    );

    for role in roles {
        let wasm_path = repo_root
            .join(".dfx")
            .join(network_dir)
            .join("canisters")
            .join(role)
            .join(format!("{role}.wasm.gz"));
        println!("cargo:rerun-if-changed={}", wasm_path.display());

        assert!(
            wasm_path.is_file(),
            "configured release artifact for role '{role}' is missing at {}",
            wasm_path.display()
        );

        let wasm_literal = wasm_path.display();
        let _ = writeln!(
            body,
            "    (canic::ids::CanisterRole::new(\"{role}\"), include_bytes!(r#\"{wasm_literal}\"#) as &[u8]),"
        );
    }

    body.push_str("];\n");

    fs::write(out_dir.join("embedded_release_set.rs"), body)
        .expect("write embedded wasm_store release set");
}

fn main() {
    canic::build_with!("../canic.toml", |_cfg_str, _cfg_path, cfg| {
        let manifest_dir =
            PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
        let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
        let roles = collect_release_roles!(cfg);

        write_embedded_release_set(&roles, &manifest_dir, &out_dir);
    });
}
