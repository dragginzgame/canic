use super::*;
use canic_wasm_store_protocol::CHUNK_BYTES;
use ic_storage::VectorMemory;

fn fixture(version: &str, bytes: usize) -> (Manifest, Vec<u8>) {
    let mut payload = vec![42; bytes];
    payload[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    (
        Manifest {
            release: Release {
                name: "app".into(),
                version: version.into(),
            },
            hash: digest(&payload),
            bytes: bytes as u64,
            chunks: payload.chunks(CHUNK_BYTES).map(digest).collect(),
        },
        payload,
    )
}
fn upload(state: &mut State<VectorMemory>, manifest: &Manifest, payload: &[u8]) {
    for (index, bytes) in payload.chunks(CHUNK_BYTES).enumerate() {
        state
            .put(&Put {
                release: manifest.release.clone(),
                index: index as u32,
                bytes: bytes.to_vec(),
            })
            .unwrap();
    }
}

#[test]
fn interrupted_upload_reopens_multiple_versions_without_duplicate_payloads() {
    let backing = VectorMemory::default();
    let mut state = State::open(backing.clone(), true);
    let (first, bytes) = fixture("1.0.0", 5 * CHUNK_BYTES + 31);
    state.prepare(&first).unwrap();
    state
        .put(&Put {
            release: first.release.clone(),
            index: 0,
            bytes: bytes[..CHUNK_BYTES].to_vec(),
        })
        .unwrap();
    let partial = state.status(&first.release).unwrap();
    assert!(!partial.published);
    assert_eq!(state.publish(&first.release), Err(Error::Incomplete));
    assert_eq!(
        state.read(&Read {
            release: first.release.clone(),
            index: 0
        }),
        Err(Error::Incomplete)
    );
    drop(state);
    let mut state = State::open(backing.clone(), false);
    assert_eq!(state.prepare(&first).unwrap(), partial);
    upload(&mut state, &first, &bytes);
    let published = state.publish(&first.release).unwrap();
    assert!(published.published);
    assert!(published.missing.is_empty());
    // Lost successful replies are handled by exact idempotent publication, not new identity.
    assert_eq!(state.publish(&first.release).unwrap(), published);
    let usage = state.info();
    let second = Manifest {
        release: Release {
            version: "2.0.0".into(),
            ..first.release.clone()
        },
        ..first.clone()
    };
    assert!(state.prepare(&second).unwrap().published);
    assert_eq!(state.info().reserved_bytes, usage.reserved_bytes);
    assert_eq!(state.info().chunks, usage.chunks);
    let (third, changed) = fixture("3.0.0", CHUNK_BYTES + 17);
    state.prepare(&third).unwrap();
    upload(&mut state, &third, &changed);
    state.publish(&third.release).unwrap();
    drop(state);
    let state = State::open(backing, false);
    assert_eq!(state.list(None).unwrap().len(), 3);
    assert_eq!(
        state
            .read(&Read {
                release: first.release,
                index: 5
            })
            .unwrap()
            .0,
        bytes[5 * CHUNK_BYTES..]
    );
}

#[test]
fn conflicts_hashes_and_quotas_refuse_without_changing_custody() {
    let mut state = State::open(VectorMemory::default(), true);
    let (manifest, bytes) = fixture("1", 37);
    state
        .configure(&Quota {
            reserved_bytes: 37,
            releases: 1,
        })
        .unwrap();
    state.prepare(&manifest).unwrap();
    let before = state.info();
    let mut conflict = manifest.clone();
    conflict.hash[0] ^= 1;
    assert_eq!(state.prepare(&conflict), Err(Error::Conflict));
    conflict = manifest.clone();
    conflict.chunks[0][0] ^= 1;
    assert_eq!(state.prepare(&conflict), Err(Error::Conflict));
    assert_eq!(
        state.put(&Put {
            release: manifest.release.clone(),
            index: 0,
            bytes: vec![7; 37]
        }),
        Err(Error::Hash)
    );
    assert_eq!(state.info(), before);
    let (second, _) = fixture("2", 38);
    assert_eq!(state.prepare(&second), Err(Error::Capacity));
    assert_eq!(
        state.configure(&Quota {
            reserved_bytes: 36,
            releases: 1
        }),
        Err(Error::Capacity)
    );
    upload(&mut state, &manifest, &bytes);
    state.publish(&manifest.release).unwrap();
    state
        .configure(&Quota {
            reserved_bytes: 75,
            releases: 2,
        })
        .unwrap();
    state.prepare(&second).unwrap();
    assert_eq!(state.info().reserved_bytes, 75);
    assert!(state.status(&manifest.release).unwrap().published);
}

#[test]
fn full_digest_and_geometry_are_checked_before_publication() {
    let mut state = State::open(VectorMemory::default(), true);
    let (mut manifest, bytes) = fixture("bad-hash", 10);
    manifest.hash[0] ^= 1;
    state.prepare(&manifest).unwrap();
    upload(&mut state, &manifest, &bytes);
    assert_eq!(state.publish(&manifest.release), Err(Error::Hash));
    assert!(!state.status(&manifest.release).unwrap().published);
    let invalid = Manifest {
        bytes: u64::MAX,
        ..manifest.clone()
    };
    assert_eq!(state.prepare(&invalid), Err(Error::Invalid));
    let invalid = Release {
        name: "app\0other".into(),
        version: "1".into(),
    };
    assert_eq!(state.status(&invalid), Err(Error::Invalid));
}

#[test]
fn history_pages_are_bounded_and_keep_all_versions() {
    let mut state = State::open(VectorMemory::default(), true);
    for i in 0..20 {
        let (manifest, _) = fixture(&format!("{i:02}"), 10);
        state.prepare(&manifest).unwrap();
    }
    let mut after = None;
    let mut seen = Vec::new();
    loop {
        let page = state.list(after.as_ref()).unwrap();
        assert!(page.len() <= PAGE_SIZE);
        if page.is_empty() {
            break;
        }
        after = Some(page.last().unwrap().manifest.release.clone());
        seen.extend(
            page.into_iter()
                .map(|status| status.manifest.release.version),
        );
    }
    assert_eq!(seen, (0..20).map(|i| format!("{i:02}")).collect::<Vec<_>>());
    assert_eq!(state.info().artifacts, 1);
    assert_eq!(state.info().reserved_bytes, 10);
}

#[test]
fn pins_keep_exact_owner_operation_and_release_across_reopen() {
    let backing = VectorMemory::default();
    let mut state = State::open(backing.clone(), true);
    let (manifest, bytes) = fixture("1", 10);
    let pin = Pin {
        operation: [1; 32],
        manifest: manifest.clone(),
    };
    state.prepare(&manifest).unwrap();
    assert_eq!(state.pin(&[1], &pin), Err(Error::Incomplete));
    upload(&mut state, &manifest, &bytes);
    state.publish(&manifest.release).unwrap();
    assert_eq!(state.observe_pin(&[1], &pin), Err(Error::NotFound));
    let receipt = state.pin(&[1], &pin).unwrap();
    assert_eq!(state.pin(&[1], &pin).unwrap(), receipt);
    assert_eq!(state.observe_pin(&[2], &pin), Err(Error::NotFound));
    let other = Manifest {
        release: Release {
            version: "2".into(),
            ..manifest.release.clone()
        },
        ..manifest
    };
    state.prepare(&other).unwrap();
    assert_eq!(
        state.pin(
            &[1],
            &Pin {
                manifest: other,
                ..pin.clone()
            }
        ),
        Err(Error::Conflict)
    );
    drop(state);
    let state = State::open(backing, false);
    assert_eq!(state.observe_pin(&[1], &pin).unwrap(), receipt);
}

#[test]
fn pin_capacity_never_evicts_and_old_artifact_schema_gains_no_authority() {
    use ic_storage::adapter::Storable;
    use std::borrow::Cow;
    #[derive(serde::Serialize)]
    struct Legacy {
        bytes: u64,
        chunks: Vec<[u8; 32]>,
        published: bool,
    }
    let old = Legacy {
        bytes: 8,
        chunks: vec![[3; 32]],
        published: true,
    };
    let mut encoded = b"CWA1".to_vec();
    encoded.extend(ic_wire::native::encode(&old, canic_wasm_store_protocol::LIMITS).unwrap());
    let decoded = Artifact::from_bytes(Cow::Owned(encoded)).unwrap();
    assert!(decoded.published);
    assert!(decoded.pins.is_empty());
    let round_trip = Artifact::from_bytes(decoded.to_bytes().unwrap()).unwrap();
    assert!(round_trip.pins.is_empty());

    let mut state = State::open(VectorMemory::default(), true);
    let (mut manifest, bytes) = fixture(&"v".repeat(64), 10);
    manifest.release.name = "n".repeat(64);
    state.prepare(&manifest).unwrap();
    upload(&mut state, &manifest, &bytes);
    state.publish(&manifest.release).unwrap();
    for i in 0..MAX_PINS {
        let pin = Pin {
            operation: [i as u8 + 1; 32],
            manifest: manifest.clone(),
        };
        state.pin(&[255; 29], &pin).unwrap();
    }
    let first = Pin {
        operation: [1; 32],
        manifest: manifest.clone(),
    };
    assert!(state.pin(&[255; 29], &first).is_ok());
    assert_eq!(
        state.pin(
            &[255; 29],
            &Pin {
                operation: [255; 32],
                manifest
            }
        ),
        Err(Error::Capacity)
    );
    assert!(state.observe_pin(&[255; 29], &first).is_ok());
    let mut maximum = state.artifacts.get(&first.manifest.hash).unwrap();
    maximum.bytes = (CHUNK_BYTES * canic_wasm_store_protocol::MAX_CHUNKS) as u64;
    maximum.chunks = vec![[255; 32]; canic_wasm_store_protocol::MAX_CHUNKS];
    let encoded = maximum.to_bytes_checked().unwrap();
    let restored = Artifact::from_bytes(encoded).unwrap();
    assert_eq!(restored.pins.len(), MAX_PINS);
    assert_eq!(restored.chunks, maximum.chunks);
}
