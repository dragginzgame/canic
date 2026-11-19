//! IC entrypoints for the persistent counter.

use crate::{CounterMethod, Error, Request, Snapshot, read, write};
use canic::recovery::Command;
use ic_platform_runtime::system as api;

// Expose the counter’s current fence and readiness.
canic::mount_readiness_endpoint!(crate::readiness);

// Create the counter on install and strictly reopen it on upgrade.
canic::mount_lifecycle! {
    install = crate::init,
    reopen = crate::post_upgrade,
}

// counter
// Read or increment the counter, or change its fence as the recorded owner.
// limits : default
#[canic::update(contract = CounterMethod)]
fn counter(request: Request) -> Result<Snapshot, Error> {
    let mut record = read();
    record.apply(request, api::caller())?;
    let snapshot = record.snapshot();
    if request != Request::Read {
        write(&record);
    }

    Ok(snapshot)
}

// recovery
// Apply a recovery command as the counter’s recorded owner.
// limits : canic::recovery::LIMITS
#[canic::update(contract = Command)]
fn recovery(command: Command) -> Command {
    let mut record = read();
    let caller = api::caller();
    assert_eq!(record.owner, caller, "counter recovery owner");
    match command {
        Command::Fence(attempt) => record
            .apply(Request::Fence { attempt }, caller)
            .expect("counter fence"),
        Command::Open(attempt) => {
            assert_eq!(record.fence, Some(attempt), "counter opening generation");
            record
                .apply(Request::Activate { attempt }, caller)
                .expect("counter opening");
        }
        Command::Accept {
            checkpoint,
            generation,
        } => {
            assert_eq!(record.fence, Some(checkpoint), "counter checkpoint");
            assert!(generation > record.last_attempt, "fresh counter generation");
            record.last_attempt = generation;
            record.fence = Some(generation);
        }
    }
    write(&record);

    command
}
