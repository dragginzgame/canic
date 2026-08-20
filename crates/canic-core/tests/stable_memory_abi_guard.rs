// Category C - System-level artifact test (no embedded config).

use std::{
    fs,
    path::{Path, PathBuf},
};

const CANIC_MANAGED_RUNTIME_CRATES: &[&str] = &[
    "canic",
    "canic-core",
    "canic-control-plane",
    "canic-macros",
    "canic-wasm-store",
];

#[test]
fn canic_managed_runtime_code_uses_managed_explicit_stable_keys() {
    let workspace_root = workspace_root();
    let mut violations = Vec::new();

    for crate_name in CANIC_MANAGED_RUNTIME_CRATES {
        scan_dir(
            &workspace_root.join("crates").join(crate_name).join("src"),
            &mut violations,
        );
    }

    assert!(
        violations.is_empty(),
        "Canic-managed runtime code must not bypass the managed explicit-key ABI: {violations:?}"
    );
}

fn scan_dir(root: &Path, violations: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, violations);
            continue;
        }

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        if is_managed_memory_runtime_boundary(&path) {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };

        if contents.starts_with("#![cfg(test)]") {
            continue;
        }

        if has_forbidden_memory_pattern(&contents) {
            violations.push(path);
        }
    }
}

fn has_forbidden_memory_pattern(contents: &str) -> bool {
    const FORBIDDEN: &[&str] = &[
        "ic_memory!(",
        "MemoryApi::register(",
        "MemoryApi::register_with_key(",
        "MEMORY_MANAGER",
        "MemoryManager::init",
        "RestrictedMemory",
        "stable_read(",
        "stable_write(",
        "stable_grow(",
        "stable_size(",
    ];

    FORBIDDEN.iter().any(|pattern| contents.contains(pattern))
}

#[test]
fn managed_memory_guard_matches_calls_without_rejecting_observation_names() {
    assert!(has_forbidden_memory_pattern("let pages = stable_grow(1);"));
    assert!(has_forbidden_memory_pattern(
        "let pages = ic_cdk::api::stable::stable_size();"
    ));
    assert!(!has_forbidden_memory_pattern(
        "let pages = memory.maximum_stable_growth_pages();"
    ));
}

fn is_managed_memory_runtime_boundary(path: &Path) -> bool {
    path.to_string_lossy()
        .contains("/crates/canic-core/src/memory/")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}

// B1 contract fixture for the planned schema-1 runtime-whitelist record.
// This is deliberately test-only until the owning 0.107 baseline is accepted.
mod runtime_whitelist_b1_contract {
    use candid::{CandidType, Principal};
    use serde::{Deserialize, Serialize};

    const MAX_PRINCIPALS: usize = 256;
    const MAX_STATUS_PAGE_ENTRIES: usize = 128;
    const MAX_STABLE_RECORD_BYTES: usize = 32 * 1_024;
    const MAX_COMMAND_INGRESS_BYTES: usize = 16 * 1_024;

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
    enum MutationOutcomeRecordFixture {
        Added,
        AlreadyAbsent,
        AlreadyPresent,
        Removed,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    struct MutationResultRecordFixture {
        outcome: MutationOutcomeRecordFixture,
        principal: Principal,
        revision: u64,
        membership_digest: [u8; 32],
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    struct OperationRecordFixture {
        operation_id: [u8; 32],
        request_hash: [u8; 32],
        result: MutationResultRecordFixture,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    struct RuntimeWhitelistRecordFixture {
        schema_version: u32,
        principals: Vec<Principal>,
        revision: u64,
        membership_digest: [u8; 32],
        last_operation: Option<OperationRecordFixture>,
    }

    #[derive(CandidType, Deserialize)]
    struct RuntimeWhitelistMutationRequestFixture {
        principal: Principal,
        expected_revision: u64,
        operation_id: [u8; 32],
    }

    #[derive(CandidType, Deserialize)]
    struct PrincipalPageFixture {
        entries: Vec<Principal>,
        total: u64,
    }

    #[derive(CandidType, Deserialize)]
    struct RuntimeWhitelistStatusFixture {
        principals: PrincipalPageFixture,
        revision: u64,
        membership_digest: [u8; 32],
        maximum_principals: u16,
    }

    #[test]
    fn runtime_whitelist_b1_stable_and_candid_bounds_are_measured() {
        let principals = (0..MAX_PRINCIPALS).map(principal).collect::<Vec<_>>();
        let stable_bytes = [
            MutationOutcomeRecordFixture::Added,
            MutationOutcomeRecordFixture::AlreadyAbsent,
            MutationOutcomeRecordFixture::AlreadyPresent,
            MutationOutcomeRecordFixture::Removed,
        ]
        .into_iter()
        .map(|outcome| {
            canic_core::cdk::serialize::serialize(&RuntimeWhitelistRecordFixture {
                schema_version: 1,
                principals: principals.clone(),
                revision: u64::MAX,
                membership_digest: [0xff; 32],
                last_operation: Some(OperationRecordFixture {
                    operation_id: [0xfe; 32],
                    request_hash: [0xfd; 32],
                    result: MutationResultRecordFixture {
                        outcome,
                        principal: principals[MAX_PRINCIPALS - 1],
                        revision: u64::MAX,
                        membership_digest: [0xfc; 32],
                    },
                }),
            })
            .expect("runtime-whitelist fixture CBOR")
        })
        .max_by_key(Vec::len)
        .expect("one outcome fixture");

        let status = RuntimeWhitelistStatusFixture {
            principals: PrincipalPageFixture {
                entries: principals[..MAX_STATUS_PAGE_ENTRIES].to_vec(),
                total: MAX_PRINCIPALS as u64,
            },
            revision: u64::MAX,
            membership_digest: [0xfb; 32],
            maximum_principals: MAX_PRINCIPALS as u16,
        };
        let status_bytes = candid::encode_one(&status).expect("runtime-whitelist status Candid");
        let request_bytes = candid::encode_one(RuntimeWhitelistMutationRequestFixture {
            principal: principals[MAX_PRINCIPALS - 1],
            expected_revision: u64::MAX,
            operation_id: [0xfa; 32],
        })
        .expect("runtime-whitelist mutation Candid");

        assert_eq!(stable_bytes.len(), 8_417);
        assert_eq!(status_bytes.len(), 4_072);
        assert_eq!(request_bytes.len(), 101);
        assert!(stable_bytes.len() <= MAX_STABLE_RECORD_BYTES);
        assert!(status_bytes.len() <= MAX_COMMAND_INGRESS_BYTES);
        assert!(request_bytes.len() <= MAX_COMMAND_INGRESS_BYTES);
    }

    fn principal(index: usize) -> Principal {
        let mut bytes = [0_u8; 29];
        bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
        bytes[8..].fill(u8::try_from(index % 251).expect("bounded fixture byte"));
        Principal::from_slice(&bytes)
    }
}
