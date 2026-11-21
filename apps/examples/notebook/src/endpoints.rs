//! Public note operations and installer-authorized recovery.

use crate::{MAX_BYTES, read_note, write_note};
use canic::recovery::Command;

canic::mount_lifecycle! {
    install = crate::install,
    reopen = crate::reopen,
}

// read
// Read the public note even while writes are fenced.
// limits : default
#[canic::query]
fn read() -> String {
    read_note().text
}

// edit
// Replace the public note with at most 256 UTF-8 bytes.
// limits : default
#[canic::update]
fn edit(text: String) {
    assert!(text.len() <= MAX_BYTES, "note exceeds 256 UTF-8 bytes");
    let mut note = read_note();
    assert!(!note.fenced, "notebook fenced");
    note.text = text;
    write_note(&note);
}

// recovery
// Apply the recorded owner's recovery command and acknowledge its fence transition.
// limits : canic::recovery::LIMITS
#[canic::update(contract = Command)]
fn recovery(command: Command) -> Command {
    let mut note = read_note();
    note.recover(command);
    write_note(&note);
    command
}
