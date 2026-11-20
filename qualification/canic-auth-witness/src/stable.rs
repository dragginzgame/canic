//! Stable receiver installed through the public Canic mount.
use canic::auth::{install::Install, session::stable};
use ic_storage::{DefaultMemoryImpl, MemoryRuntime};

canic::mount_auth_session!(read = "read");
canic::mount_auth_lifecycle!(stable = open, seal = crate::issuer::seal);

fn open(input: Option<&Install>) -> MemoryRuntime<DefaultMemoryImpl> {
    let runtime = canic::storage::bootstrap(
        DefaultMemoryImpl::default(),
        input.is_some(),
        "auth",
        &[
            stable::ENGINE_KEY,
            stable::HOST_KEY,
            crate::issuer::KEY,
            crate::issuer::HOST_KEY,
        ],
        320,
    );
    #[cfg(feature = "damaged-checkpoint")]
    if input.is_none() {
        use ic_storage::StableKey;
        runtime
            .open_memory(&StableKey::parse(stable::ENGINE_KEY).unwrap())
            .unwrap()
            .write(0, &[0; 16]);
    }
    if let Some(input) = input {
        let storage: canic::auth::install::Storage = ic_wire::decode(
            &input.application,
            ic_wire::Encoding::Payload,
            crate::LIMITS,
        )
        .expect("issuer storage choice");
        assert_eq!(
            storage,
            canic::auth::install::Storage::Stable,
            "stable issuer fixture"
        );
    }
    crate::issuer::open(&runtime, input.is_some());
    runtime
}

// Exercise a fallible application coordinator outside a scoped endpoint.
#[canic::update]
async fn coordinate_write(
    barrier: Option<ic_platform_types::Principal>,
) -> Result<
    (ic_platform_types::Principal, ic_platform_types::Principal),
    canic::auth::protocol::model::SessionReadRefusal,
> {
    use ic_platform_runtime::call::{Call, Outcome, Wait};
    let auth = canic::auth::session::authorize("write")?;
    if let Some(barrier) = barrier {
        let reply = Call::new(barrier, "hold", Wait::Bounded { seconds: 300 })
            .args(&[])
            .execute()
            .await
            .expect("qualification barrier");
        assert!(matches!(reply.into_parts().0, Outcome::Reply(bytes) if bytes.is_empty()));
        auth.reauthorize()?;
    }
    Ok((auth.issuer(), auth.subject()))
}
