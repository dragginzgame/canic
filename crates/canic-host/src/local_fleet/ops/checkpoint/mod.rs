//! Bounded checkpoint acknowledgement and exact owned-child cleanup.

use crate::local_fleet::{LocalFleetError, ops::runtime::guarded};
use ic_testkit::{pic::PocketIcManagedServer, pocket_ic::PocketIc};
use std::{io::Read, sync::mpsc, time::Duration};

/// Save the instance and confirm its terminal server status before recording success.
pub fn save(
    mut pic: PocketIc,
    server: &mut Option<PocketIcManagedServer>,
    timeout: Duration,
) -> Result<(), LocalFleetError> {
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let result = guarded(|| {
            pic.stop_live();
            let id = pic.instance_id();
            let url = pic.get_server_url();
            // The upstream destructor issues DELETE without validating its response.
            // This explicit request confirms the maintained REST checkpoint boundary.
            let client = reqwest::blocking::Client::builder()
                .no_proxy()
                .timeout(timeout)
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|error| LocalFleetError::Platform(error.to_string()))?;
            let endpoint = url
                .join(&format!("instances/{id}"))
                .map_err(|_| LocalFleetError::Identity)?;
            client
                .delete(endpoint)
                .send()
                .and_then(reqwest::blocking::Response::error_for_status)
                .map_err(|error| LocalFleetError::Platform(error.to_string()))?;
            drop(pic);
            let mut response = client
                .get(
                    url.join("instances")
                        .map_err(|_| LocalFleetError::Identity)?,
                )
                .send()
                .and_then(reqwest::blocking::Response::error_for_status)
                .map_err(|error| LocalFleetError::Platform(error.to_string()))?;
            let mut bytes = Vec::new();
            response.by_ref().take(65_537).read_to_end(&mut bytes)?;
            if bytes.len() > 65_536 {
                return Err(LocalFleetError::Capacity);
            }
            let states: Vec<String> = serde_json::from_slice(&bytes)?;
            if states.get(id).map(String::as_str) != Some("Deleted") {
                return Err(LocalFleetError::UncleanCheckpoint);
            }
            Ok(())
        })
        .and_then(std::convert::identity);
        let _ = sender.send(result);
    });
    if let Ok(result) = receiver.recv_timeout(timeout) {
        let _ = worker.join();
        result
    } else {
        // Terminating this exact owned child also releases blocked client IO.
        drop(server.take());
        if worker.is_finished() {
            let _ = worker.join();
        }
        Err(LocalFleetError::CheckpointTimeout)
    }
}
