# Encrypted backup and recovery guide

Version 0.6.0 source implements manual encrypted local backups and verified
whole-workspace restore. Earlier 0.3.0 downloads do not contain these controls.
The application remains a development build for fictional records; independent
security review and clean Windows 11 offline acceptance are still outstanding.

## Create a backup

1. Unlock your workspace and open **Settings**.
2. Under **Encrypted workspace backup**, choose **Choose backup folder**. Select
   an external drive or another existing folder outside the application's data
   folder. You can also enter the folder's full path.
3. Enter your current workspace password and choose **Create encrypted backup**.
4. Wait for **Encrypted backup saved.** The app displays the dated filename.
5. Safely eject removable storage. Keep multiple backups and keep at least one
   separate from the computer containing the active workspace.

The file contains all clients, notes, budgets, debt/savings/cash-flow plans, and
existing audit history. It is a single `.ebcbackup` file. Each backup has a new
name; existing files are never replaced. The file is encrypted and authenticated.
The password is cleared from the form before the operation and is not stored in
the file. You must retain that password separately and securely.

A saved file with an audit warning is still a valid backup: the later audit entry
could not be recorded in the active database. Check available disk space. If a
directory-synchronization warning appears, the drive did not confirm that final
filesystem update; keep another backup and test recovery. Do not unplug a drive
while an operation is running. A crash during file writing can leave an incomplete
file, which restore will reject; never discard older backups just because a newer
filename exists.

## Restore or move to another computer

Close other app instances first. Use version 0.6.0 or newer for the recovered
workspace; older applications do not understand the recovery folder layout.

1. Lock the workspace. On the sign-in screen, expand **Restore an encrypted
   backup**. These controls are also available before initial setup and on the
   **Unable to open the workspace** screen if files are missing or damaged.
2. Choose **Choose backup file**, select the `.ebcbackup` file, and enter the
   password used when that backup was created.
3. Choose **Verify backup for recovery**. Verification checks the file's
   authentication, encrypted database integrity, supported schema, relationships,
   and saved financial-plan inputs before showing a review.
4. Review the organization, counselor, client count, and backup creation time.
   This is the entire workspace you will restore. It does not merge with current
   records. Changes made after that backup are absent from the restored view.
5. Type **RESTORE** exactly and choose **Restore verified workspace**. Or choose
   **Cancel recovery** to discard the prepared copy. Reviews expire in 15 minutes.
6. Wait for **Workspace restored. Unlock it using the password from this backup.**
   Unlock with that password and check the clients, notes, and financial plans.
7. Keep the original backup until you have checked the restored data. Create a
   fresh backup after continuing work.

If the password is incorrect, the file is incomplete or altered, the schema is
unsupported, a migration fails, the source changes during review, or activation
fails, the current workspace is not replaced. Try a known-good backup. Do not
remove workspace files to bypass the error. If the app stops during review, the
original workspace remains active. If it stops during the final switch, restart
with the same application version: the active pointer selects a complete verified
copy or the previous workspace, never a mixture of their database and credentials.
Disk/drive failure can still require restoration from the separate backup.

Losing the encryption password prevents restoring that backup. There is no
password reset, administrator bypass, or supported password-change feature in
this stage. Reinstalling cannot decrypt the data. The app has no remote copy.

## Preserved previous files

Restore writes a new private folder and keeps the previous files separately.
This includes damaged files, which can be useful for later recovery. It does not
silently delete newer notes or merge them into older records. Previously retained
copies require their original credentials. Do not manually remove or edit the
active pointer or recovery folders. Returning to an earlier workspace through a
supported retained-copy selector and automatic cleanup are future work; use a
known-good `.ebcbackup` through the verified restore controls. Preserve all files
if you need support recovering changes from the retained copy.

