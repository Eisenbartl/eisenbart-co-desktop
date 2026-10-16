# Workspace password change — development milestone

Settings → Security requires the current password, a different new password
(12–1,024 characters), confirmation, and explicit acknowledgment of older-copy
passwords. Passwords are not trimmed. Submitted fields clear before the command.
A successful change locks the workspace and clears private drafts, import
previews and PDF handoff receipts. Unlock with the new password, check records,
then create and verify a new encrypted backup. Forgotten-password reset and
multi-counselor accounts are not implemented.

## Key and metadata transition

1. Serialize managed application commands with the existing OS operation lock.
   Check the active generation and session after acquiring it. Concurrent
   instances fail closed while the operation runs; their stale sessions lock
   before their next command after activation. Close other instances first.
2. Verify the current password against the active encrypted database and current
   organization/counselor. Preserve the Argon2id settings (64 MiB, three passes,
   one lane, 32-byte key). Generate a fresh random 32-byte application KDF salt.
3. Use SQLCipher export to create an encrypted snapshot under the new key in a
   private UUID generation. The original database/metadata are never rekeyed or
   replaced in place. Committed WAL data is included. No plaintext snapshot or
   plaintext password is written. Application-owned password/key buffers zeroize.
4. Flush new metadata, verify encrypted pages/SQLite integrity/foreign keys and
   identity, append an atomic `workspace_password_changed` audit event, close,
   then reopen using the actual persisted metadata and new password. Existing
   source BLOBs, history, report payloads/digests and prior audit rows stay intact.
   Schema 28 only extends the immutable audit event vocabulary.
5. Reuse recovery's flushed candidate and atomic active-pointer replacement.
   Before replacement, failure leaves the old generation active. After replacement,
   report success and a distinct directory-sync warning if durability could not
   be confirmed. No fallible operation follows the committed switch. The app locks.
   Restart selects a complete old or new pair, never mixed key metadata/pages.

The IPC operation lock covers whole commands, including waiting database writes,
so a cooperating second app cannot pass a generation check then write into the
retired workspace after activation. This does not coordinate unsupported older
application versions, manual SQL tools or direct file modification.

## Recovery and retained-copy passwords

A change does **not** rewrite, revoke or cryptographically erase older backups or
retained workspace generations. Those copies retain their original passwords.
A restored backup also restores its password; it does not adopt the latest active
password. New backups require the active generation's password. Keep all needed
passwords securely, separately from the copies. The application has no bypass or
remote recovery copy. Do not manually edit the pointer or delete retained files.
Use the existing verified whole-workspace restore controls with explicit RESTORE
confirmation. Retained-copy selection/pruning remains future work.

Incomplete unselected generations can remain after a process crash. They are
never selected automatically. Preserve them for diagnosis; no automatic cleanup
or secure-erasure guarantee is introduced. A disk/drive failure can still require
a separate known-good backup. A directory-sync warning requires keeping both
passwords and backups until restart verifies the active generation.

## Validation boundaries

Backend fixtures cover rejected requests, all-table value digests (including
source BLOBs and exact report bytes), append-only audit, stale instances,
new/old unlock, old/new backup recovery, injected snapshot/audit/metadata/activation
failures, and actual subprocess exit before/after activation. These are process
interruption tests, not physical power-loss or removable-drive certification.
Independent security review and clean installed Windows offline recovery remain
required before real client use. See development status for actual executed checks.
