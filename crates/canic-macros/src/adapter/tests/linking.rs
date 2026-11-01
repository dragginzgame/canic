//! Link two consumer crates with overlapping IC methods and lifecycle entrypoints.

use crate::adapter::{EntryKind, export_entrypoint};
use quote::{format_ident, quote};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn native_entrypoints_link_across_consuming_crates() {
    let directory = std::env::temp_dir().join(format!(
        "canic-native-link-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("Cargo.toml"), "[workspace]\nmembers=[\"role-a\",\"role-b\",\"consumer\"]\nresolver=\"3\"\n[profile.dev]\ncodegen-units=1\n").unwrap();
    let mut wrappers = String::from("fn run(task: impl FnOnce()) { task(); }\n");
    for (kind, method, name) in [
        (EntryKind::Query, "shared", "query_entry"),
        (EntryKind::Update, "shared", "update_entry"),
        (EntryKind::Init, "", "init_entry"),
        (EntryKind::PostUpgrade, "", "upgrade_entry"),
    ] {
        let generated = export_entrypoint(
            &kind,
            method,
            &format_ident!("{name}"),
            quote!(run),
            quote!(),
        );
        wrappers.push_str(&generated.to_string());
        wrappers.push('\n');
    }
    for (name, value) in [("role-a", 1), ("role-b", 2)] {
        let role = directory.join(name);
        fs::create_dir_all(role.join("src")).unwrap();
        fs::write(
            role.join("Cargo.toml"),
            format!("[package]\nname={name:?}\nversion=\"0.0.0\"\nedition=\"2024\"\n"),
        )
        .unwrap();
        fs::write(
            role.join("src/lib.rs"),
            format!("{wrappers}\npub fn value() -> u8 {{ {value} }}\n"),
        )
        .unwrap();
    }
    let consumer = directory.join("consumer");
    fs::create_dir_all(consumer.join("src")).unwrap();
    fs::write(consumer.join("Cargo.toml"), "[package]\nname=\"native-link-consumer\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[dependencies]\nrole-a={path=\"../role-a\"}\nrole-b={path=\"../role-b\"}\n").unwrap();
    fs::write(
        consumer.join("src/main.rs"),
        "fn main() { assert_eq!(role_a::value(), 1); assert_eq!(role_b::value(), 2); }",
    )
    .unwrap();
    let output = Command::new(env!("CARGO"))
        .current_dir(&directory)
        .args(["run", "--offline", "--package", "native-link-consumer"])
        .env("CARGO_TARGET_DIR", directory.join("target"))
        .env_remove("CARGO_BUILD_TARGET")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}; fixture {}",
        String::from_utf8_lossy(&output.stderr),
        directory.display()
    );
    fs::remove_dir_all(directory).unwrap();
}
