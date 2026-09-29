//! Verify structured protocol evidence and retained source-document presence.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const RECORD: &str = include_str!("../../../docs/contracts/blob-storage-protocol-evidence.json");
const GATE: &str = "scripts/ci/check-blob-storage-protocol-evidence.sh";
const DOCUMENTS: [&str; 2] = [
    "docs/contracts/BLOB_STORAGE_INVENTORY.md",
    "docs/contracts/BLOB_STORAGE_CASHIER_INVENTORY.md",
];

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("canic-protocol-{}-{nonce}", std::process::id()));
        fs::create_dir_all(root.join("docs/contracts")).unwrap();
        for document in DOCUMENTS {
            fs::write(root.join(document), "Source notes may be edited freely.\n").unwrap();
        }
        Self(root)
    }

    fn accepts(&self, record: &str) -> bool {
        let path = self.0.join("evidence.json");
        fs::write(&path, record).unwrap();
        Command::new("bash")
            .arg(workspace_root().join(GATE))
            .arg(path)
            .current_dir(&self.0)
            .output()
            .unwrap()
            .status
            .success()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn accepted_evidence_is_independent_of_inventory_wording() {
    let fixture = Fixture::new();
    assert!(fixture.accepts(RECORD));
    fs::write(
        fixture.0.join(DOCUMENTS[1]),
        "Reject an empty result and retain the accepted principals.\n",
    )
    .unwrap();
    assert!(fixture.accepts(RECORD));
}

#[test]
fn evidence_requires_exact_methods_modes_and_provenance() {
    let fixture = Fixture::new();
    for (before, after) in [
        ("account_balance_get_v1", "unregistered_method"),
        ("account_balance_get_v1", "account_top_up_v1"),
        ("9ca150b396a2bde42f2b8977a04a7ca2c6172b56", "unresolved"),
        ("\"query\"", "\"update\""),
        ("\"consumer\"", "\"endpoint\""),
        ("reject_preserve_previous", "accept_empty"),
        (
            "fleets/toko/project/hub/src/ops/blob_storage.rs",
            "../outside.rs",
        ),
        ("canic.blob_storage_protocol_evidence.v1", "unknown"),
    ] {
        assert!(RECORD.contains(before));
        assert!(
            !fixture.accepts(&RECORD.replace(before, after)),
            "accepted invalid {before}"
        );
    }
}

#[test]
fn evidence_requires_retained_source_documents() {
    let fixture = Fixture::new();
    for document in DOCUMENTS {
        fs::remove_file(fixture.0.join(document)).unwrap();
        assert!(!fixture.accepts(RECORD));
        fs::write(fixture.0.join(document), "Retained source notes.").unwrap();
    }
}
