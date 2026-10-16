# Stage requirements: local workspace password change and recovery safeguards

Implement the planned **Settings → Security** password-change workflow using
the existing authentication, SQLCipher and generation-based recovery architecture.
Require the current password and a reviewed new password/confirmation; never log,
return or persist plaintext passwords. Keep essential operation offline.

Design and review the exact key/salt/metadata transition before writing storage.
Rekey must preserve all client/source/history/report bytes and fail atomically:
interrupted writes, audit failure, wrong password, full disk and restart must
leave a usable, explicit old or new generation. Reuse durable staged activation
instead of destructive in-place replacement without recoverability. Lock stale
instances and clear private previews/receipts/drafts. Do not weaken key derivation,
TLS, backup authentication or session scope to make a test pass.

Existing backups retain their original password; disclose that a password change
does not rewrite or revoke older encrypted copies. Test old/new-password unlock,
rollback, restart and recovery with old and new backups. Forgotten-password reset
without an authorized recovery mechanism remains unavailable. This is password
change, not a new roles or multi-counselor architecture.

Use fictional records and independently verify unchanged source/report digests.
Run full UI/backend/native and Windows source checks, document remaining clean
Windows/security acceptance, and commit one bounded stage. No installer unless
the user explicitly requests one.

Implementation is now in the 0.44.0 source. See [design, recovery policy and limits](workspace-password-change.md) and [executed validation](development-status.md). The criteria above remain acceptance requirements; installed Windows and independent security results must be recorded separately.
