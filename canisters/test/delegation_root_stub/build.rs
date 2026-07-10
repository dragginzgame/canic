use std::{env, fs, path::PathBuf};

const EMBEDDED_CANISTER_PACKAGES: [&str; 3] = [
    "delegation_issuer_stub",
    "project_hub_stub",
    "project_instance_stub",
];
const WASM32_TARGET: &str = "wasm32-unknown-unknown";
const EMPTY_WASM_MODULE: &[u8] = b"\0asm\x01\0\0\0";

// Compile root configuration and embed explicitly prepared test-canister artifacts.
fn main() {
    configure_cfg();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = discover_workspace_root(&manifest_dir);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let outer_target_dir = env::var("CARGO_TARGET_DIR").map(PathBuf::from).map_or_else(
        |_| workspace_root.join("target"),
        |path| absolute_from(&workspace_root, path),
    );
    let target = env::var("TARGET").expect("TARGET");
    let profile_dir = output_profile_dir(&out_dir);

    canic::build!("canic.toml");
    if target == WASM32_TARGET {
        copy_prepared_test_canisters(&outer_target_dir, &out_dir, &target, &profile_dir);
    } else {
        write_host_placeholders(&out_dir);
    }
    emit_rerun_inputs(&workspace_root, &outer_target_dir, &target, &profile_dir);
}

// Resolve relative Cargo target directories against the workspace invocation root.
fn absolute_from(workspace_root: &std::path::Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    }
}

// Register build-script inputs used by this build script.
fn configure_cfg() {
    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
    println!("cargo:rerun-if-env-changed=TARGET");
}

// Resolve Cargo's actual target profile directory, including custom profiles.
fn output_profile_dir(out_dir: &std::path::Path) -> String {
    out_dir
        .ancestors()
        .nth(3)
        .and_then(std::path::Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .map_or_else(
            || {
                panic!(
                    "unable to resolve Cargo profile directory from OUT_DIR {}",
                    out_dir.display()
                )
            },
            ToString::to_string,
        )
}

// Copy one explicitly prepared wasm artifact into this build script's OUT_DIR.
fn copy_prepared_test_canisters(
    outer_target_dir: &std::path::Path,
    out_dir: &std::path::Path,
    target: &str,
    profile: &str,
) {
    for package in EMBEDDED_CANISTER_PACKAGES {
        let wasm_path = prepared_wasm_path(outer_target_dir, target, profile, package);
        let bytes = fs::read(&wasm_path).unwrap_or_else(|err| {
            panic!(
                "prepared {package} wasm is unavailable at {}: {err}; build embedded test canisters before delegation_root_stub",
                wasm_path.display()
            )
        });
        assert!(
            bytes.starts_with(b"\0asm"),
            "prepared {package} artifact at {} is not a wasm module",
            wasm_path.display()
        );
        fs::write(out_dir.join(format!("{package}.wasm")), bytes)
            .unwrap_or_else(|err| panic!("write embedded {package} wasm failed: {err}"));
    }
}

// Host checks do not execute the embedded canisters and only require include paths.
fn write_host_placeholders(out_dir: &std::path::Path) {
    for package in EMBEDDED_CANISTER_PACKAGES {
        fs::write(out_dir.join(format!("{package}.wasm")), EMPTY_WASM_MODULE)
            .unwrap_or_else(|err| panic!("write host placeholder for {package} failed: {err}"));
    }
}

// Emit rerun markers for prepared artifacts and their source packages.
fn emit_rerun_inputs(
    workspace_root: &std::path::Path,
    outer_target_dir: &std::path::Path,
    target: &str,
    profile: &str,
) {
    println!("cargo:rerun-if-changed=build.rs");
    for package in EMBEDDED_CANISTER_PACKAGES {
        emit_canister_rerun_inputs(workspace_root, package);
        if target == WASM32_TARGET {
            println!(
                "cargo:rerun-if-changed={}",
                prepared_wasm_path(outer_target_dir, target, profile, package).display()
            );
        }
    }
}

// Emit rerun markers for one embedded test-canister package.
fn emit_canister_rerun_inputs(workspace_root: &std::path::Path, package: &str) {
    let package_root = workspace_root.join("canisters/test").join(package);

    for relative in ["Cargo.toml", "src/lib.rs", "canic.toml"] {
        println!(
            "cargo:rerun-if-changed={}",
            package_root.join(relative).display()
        );
    }
}

fn prepared_wasm_path(
    outer_target_dir: &std::path::Path,
    target: &str,
    profile: &str,
    package: &str,
) -> PathBuf {
    outer_target_dir
        .join(target)
        .join(profile)
        .join(format!("{package}.wasm"))
}

// Walk up from the current manifest until the workspace Cargo.toml is found.
fn discover_workspace_root(manifest_dir: &std::path::Path) -> PathBuf {
    for candidate in manifest_dir.ancestors() {
        let cargo_toml = candidate.join("Cargo.toml");
        if !cargo_toml.is_file() {
            continue;
        }

        let cargo_toml_text = fs::read_to_string(&cargo_toml)
            .unwrap_or_else(|err| panic!("read {} failed: {err}", cargo_toml.display()));

        if cargo_toml_text.contains("[workspace]") {
            return candidate.to_path_buf();
        }
    }

    panic!(
        "unable to discover workspace root from {}; expected an ancestor Cargo.toml with [workspace]",
        manifest_dir.display()
    );
}
