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
