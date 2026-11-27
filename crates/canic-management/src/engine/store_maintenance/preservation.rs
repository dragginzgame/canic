//! One read per continuation. Root retains a cursor and digest, never Store's catalog or bytes.
use super::{Call, Failure, Phase, Status, Wait, change, store, target};
use canic_management_protocol::{store_maintenance::Preservation, stored::STORE_LIMITS};

pub(super) fn validate(record: &Status) {
    if let Some(value) = record.preservation.as_ref() {
        assert!(!matches!(record.phase, Phase::Reserved | Phase::Closing));
        validate_scan(value);
        if record.phase != Phase::Closed {
            assert!(value.complete, "scan must finish before stop");
        }
    }
    if let Some(installation) = &record.installation {
        let Some(value) = installation.verification.as_ref() else {
            return;
        };
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
    assert!(value.after.is_some() || !value.after_removed);
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
    let method = store::PRESERVATION;
    let request = store::preservation::Request {
        attempt: record.approval.attempt.clone(),
        after: scan(record).as_ref().and_then(|value| value.after.clone()),
    };
    let bytes = ic_wire::encode(
        &request,
        ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Request,
        },
        STORE_LIMITS,
    )
    .expect("preservation request");
    let reply = Call::new(
        target(&record.approval),
        "observe_preservation",
        Wait::Bounded { seconds: 5 },
    )
    .owned_args(bytes)
    .execute()
    .await
    .map_err(super::super::failure)
    .and_then(super::super::response)?;
    let encoding = ic_wire::Encoding::Frame {
        method,
        direction: ic_wire::Direction::Response,
    };
    let page = ic_wire::decode::<Result<store::preservation::Page, store::Error>>(
        &reply,
        encoding,
        STORE_LIMITS,
    )
    .map_err(|_| Failure::InvalidReply)?
    .map_err(|_| Failure::Mismatch)?;
    if !page.catalog.quota.valid()
        || page.catalog.releases > page.catalog.quota.releases
        || page.catalog.reserved_bytes > page.catalog.quota.reserved_bytes
        || page.catalog.artifacts > page.catalog.releases
    {
        return Err(Failure::Mismatch);
    }
    let mut value = scan(record)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| Preservation {
            catalog: page.catalog.clone(),
            after: None,
            observed: 0,
            digest: store::digest(b"Canic closed Store metadata v2"),
            complete: false,
            after_removed: false,
        });
    if page.catalog != value.catalog {
        return Err(Failure::Mismatch);
    }
    if let Some(entry) = &page.entry {
        let release = &entry.manifest().release;
        if !entry.manifest().valid()
            || matches!(entry, store::preservation::Entry::Active(custody) if custody.pins.len() > store::MAX_PINS)
            || value.observed >= value.catalog.releases
            || value.after.as_ref().is_some_and(|old| {
                (old.name.as_str(), value.after_removed, old.version.as_str()) >= entry.order_key()
            })
        {
            return Err(Failure::Mismatch);
        }
        let bytes = ic_wire::encode(
            &(value.digest, entry),
            ic_wire::Encoding::Payload,
            STORE_LIMITS,
        )
        .map_err(|_| Failure::InvalidReply)?;
        value.digest = store::digest(&bytes);
        value.after = Some(release.clone());
        value.after_removed = matches!(entry, store::preservation::Entry::Removed(_));
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
    let value = Some(value);
    // Concurrent responses to the same cursor may commit it only once. A later page must win.
    change(&record.approval, record.phase, |current| {
        if scan(current) == scan(record) {
            if record.phase == Phase::Verifying {
                if value.as_ref().is_some_and(|v| v.complete) {
                    current.phase = Phase::Verified;
                }
                current
                    .installation
                    .as_mut()
                    .expect("issued install")
                    .verification = value;
            } else {
                current.preservation = value;
            }
            current.failure = None;
        }
    });
    Ok(())
}
