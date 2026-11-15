use super::*;
use canic_wasm_store_protocol::{AdmissionCommand as Command, MaintenanceAttempt};
use ic_storage::{VectorMemory, adapter::Storable};
use std::borrow::Cow;

#[test]
fn admission_preserves_closed_attempt_and_fences_replays_and_cache_continuations() {
    let backing = VectorMemory::default();
    let mut state = State::open(backing.clone(), true);
    let payload = b"\0asm\x01\0\0\0";
    let manifest = Manifest {
        release: Release {
            name: "app".into(),
            version: "one".into(),
        },
        bytes: 8,
        hash: digest(payload),
        chunks: vec![digest(payload)],
    };
    state.prepare(&manifest).unwrap();
    let put = Put {
        release: manifest.release.clone(),
        index: 0,
        bytes: payload.to_vec(),
    };
    state.put(&put).unwrap();
    state.publish(&manifest.release).unwrap();
    let pin = Pin {
        operation: [9; 32],
        manifest: manifest.clone(),
    };
    state.pin(&[1], &pin).unwrap();
    let custody = state.custody(&manifest.release).unwrap();
    let info = state.info();
    let old_cache = state.admission_epoch().unwrap();
    let first = MaintenanceAttempt {
        sequence: 1,
        operation: [1; 32],
    };
    let second = MaintenanceAttempt {
        sequence: 2,
        operation: [2; 32],
    };
    assert_eq!(
        state.admission(&[1], &Command::Close(second.clone())),
        Err(Error::Conflict)
    );
    let closed = state
        .admission(&[1], &Command::Close(first.clone()))
        .unwrap()
        .unwrap();
    assert!(closed.closed);
    assert_eq!(state.prepare(&manifest), Err(Error::Closed));
    assert_eq!(state.put(&put), Err(Error::Closed));
    assert_eq!(state.publish(&manifest.release), Err(Error::Closed));
    assert_eq!(state.pin(&[1], &pin), Err(Error::Closed));
    assert_eq!(state.configure(&info.quota), Err(Error::Closed));
    assert_eq!(state.cache_fence(old_cache), Err(Error::Closed));
    assert_eq!(state.info(), info);
    assert_eq!(state.custody(&manifest.release).unwrap(), custody);
    assert_eq!(
        state
            .read(&Read {
                release: manifest.release.clone(),
                index: 0
            })
            .unwrap()
            .0,
        payload
    );
    assert!(state.observe_pin(&[1], &pin).is_ok());
    drop(state);
    let mut state = State::open(backing, false);
    assert_eq!(
        state.admission(&[1], &Command::Observe).unwrap(),
        Some(closed)
    );
    assert_eq!(
        state.admission(&[2], &Command::Activate(first.clone())),
        Err(Error::Conflict)
    );
    assert_eq!(
        state.admission(&[1], &Command::Close(second.clone())),
        Err(Error::Conflict)
    );
    let active = state
        .admission(&[1], &Command::Activate(first.clone()))
        .unwrap()
        .unwrap();
    assert!(!active.closed);
    assert_eq!(
        state
            .admission(&[1], &Command::Close(first.clone()))
            .unwrap(),
        Some(active)
    );
    assert_eq!(state.cache_fence(old_cache), Err(Error::Conflict));
    assert!(state.cache_fence(state.admission_epoch().unwrap()).is_ok());
    state.configure(&info.quota).unwrap(); // Quota changes cannot reset the sequence.
    state
        .admission(&[1], &Command::Close(second.clone()))
        .unwrap();
    assert_eq!(
        state.admission(&[1], &Command::Activate(first)),
        Err(Error::Conflict)
    );
    assert!(
        state
            .admission(&[1], &Command::Observe)
            .unwrap()
            .unwrap()
            .closed
    );
    state.admission(&[1], &Command::Activate(second)).unwrap();
    assert_eq!(state.custody(&manifest.release).unwrap(), custody);
    // Exhaustion blocks successors rather than wrapping into an old authority epoch.
    let mut settings = state.settings.get().clone();
    settings.admission.as_mut().unwrap().attempt.sequence = u64::MAX;
    state.settings.set(settings);
    assert_eq!(
        state.admission(
            &[1],
            &Command::Close(MaintenanceAttempt {
                sequence: 1,
                operation: [3; 32]
            })
        ),
        Err(Error::Conflict)
    );
}

#[test]
fn admission_settings_migrate_only_the_recognized_previous_schema() {
    #[derive(serde::Serialize)]
    struct Previous {
        quota: Quota,
        reserved_bytes: u64,
    }
    let previous = Previous {
        quota: Quota::default(),
        reserved_bytes: 8,
    };
    let mut bytes = b"CWS1".to_vec();
    bytes.extend(
        ic_wire::encode(
            &previous,
            ic_wire::Encoding::Payload,
            canic_wasm_store_protocol::LIMITS,
        )
        .unwrap(),
    );
    let mut migrated = Settings::from_bytes(Cow::Owned(bytes)).unwrap();
    assert_eq!(migrated.reserved_bytes, 8);
    assert!(migrated.admission.is_none());
    migrated.admission = Some(canic_wasm_store_protocol::Admission {
        attempt: MaintenanceAttempt {
            sequence: 1,
            operation: [1; 32],
        },
        owner: vec![1],
        closed: true,
    });
    let encoded = migrated.to_bytes().unwrap();
    assert!(encoded.starts_with(b"CWS2"));
    let decoded = Settings::from_bytes(encoded).unwrap();
    assert_eq!(decoded.admission, migrated.admission);
    for corrupt in [b"CWS1bad".as_slice(), b"CWS2bad", b"CWS3bad"] {
        assert!(Settings::from_bytes(Cow::Borrowed(corrupt)).is_err());
    }
    migrated.admission.as_mut().unwrap().attempt.sequence = 0;
    assert!(migrated.to_bytes().is_err());
}
