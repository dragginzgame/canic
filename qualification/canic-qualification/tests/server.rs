//! Canic's explicit simulator admission, ownership and cleanup boundary.

use ic_testkit::{
    pocket_ic::PocketIcBuilder,
    server::{Deletion, Error, Limits, PreparedServer, Server},
};
use std::{net::TcpStream, time::Duration};

fn limits() -> Limits {
    Limits {
        startup: Duration::from_secs(30),
        lifetime: Duration::from_secs(60),
        stdout_bytes: 1024 * 1024,
        stderr_bytes: 1024 * 1024,
    }
}

#[test]
fn missing_or_malformed_assets_never_start_or_download() {
    use std::os::unix::fs::PermissionsExt;
    let directory = canic_qualification::server_directory()
        .parent()
        .unwrap()
        .join(format!("refused-assets-{}", std::process::id()));
    assert!(!directory.exists());
    assert!(PreparedServer::check(&directory).is_err());
    assert!(
        !directory.exists(),
        "offline checking cannot provision assets"
    );
    std::fs::create_dir(&directory).unwrap();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    let binary = directory.join("pocket-ic");
    let bytes = b"#!/bin/sh\nexit 0\n";
    std::fs::write(&binary, bytes).unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(PreparedServer::check(&directory).is_err());
    assert_eq!(std::fs::read(&binary).unwrap(), bytes);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn startup_deadline_retains_joined_bounded_failure() {
    let prepared = PreparedServer::check(&canic_qualification::server_directory()).unwrap();
    let mut bounds = limits();
    bounds.startup = Duration::from_nanos(1);
    let Err(Error::Startup { evidence, .. }) = Server::start(prepared, bounds) else {
        panic!("startup must refuse an elapsed readiness deadline");
    };
    assert!(evidence.process.cleanup.is_empty(), "{evidence:?}");
    assert!(evidence.output.stdout.len() <= bounds.stdout_bytes);
    assert!(evidence.output.stderr.len() <= bounds.stderr_bytes);
    assert_eq!(evidence.abandoned, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn instances_are_deleted_before_server_stop() {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        limits(),
    )
    .unwrap();
    let address = server
        .endpoint()
        .strip_prefix("http://")
        .unwrap()
        .trim_end_matches('/')
        .to_owned();
    let first = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let second = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    assert_ne!(first.client().instance_id, second.client().instance_id);
    let canister = first.client().create_canister().await;
    assert!(second.client().get_subnet(canister).await.is_none());
    assert!(matches!(server.stop(), Err(Error::LiveInstances)));
    assert_eq!(
        first.close(Duration::from_secs(10)).await,
        Deletion::Attempted
    );
    assert!(matches!(server.stop(), Err(Error::LiveInstances)));
    assert_eq!(
        second.close(Duration::from_secs(10)).await,
        Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
    assert_eq!(stopped.abandoned, 0);
    // The upstream deletion result proves attempts only. This separately observes
    // that the owned server listener is closed after process teardown.
    assert!(TcpStream::connect(address).is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn abandoned_instance_is_reported_and_last_owner_stops_process() {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        limits(),
    )
    .unwrap();
    let address = server
        .endpoint()
        .strip_prefix("http://")
        .unwrap()
        .trim_end_matches('/')
        .to_owned();
    let instance = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    drop(instance);
    let stopped = server.stop().unwrap();
    assert_eq!(stopped.abandoned, 1);
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
    assert!(TcpStream::connect(&address).is_err());

    let server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        limits(),
    )
    .unwrap();
    let address = server
        .endpoint()
        .strip_prefix("http://")
        .unwrap()
        .trim_end_matches('/')
        .to_owned();
    let instance = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    drop(server);
    assert!(
        TcpStream::connect(&address).is_ok(),
        "live instance retains its server"
    );
    drop(instance);
    assert!(
        TcpStream::connect(address).is_err(),
        "fallback stops the last owner's process"
    );
}
