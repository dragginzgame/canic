use std::collections::HashMap;

use candid::Principal;

use super::{ControllerSnapshots, Pic};

impl Pic {
    /// Capture one restorable snapshot per canister using a shared controller.
    pub fn capture_controller_snapshots<I>(
        &self,
        controller_id: Principal,
        canister_ids: I,
    ) -> Option<ControllerSnapshots>
    where
        I: IntoIterator<Item = Principal>,
    {
        let mut snapshots = HashMap::new();

        for canister_id in canister_ids {
            let Some(snapshot) = self.try_take_controller_snapshot(controller_id, canister_id)
            else {
                eprintln!(
                    "capture_controller_snapshots: snapshot capture unavailable for {canister_id}"
                );
                return None;
            };
            snapshots.insert(canister_id, snapshot);
        }

        Some(ControllerSnapshots::new(snapshots))
    }

    /// Restore a previously captured snapshot set using the same controller.
    pub fn restore_controller_snapshots(
        &self,
        controller_id: Principal,
        snapshots: &ControllerSnapshots,
    ) {
        for (canister_id, snapshot_id, sender) in snapshots.iter() {
            self.restore_controller_snapshot(controller_id, canister_id, sender, snapshot_id);
        }
    }

    // Capture one snapshot with sender fallbacks that match controller ownership.
    fn try_take_controller_snapshot(
        &self,
        controller_id: Principal,
        canister_id: Principal,
    ) -> Option<(Vec<u8>, Option<Principal>)> {
        let candidates = controller_sender_candidates(controller_id, canister_id);
        let mut last_err = None;

        for sender in candidates {
            match self.take_canister_snapshot(canister_id, sender, None) {
                Ok(snapshot) => return Some((snapshot.id, sender)),
                Err(err) => last_err = Some((sender, err)),
            }
        }

        if let Some((sender, err)) = last_err {
            eprintln!(
                "failed to capture canister snapshot for {canister_id} using sender {sender:?}: {err}"
            );
        }
        None
    }

    // Restore one snapshot with sender fallbacks that match controller ownership.
    fn restore_controller_snapshot(
        &self,
        controller_id: Principal,
        canister_id: Principal,
        snapshot_sender: Option<Principal>,
        snapshot_id: &[u8],
    ) {
        let fallback_sender = if snapshot_sender.is_some() {
            None
        } else {
            Some(controller_id)
        };
        let candidates = [snapshot_sender, fallback_sender];
        let mut last_err = None;

        for sender in candidates {
            match self.load_canister_snapshot(canister_id, sender, snapshot_id.to_vec()) {
                Ok(()) => return,
                Err(err) => last_err = Some((sender, err)),
            }
        }

        let (sender, err) =
            last_err.expect("snapshot restore must have at least one sender attempt");
        panic!(
            "failed to restore canister snapshot for {canister_id} using sender {sender:?}: {err}"
        );
    }
}

// Prefer the likely controller sender first to reduce noisy management-call failures.
fn controller_sender_candidates(
    controller_id: Principal,
    canister_id: Principal,
) -> [Option<Principal>; 2] {
    if canister_id == controller_id {
        [None, Some(controller_id)]
    } else {
        [Some(controller_id), None]
    }
}
