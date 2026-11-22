//! Stable receiver installed through the public Canic mount.
use crate::ProjectError;
use canic::auth::{
    install::Install,
    session::{Authorization, stable},
};
use ic_storage::{DefaultMemoryImpl, MemoryRuntime};

const READ_SCOPE: &str = "read";
canic::mount_auth_session!(read = READ_SCOPE);
#[cfg(not(feature = "changed-login-limit"))]
canic::mount_auth_lifecycle!(
    storage = open,
    init = on_init,
    pre_upgrade = crate::issuer::seal,
    post_upgrade = on_post_upgrade,
);

#[cfg(feature = "changed-login-limit")]
canic::mount_auth_lifecycle!(
    storage = open,
    max_login_secs = 600,
    init = on_init,
    pre_upgrade = crate::issuer::seal,
    post_upgrade = on_post_upgrade,
);

thread_local! {
    static PROJECT_WRITES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static RESTORED: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

fn on_init(input: &Install) {
    assert_eq!(input.storage, canic::auth::install::Storage::Stable);
    RESTORED.set(Some(false));
}

fn on_post_upgrade() {
    RESTORED.set(Some(true));
}

// restored
// Report which lifecycle callback completed in this incarnation.
// guard  : installed issuer
// limits : default
#[canic::query(guard = canic::guard::require_issuer)]
fn restored() -> Option<bool> {
    RESTORED.get()
}

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
    let auth = canic::auth::session::authorize("write")?;
    if let Some(barrier) = barrier {
        hold(barrier).await;
        auth.reauthorize()?;
    }
    Ok((auth.issuer(), auth.subject()))
}

async fn hold(barrier: ic_platform_types::Principal) {
    use ic_platform_runtime::call::{Call, Outcome, Wait};
    let reply = Call::new(barrier, "hold", Wait::Bounded { seconds: 300 })
        .args(&[])
        .execute()
        .await
        .expect("qualification barrier");
    assert!(matches!(reply.into_parts().0, Outcome::Reply(bytes) if bytes.is_empty()));
}

// application_user_id
// Return the application binding from the admitted session.
// scope  : read
// limits : default Wire limits for the fixture ID
#[canic::query(scope = "read")]
fn application_user_id() -> Option<Vec<u8>> {
    auth.user_id().map(<[u8]>::to_vec)
}

fn project_owner(auth: &Authorization) -> Result<(), ProjectError> {
    (auth.subject() == ic_platform_types::Principal::from_slice(&[2]))
        .then_some(())
        .ok_or(ProjectError::Forbidden)
}

// project_count
// Read the project's write count as its verified owner.
// scope  : read, then project_owner
// limits : default Wire budgets
#[canic::query(scope = "read", scope_guard = project_owner)]
fn project_count() -> Result<u32, ProjectError> {
    Ok(PROJECT_WRITES.get())
}

// project_write
// Record one write as the verified project owner.
// scope  : write, then project_owner
// limits : default Wire budgets
#[canic::update(scope = "write", scope_guard = project_owner)]
async fn project_write(barrier: Option<ic_platform_types::Principal>) -> Result<u32, ProjectError> {
    if let Some(barrier) = barrier {
        hold(barrier).await;
        auth.reauthorize().map_err(|_| ProjectError::Forbidden)?;
    }
    PROJECT_WRITES.update(|n| n + 1);
    Ok(PROJECT_WRITES.get())
}