On Windows the root is ordinarily
`%LOCALAPPDATA%\com.eisenbart.counseling\` (see the app identifier in the build
configuration). On Linux it is under the user data directory. Initially the root
contains `workspace.db` and `workspace.json`. After restore,
`workspace-active.json` selects a UUID folder inside `recovery-generations`,
containing its own database and metadata. The older root or generation remains.
The `workspace-operation.lock` file is normal; its operating-system lock is
released when a process ends. Unconfirmed encrypted staging folders left after
an abrupt crash are not automatically selected or deleted in this milestone.

## Technical format and safeguards

SQLCipher encrypts the database snapshot and authenticates encrypted pages.
The workspace key is derived using the existing Argon2id settings (64 MiB,
three iterations, one lane, 32-byte output). A domain-separated HMAC-SHA256 key
is derived from it for authenticating the complete container. No independent
backup password, raw password, or encryption key is persisted.

Container version 1 consists of:

- Eight-byte magic `EBCBACK1`.
- Little-endian 32-bit JSON-header length and 64-bit encrypted-database length.
- Header: format version, workspace KDF salt/version, UTC creation time, opaque
  backup UUID, and source schema version. These non-client metadata fields are
  readable; organization names, counselor names, clients, and records are encrypted.
- SQLCipher database bytes, followed by a 32-byte HMAC-SHA256 tag authenticating
  all preceding bytes, including the lengths and header.

Limits are 4 KiB header, 256 MiB database, and 1,000 clients in this milestone.
The parser bounds allocations and streams encrypted data. HMAC verification uses
the cryptographic library's constant-time verifier. It then checks SQLCipher page
integrity, SQLite integrity and foreign keys, application identity, supported
schema, current client financial inputs, approved imported records, and reciprocal transfer links. Older schemas migrate only in the
private candidate. The original files are not migrated or repaired by restore.

Snapshot creation uses `sqlcipher_export` with an explicit encryption key inside
a database transaction; committed WAL data is included. Copying a live `.db`
file alone could miss that data and is not the backup method. A completed backup
precedes its live `backup_created` audit entry; the snapshot therefore contains
prior audit history, not that backup's later audit event. A `workspace_restored`
event linking the backup UUID is written to the candidate before activation.
Audit tables remain immutable; schema 8 preserves preceding events and adds CSV/review events.

Publication uses exclusive file creation for backups. Restore uses a flushed
candidate database/metadata pair and a single atomically replaced pointer on the
same local filesystem. Unix synchronizes directories; Windows uses
`MoveFileExW` with replacement and write-through flags. A short OS file lock
serializes setup, backup, and activation. A changed pointer or source-file stamp
invalidates a pending review. Existing new-version sessions detect an activated
workspace change and lock before their next operation. Close other instances
before recovery; this is not a multi-user database/merge system.

Passwords, Argon2 working memory, and application-managed key buffers are
zeroized. SQLCipher connections holding keys are closed on cancellation, expiry,
and completion. RustCrypto internal states and operating-system/compiler copies
have not been independently audited. Windows memory locking remains best effort.
No key-erasure guarantee or independent security certification is claimed.

Native folder/file pickers run through Rust; the frontend receives no general
filesystem-plugin, shell, or HTTP permission. Diagnostic messages do not include
passwords, encryption keys, or client financial records. The workflow is offline.

## Verified and remaining work

Automated tests cover all saved modules and immutable audit history; recovery on
another data directory; wrong passwords and tampered/truncated/oversized files;
authenticated damaged databases; missing source files; cancellation and expiry;
activation/audit/migration failures; no-overwrite behavior; competing operations;
changed source data; old-session locking; older schema upgrades; and committed
WAL data. The native smoke test exercises visible backup/review/restore controls,
a full process interruption during review, and recovery from missing metadata.
See [development-status.md](development-status.md) for completed results.

Unrun or future work includes actual removable-drive/power-loss drills, a clean
installed Windows 11 recovery workflow, independent crypto/security review,
scheduled backups, retained-copy selection/pruning, password change/recovery,
and inclusion of future document/report file attachments. Authorized original CSVs are stored inside SQLCipher, so current backups include their BLOBs, provenance, decisions, and immutable history. There are no separate file attachments yet; the format covers the whole existing database.
