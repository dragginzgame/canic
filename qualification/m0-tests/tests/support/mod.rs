use ic_testkit::{
    pic::{PocketIcInstance, PocketIcStartupConfig, server_assets},
    pocket_ic::PocketIcBuilder,
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub fn inputs() -> (Vec<u8>, PathBuf) {
    let wasm = std::fs::read(
        std::env::var_os("CANIC_M0_PROBE_WASM")
            .expect("run make m0-test to select the Cargo-built Wasm"),
    )
    .expect("read probe Wasm");
    let directory = std::env::var_os("IC_TESTKIT_SERVER_DIRECTORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .expect("repository root")
                .join("target/m0/server")
        });
    let binary = server_assets::check(&directory).expect("run make m0-setup first");
    (wasm, binary)
}

pub async fn instance(binary: &Path) -> PocketIcInstance {
    PocketIcStartupConfig::Spawn {
        server_binary: binary.to_path_buf(),
        timeout: Duration::from_secs(30),
        server_hard_ttl: Some(Duration::from_secs(120)),
        server_idle_ttl: None,
        server_output_files: None,
    }
    .build_instance(
        PocketIcBuilder::new()
            .with_max_request_time_ms(Some(30_000))
            .with_application_subnet(),
    )
    .await
    .unwrap()
}
