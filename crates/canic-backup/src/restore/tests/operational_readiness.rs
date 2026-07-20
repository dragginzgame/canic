//! Module: restore::tests::operational_readiness
//!
//! Responsibility: prove initial restore-document publication across process death.
//! Does not own: restore execution effects or the frozen protocol inventory.
//! Boundary: binds restore planning and journaling to the durable create authority.

use super::*;
use crate::{
    operational_readiness::manifest::assert_case_defined,
    persistence::{BackupLayout, create_json_durable_at_barriers, read_json},
    test_support::{hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier, temp_dir},
};

use std::{fs, path::Path, process::Command};

const CHILD_ROOT_ENV: &str = "CANIC_TEST_RESTORE_PUBLICATION_ROOT";
const CHILD_DOCUMENT_ENV: &str = "CANIC_TEST_RESTORE_PUBLICATION_DOCUMENT";
const CHILD_BARRIER_ENV: &str = "CANIC_TEST_RESTORE_PUBLICATION_BARRIER";
const CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_RESTORE_PUBLICATION_HANDSHAKE";

#[test]
fn initial_restore_documents_survive_process_death_on_both_write_sides() {
    let Some(root) = std::env::var_os(CHILD_ROOT_ENV) else {
        for (document, barrier, case_id) in [
            (
                "plan",
                "before-publication",
                "CANIC-094-R01/restore-plan-publication/before-durable-write",
            ),
            (
                "plan",
                "after-directory-sync",
                "CANIC-094-R01/restore-plan-publication/after-durable-write",
            ),
            (
                "journal",
                "before-publication",
                "CANIC-094-R02/apply-journal-publication/before-durable-write",
            ),
            (
                "journal",
                "after-directory-sync",
                "CANIC-094-R02/apply-journal-publication/after-durable-write",
            ),
        ] {
            assert_case_defined(case_id);
            prove_initial_restore_document_publication(document, barrier);
        }
        return;
    };

    let root = std::path::PathBuf::from(root);
    let document = std::env::var(CHILD_DOCUMENT_ENV).expect("restore document kind");
    let barrier = std::env::var(CHILD_BARRIER_ENV).expect("restore publication barrier");
    let handshake_root = std::path::PathBuf::from(
        std::env::var_os(CHILD_HANDSHAKE_ENV).expect("restore publication handshake root"),
    );
    let (plan, journal) = prepared_restore_documents(&root);
    match document.as_str() {
        "plan" => publish_document_at_barrier(
            &root.join("restore-plan.json"),
            &plan,
            &barrier,
            &handshake_root,
        ),
        "journal" => publish_document_at_barrier(
            &root.join("restore-apply-journal.json"),
            &journal,
            &barrier,
            &handshake_root,
        ),
        _ => panic!("unsupported restore publication document: {document}"),
    }
    panic!("restore publication child passed its armed barrier");
}

fn publish_document_at_barrier<T: serde::Serialize>(
    path: &Path,
    document: &T,
    barrier: &str,
    handshake_root: &Path,
) {
    create_json_durable_at_barriers(
        path,
        document,
        || {
            if barrier == "before-publication" {
                hold_at_acknowledged_barrier(handshake_root);
            }
        },
        || {
            if barrier == "after-directory-sync" {
                hold_at_acknowledged_barrier(handshake_root);
            }
        },
    )
    .expect("publish restore document in crash child");
}

fn prove_initial_restore_document_publication(document: &str, barrier: &str) {
    let root = temp_dir(&format!("canic-restore-{document}-{barrier}"));
    let handshake_root = temp_dir(&format!("canic-restore-handshake-{document}-{barrier}"));
    fs::create_dir_all(&handshake_root).expect("create restore publication handshake root");
    publish_restore_fixture(&root);
    let (expected_plan, expected_journal) = prepared_restore_documents(&root);
    let plan_path = root.join("restore-plan.json");
    let journal_path = root.join("restore-apply-journal.json");

    if document == "journal" {
        create_or_adopt_restore_plan(&plan_path, &expected_plan)
            .expect("publish prerequisite restore plan");
    }

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "restore::tests::operational_readiness::initial_restore_documents_survive_process_death_on_both_write_sides",
            "--nocapture",
        ])
        .env(CHILD_ROOT_ENV, &root)
        .env(CHILD_DOCUMENT_ENV, document)
        .env(CHILD_BARRIER_ENV, barrier)
        .env(CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn restore publication child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);

    if document == "plan" {
        assert!(!journal_path.exists());
        if barrier == "before-publication" {
            assert!(!plan_path.exists());
        } else {
            assert_eq!(
                read_json::<RestorePlan>(&plan_path).expect("read published restore plan"),
                expected_plan
            );
        }
        create_or_adopt_restore_plan(&plan_path, &expected_plan)
            .expect("restart publishes or adopts exact restore plan");
    } else if barrier == "before-publication" {
        assert!(!journal_path.exists());
    } else {
        assert_eq!(
            read_json::<RestoreApplyJournal>(&journal_path)
                .expect("read published restore apply journal"),
            expected_journal
        );
    }

    assert_eq!(
        read_json::<RestorePlan>(&plan_path).expect("restart reads exact restore plan"),
        expected_plan
    );
    create_or_adopt_restore_apply_journal(&journal_path, &expected_journal)
        .expect("restart publishes or adopts exact restore journal");
    let recovered_journal = read_json::<RestoreApplyJournal>(&journal_path)
        .expect("restart reads exact restore journal");

    assert_eq!(recovered_journal, expected_journal);
    assert_eq!(recovered_journal.pending_operations, 0);
    assert_eq!(recovered_journal.completed_operations, 0);
    assert_eq!(recovered_journal.failed_operations, 0);
    assert!(recovered_journal.operation_receipts.is_empty());

    fs::remove_dir_all(root).expect("remove restore publication fixture");
    fs::remove_dir_all(handshake_root).expect("remove restore publication handshake root");
}

fn publish_restore_fixture(root: &Path) {
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(&mut manifest, ROOT, root, "artifacts/root", b"root state A");
    set_member_artifact(
        &mut manifest,
        CHILD,
        root,
        "artifacts/app",
        b"child state A",
    );
    BackupLayout::new(root.to_path_buf())
        .publish_manifest(&manifest)
        .expect("publish restore fixture manifest");
}

fn prepared_restore_documents(root: &Path) -> (RestorePlan, RestoreApplyJournal) {
    let manifest = BackupLayout::new(root.to_path_buf())
        .read_manifest()
        .expect("read restore fixture manifest");
    let plan = RestorePlanner::plan(&manifest, None).expect("build restore fixture plan");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, root)
        .expect("validate restore fixture artifacts");
    let journal =
        RestoreApplyJournal::from_dry_run(&dry_run).expect("build restore fixture apply journal");
    (plan, journal)
}
