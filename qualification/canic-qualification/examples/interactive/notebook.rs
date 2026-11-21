//! One local snapshot, restored under a distinct recovery generation.
use super::{Result, run_prompt};
use canic::{
    method::{Contract, LIMITS},
    recovery::Command,
};
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};
use ic_wire::{Direction, Encoding};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Recovery {
    Live,
    Restored,
    Accepted,
    Reopened,
}

fn frame(name: &str, direction: Direction) -> Encoding {
    Encoding::Frame {
        method: ic_wire::method_id(name),
        direction,
    }
}

async fn recover(
    pic: &PocketIc,
    id: CanisterId,
    installer: CanisterId,
    command: Command,
) -> Result<()> {
    let bytes = pic
        .update_call(
            id,
            installer,
            Command::NAME,
            Command::encode_request(&command)?,
        )
        .await
        .map_err(|reject| reject.reject_message)?;
    if Command::decode_reply(&bytes)? != command {
        return Err("unexpected recovery acknowledgement".into());
    }
    Ok(())
}

pub(super) async fn session(pic: &PocketIc, wasm: &Path, seconds: u64) -> Result<()> {
    let installer = CanisterId::self_authenticating(b"notebook installer");
    let visitor = CanisterId::self_authenticating(b"notebook visitor");
    let id = pic
        .create_canister_with_settings(Some(installer), None)
        .await;
    pic.add_cycles(id, 2_000_000_000_000).await;
    pic.install_canister(id, std::fs::read(wasm)?, vec![], Some(installer))
        .await;
    let hint = "Commands: edit TEXT, read, checkpoint, restore, accept, reopen, exit";
    println!(
        "Notebook {id}. {hint}\nTry edit draft, checkpoint, edit published, restore, read, accept, reopen."
    );
    let mut snapshot = None;
    let mut recovery = Recovery::Live;
    run_prompt("notebook", seconds, async |line: &str| {
        if let Some(text) = line.strip_prefix("edit ") {
            let args = ic_wire::encode(&text, frame("edit", Direction::Request), LIMITS)?;
            let bytes = pic
                .update_call(id, visitor, "edit", args)
                .await
                .map_err(|reject| match recovery {
                    Recovery::Restored | Recovery::Accepted => format!(
                        "Edit refused while checkpoint is fenced ({:?}).",
                        reject.error_code
                    ),
                    _ => reject.reject_message,
                })?;
            ic_wire::decode::<()>(&bytes, frame("edit", Direction::Response), LIMITS)?;
            return Ok("Saved.".into());
        }
        match line {
            "read" => {
                let bytes = pic
                    .query_call(
                        id,
                        visitor,
                        "read",
                        ic_wire::encode(&(), frame("read", Direction::Request), LIMITS)?,
                    )
                    .await
                    .map_err(|reject| reject.reject_message)?;
                Ok(ic_wire::decode(
                    &bytes,
                    frame("read", Direction::Response),
                    LIMITS,
                )?)
            }
            "checkpoint" if snapshot.is_none() => {
                recover(pic, id, installer, Command::Fence(1)).await?;
                pic.stop_canister(id, Some(installer))
                    .await
                    .map_err(|reject| reject.reject_message)?;
                let result = pic.take_canister_snapshot(id, Some(installer), None).await;
                pic.start_canister(id, Some(installer))
                    .await
                    .map_err(|reject| reject.reject_message)?;
                snapshot = Some(result.map_err(|reject| reject.reject_message)?.id);
                recover(pic, id, installer, Command::Open(1)).await?;
                Ok("Saved fenced checkpoint 1. Live edits are open.".into())
            }
            "checkpoint" => Ok("This session already has its checkpoint.".into()),
            "restore" if recovery == Recovery::Live => {
                let saved = snapshot.as_ref().ok_or("create a checkpoint first")?;
                pic.stop_canister(id, Some(installer))
                    .await
                    .map_err(|reject| reject.reject_message)?;
                let result = pic
                    .load_canister_snapshot(id, Some(installer), saved.clone())
                    .await;
                pic.start_canister(id, Some(installer))
                    .await
                    .map_err(|reject| reject.reject_message)?;
                result.map_err(|reject| reject.reject_message)?;
                recovery = Recovery::Restored;
                Ok("Restored checkpoint 1. Reads are available. Edits are fenced.".into())
            }
            "accept" if recovery == Recovery::Restored => {
                recover(
                    pic,
                    id,
                    installer,
                    Command::Accept {
                        checkpoint: 1,
                        generation: 2,
                    },
                )
                .await?;
                recovery = Recovery::Accepted;
                Ok("Accepted recovery generation 2. Edits are still fenced.".into())
            }
            "reopen" if recovery == Recovery::Accepted => {
                recover(pic, id, installer, Command::Open(2)).await?;
                recovery = Recovery::Reopened;
                Ok("Reopened generation 2 for edits.".into())
            }
            "accept" | "reopen" | "restore" => {
                Ok("Recovery order: restore, accept, reopen (once per session).".into())
            }
            _ => Ok(hint.into()),
        }
    })
    .await
}
