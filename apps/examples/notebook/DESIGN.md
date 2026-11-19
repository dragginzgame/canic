# Notebook

**Planned.** Demonstrate operator-triggered Backup and the application's recovery fence with a single persistent note. Build on the installed Backup counter qualification. The notebook owns the note and write admission. Root retains the recovery generation and operation evidence outside the restored child and mediates separate operator acceptance and reopening.

1. Write “draft”, capture a snapshot, then change the note to “published”.
2. Restore the original snapshot. Read “draft” and attempt an edit. The write must be refused.
3. Accept the matching recovery generation. Verify writes are still fenced. Explicitly reopen that generation and edit successfully.
4. Interrupt delivery of a restore result. Resume observation of the original attempt. Show one restore and reject a stale reopening request.

The fleet view should let the operator identify the affected notebook, snapshot operation and current fence. Use local synthetic cycles for the walkthrough.
