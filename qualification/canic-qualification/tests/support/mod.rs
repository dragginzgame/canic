use ic_testkit::pocket_ic::PocketIcBuilder;
use ic_testkit::server::{Instance, Limits, PreparedServer, Server};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub fn inputs() -> (Vec<u8>, PathBuf) {
    let wasm = std::fs::read(
        std::env::var_os("CANIC_EFFECT_PROBE_WASM")
            .expect("run make effect-test to select the Cargo-built Wasm"),
    )
    .expect("read probe Wasm");
    (wasm, canic_qualification::server_directory())
}

pub async fn instance(directory: &Path) -> (Server, Instance) {
    let server = Server::start(
        PreparedServer::check(directory).expect("run make qualification-setup first"),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(120),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let instance = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    (server, instance)
}
