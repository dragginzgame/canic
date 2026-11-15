//! One read per continuation. Root retains a cursor and digest, never Store's catalog or bytes.
use super::{Call, Failure, Phase, Status, WaitPolicy, change, store, target};
use canic_management_protocol::store_maintenance::Preservation;

pub(super) fn validate(record: &Status) {
    if let Some(value) = &record.preservation {
        assert!(!matches!(record.phase, Phase::Reserved | Phase::Closing));
        validate_scan(value);
        if record.phase != Phase::Closed {
            assert!(value.complete, "scan must finish before stop");
        }
    }
    if let Some(value) = record
        .installation
        .as_ref()
        .and_then(|i| i.verification.as_ref())
    {
        assert!(matches!(
            record.phase,
            Phase::Verifying | Phase::Verified | Phase::ActivateIssued | Phase::Ready
        ));
        validate_scan(value);
    }
}
fn validate_scan(value: &Preservation) {
    assert!(value.catalog.quota.valid());
    assert!(value.observed <= value.catalog.releases);
    assert!(value.catalog.releases <= value.catalog.quota.releases);
    assert!(value.catalog.reserved_bytes <= value.catalog.quota.reserved_bytes);
    assert!(value.catalog.artifacts <= value.catalog.releases);
    assert_eq!(value.after.is_some(), value.observed != 0);
    assert!(value.after.as_ref().is_none_or(store::Release::valid));
    if value.complete {
        assert_eq!(value.observed, value.catalog.releases);
    }
}

fn scan(record: &Status) -> &Option<Preservation> {
    if record.phase == Phase::Verifying {
        &record
            .installation
            .as_ref()
            .expect("issued install")
            .verification
    } else {
        &record.preservation
    }
}

pub(super) async fn capture(record: &Status) -> Result<(), Failure> {
    let request = store::preservation::Request {
        attempt: record.approval.attempt.clone(),
        after: scan(record).as_ref().and_then(|value| value.after.clone()),
    };
    let bytes = ic_wire::encode(
        &request,
        ic_wire::Encoding::Frame {
            method: store::PRESERVATION,
            direction: ic_wire::Direction::Request,
        },
        store::LIMITS,
    )
    .expect("preservation request");
    let reply = Call::new(
        target(&record.approval),
        "observe_preservation",
        WaitPolicy::Bounded { timeout_seconds: 5 },
    )
    .take_raw_args(bytes)
    .execute()
    .await
    .map_err(super::super::failure)?;
    let page: Result<store::preservation::Page, store::Error> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: store::PRESERVATION,
            direction: ic_wire::Direction::Response,
        },
        store::LIMITS,
    )
    .map_err(|_| Failure::InvalidReply)?;
    let page = page.map_err(|_| Failure::Mismatch)?;
    if !page.catalog.quota.valid()
        || page.catalog.releases > page.catalog.quota.releases
        || page.catalog.reserved_bytes > page.catalog.quota.reserved_bytes
        || page.catalog.artifacts > page.catalog.releases
    {
        return Err(Failure::Mismatch);
    }
    let mut value = scan(record).clone().unwrap_or_else(|| Preservation {
        catalog: page.catalog.clone(),
        after: None,
        observed: 0,
        digest: store::digest(b"Canic closed Store metadata v1"),
        complete: false,
    });
    if page.catalog != value.catalog {
        return Err(Failure::Mismatch);
    }
    if let Some(entry) = &page.entry {
        let release = &entry.status.manifest.release;
        if !entry.status.manifest.valid()
            || entry.pins.len() > store::MAX_PINS
            || value.observed >= value.catalog.releases
            || value
                .after
                .as_ref()
                .is_some_and(|old| (&old.name, &old.version) >= (&release.name, &release.version))
        {
            return Err(Failure::Mismatch);
        }
        let bytes = ic_wire::encode(
            &(value.digest, entry),
            ic_wire::Encoding::Payload,
            store::LIMITS,
        )
        .map_err(|_| Failure::InvalidReply)?;
        value.digest = store::digest(&bytes);
        value.after = Some(release.clone());
        value.observed += 1;
    } else {
        if value.observed != value.catalog.releases {
            return Err(Failure::Mismatch);
        }
        value.complete = true;
    }
    if record.phase == Phase::Verifying
        && value.complete
        && record.preservation.as_ref() != Some(&value)
    {
        return Err(Failure::Mismatch);
    }
    // Concurrent responses to the same cursor may commit it only once. A later page must win.
    change(&record.approval, record.phase, |current| {
        if scan(current) == scan(record) {
            if record.phase == Phase::Verifying {
                if value.complete {
                    current.phase = Phase::Verified;
                }
                current
                    .installation
                    .as_mut()
                    .expect("issued install")
                    .verification = Some(value);
            } else {
                current.preservation = Some(value);
            }
            current.failure = None;
        }
    });
    Ok(())
}
