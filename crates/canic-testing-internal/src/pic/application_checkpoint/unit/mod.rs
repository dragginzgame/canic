//! Installed complete-unit acknowledgement and restoration proof.
//! The driver retains canonical Root custody; no production Root orchestration is implied.

use super::{
    Binding, Error, Phase, PocketIc, Principal, RootCheckpointPhase, RootCheckpointRecord,
    authority, increment, issue, release, restore, retained, transition, view,
};
use canic_core::shared_support::application_checkpoint_root::RootCheckpointError;
use ic_testkit::pic::{CanisterSnapshotTarget, PocketIcSnapshotExt, SnapshotRestoreFunding};

pub fn qualify(fixture: &super::super::ManagedLifecycleFixture, mut targets: [Principal; 2]) {
    let pic = &fixture.pic;
    let root = fixture.root;
    targets.sort();
    for target in targets {
        pic.add_cycles(target, super::super::SNAPSHOT_RESTORE_MINIMUM_CYCLES);
        assert_eq!(increment(pic, target), Ok(1));
    }
    let authorities = targets.map(|target| authority(pic, target, root)).to_vec();
    let mut issued = issue(None, authorities.clone(), [11; 32]);
    let bindings: Vec<_> = issued
        .participants()
        .iter()
        .map(|p| p.binding().clone())
        .collect();
    for (target, binding) in targets.into_iter().zip(&bindings) {
        assert_eq!(
            transition(pic, target, root, binding, Phase::Preparing),
            Ok(Phase::Preparing)
        );
        assert_eq!(
            transition(pic, target, root, binding, Phase::Prepared),
            Ok(Phase::Prepared)
        );
        assert_eq!(increment(pic, target), Err(invalid()));
    }
    // Both applications are frozen, but the second reply has been discarded.
    issued = retained(&issued.acknowledge(&bindings[0], Phase::Prepared).unwrap());
    assert_eq!(issued.phase(), RootCheckpointPhase::Preparing);
    assert!(!issued.membership_mutation_allowed());
    assert_eq!(
        RootCheckpointRecord::begin(
            Some(&issued),
            &authorities[0].issuer,
            1,
            [12; 32],
            [7; 32],
            authorities.clone()
        ),
        Err(RootCheckpointError::ActiveCheckpoint)
    );
    assert_eq!(
        transition(pic, targets[1], root, &bindings[1], Phase::Prepared),
        Ok(Phase::Prepared)
    );
    issued = retained(&issued.acknowledge(&bindings[1], Phase::Prepared).unwrap());
    assert_eq!(issued.phase(), RootCheckpointPhase::Prepared);
    let prepared = targets.map(|target| view(pic, target));
    assert!(prepared.iter().all(|state| state.prepare_runs == 1));
    // The complete Root acknowledgement predicate precedes this real batch capture.
    let snapshots = pic
        .capture_snapshots(targets.map(|canister_id| CanisterSnapshotTarget {
            canister_id,
            sender: None,
        }))
        .unwrap();
    issued = release_unit(pic, targets, root, &bindings, issued);
    for target in targets {
        assert_eq!(increment(pic, target), Ok(2));
    }
    pic.restore_snapshots(
        &snapshots,
        SnapshotRestoreFunding::TopUpTo {
            minimum_cycles: super::super::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
        },
    )
    .unwrap();
    for (target, before) in targets.into_iter().zip(&prepared) {
        assert_eq!(&view(pic, target), before);
        assert_eq!(increment(pic, target), Err(invalid()));
    }
    restore_unit(pic, targets, root, &bindings, issued);
}

fn restore_unit(
    pic: &PocketIc,
    targets: [Principal; 2],
    root: Principal,
    bindings: &[Binding],
    mut issued: RootCheckpointRecord,
) {
    let authorities = bindings
        .iter()
        .map(|b| b.authority.clone())
        .collect::<Vec<_>>();
    assert_eq!(issued.phase(), RootCheckpointPhase::Released);
    assert_eq!(issued.participants()[0].binding().sequence, 1);
    issued = issue(Some(&issued), authorities.clone(), [12; 32]);
    let fresh: Vec<_> = issued
        .participants()
        .iter()
        .map(|p| p.binding().clone())
        .collect();
    for ((target, captured), fresh) in targets.into_iter().zip(bindings).zip(&fresh) {
        assert_eq!(fresh.sequence, 2);
        let unchanged = view(pic, target);
        let mut wrong = captured.clone();
        wrong.unit_digest = [8; 32];
        assert_eq!(restore(pic, target, root, &wrong, fresh), Err(invalid()));
        assert_eq!(view(pic, target), unchanged);
        assert_eq!(
            restore(pic, target, root, captured, fresh),
            Ok(Phase::Preparing)
        );
        assert_eq!(
            transition(pic, target, root, fresh, Phase::Prepared),
            Ok(Phase::Prepared)
        );
        issued = retained(&issued.acknowledge(fresh, Phase::Prepared).unwrap());
    }
    assert_eq!(issued.phase(), RootCheckpointPhase::Prepared);
    issued = release_unit(pic, targets, root, &fresh, issued);
    // Exact terminal replay is a local journal projection, with no remote dispatch.
    let terminal = targets.map(|target| view(pic, target));
    assert_eq!(
        retained(
            &RootCheckpointRecord::begin(
                Some(&issued),
                &authorities[0].issuer,
                1,
                [12; 32],
                [7; 32],
                authorities.clone()
            )
            .unwrap()
        ),
        issued
    );
    assert_eq!(targets.map(|target| view(pic, target)), terminal);
    for (target, stale) in targets.into_iter().zip(bindings) {
        let completed = view(pic, target);
        assert_eq!(completed.prepare_runs, 2);
        assert_eq!(completed.release_runs, 1);
        assert_eq!(
            transition(pic, target, root, stale, Phase::Preparing),
            Err(invalid())
        );
        assert_eq!(increment(pic, target), Ok(2));
    }
}

fn release_unit(
    pic: &PocketIc,
    targets: [Principal; 2],
    root: Principal,
    bindings: &[Binding],
    issued: RootCheckpointRecord,
) -> RootCheckpointRecord {
    let mut issued = retained(&issued.release());
    release(pic, targets[0], root, &bindings[0]);
    issued = retained(&issued.acknowledge(&bindings[0], Phase::Released).unwrap());
    assert_eq!(issued.phase(), RootCheckpointPhase::Releasing);
    assert!(!issued.membership_mutation_allowed());
    assert_eq!(increment(pic, targets[1]), Err(invalid()));
    // The second application commits release; lose its response and recover Root.
    release(pic, targets[1], root, &bindings[1]);
    let completed = targets.map(|target| view(pic, target));
    issued = retained(&issued);
    assert_eq!(issued.phase(), RootCheckpointPhase::Releasing);
    for (target, binding) in targets.into_iter().zip(bindings) {
        assert_eq!(
            transition(pic, target, root, binding, Phase::Releasing),
            Ok(Phase::Released)
        );
        assert_eq!(
            transition(pic, target, root, binding, Phase::Released),
            Ok(Phase::Released)
        );
    }
    assert_eq!(targets.map(|target| view(pic, target)), completed);
    issued = retained(&issued.acknowledge(&bindings[1], Phase::Released).unwrap());
    assert_eq!(issued.phase(), RootCheckpointPhase::Released);
    assert!(issued.membership_mutation_allowed());
    issued
}

const fn invalid() -> Error {
    Error::from_registered(canic::diagnostics::codes::AUTHORITY_INVALID_STATE)
}
