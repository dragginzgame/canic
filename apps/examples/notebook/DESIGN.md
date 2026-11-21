# Notebook

Notebook holds one public scratch note of at most 256 UTF-8 bytes. Anyone can read and edit it while writes are open. Only the principal that installed the canister can change its recovery fence, using Canic’s existing recovery command contract.

## Try the installed example

Prepare the local simulator once, then run the walkthrough:

```sh
make qualification-setup
make demo DEMO_APP=notebook
```

At the prompt, enter `edit draft`, `checkpoint`, `edit published`, `restore` and `read`. Try an edit before and after `accept`: both attempts are refused. Use `reopen` and edit again. Each session has one checkpoint and one restore, with a simulated installer for recovery and a separate visitor for reads and edits.

The regression (`make notebook-test`) installs Notebook with a named local identity and synthetic cycles. It writes “draft”, fences writes, upgrades the canister and snapshots it while stopped. It then reopens writes, changes the text to “published” and restores the snapshot. “Draft” is readable immediately. Accepting generation 2 keeps edits fenced. Opening generation 1 fails, and only the installer can open generation 2 and allow the next edit.

The note, installer and fence share the stable record `notebook.record.v1`. This layout requires a fresh install for anyone running the earlier text-only experiment. Reopening its `notebook.note.v1` layout fails before writes are admitted.

## Managed Backup follow-up

The installed example exercises the application boundary directly. A managed fleet must install Notebook through Root so Root is its recovery authority. Root retains the recovery generation and operation evidence outside the restored child. It must prevent replay of the checkpoint’s original opening before acceptance and mediate separate operator acceptance and reopening.

Extend the installed Backup signer journey to interrupt delivery of a Notebook restore result, resume observation of that original attempt and show one restore. The fleet view should identify the affected notebook, snapshot operation and current fence.
