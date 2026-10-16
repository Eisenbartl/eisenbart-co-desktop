# Security and local data

## Development-use boundary

Use fictional counselor and client records in this milestone. Encryption and local authentication are foundation safeguards, but they do not establish a completed security audit, regulatory compliance, or readiness to hold real financial records. Encrypted backup and verified recovery are implemented and tested in source. Clean Windows recovery, stronger lifecycle safeguards, and independent review remain release blockers.

## Database and password

The application stores its database locally using SQLCipher. A counselor password derives a 32-byte database key with Argon2id using a unique salt, 64 MiB of memory, three iterations, and one lane. The password is not stored as plaintext and there is no hard-coded administrator password or database key. Successful sign-in requires opening and validating the encrypted database.

The first setup creates one organization and counselor on the installation. Choose a long, unique password and retain it securely outside the application. No developer service can unlock the database for you.

**There is no password reset or recovery mechanism in this milestone. Losing the password can permanently prevent access to every record in the database.** Settings → Security now supports a reviewed current-password change using a verified new encrypted generation; see [password changes and retained copies](workspace-password-change.md). Encrypted backup and verified whole-workspace restore are available; they require the password used when the backup was created. Do not delete local files or reinstall in an attempt to recover a forgotten password.

The local data directory contains `workspace.db` (the encrypted database) and `workspace.json` (non-secret version and Argon2 salt metadata needed for unlocking it). Preserve both files if investigating a failure. Check the Windows guide for the expected directory before making any manual copy. A manual copy of a live database can miss journal/WAL data; use the [backup and recovery controls](backup-recovery.md). After restore or password change, `workspace-active.json` selects a complete database/metadata pair inside `recovery-generations`; prior files are retained. Preserve the whole data folder when investigating a failure.

The initial password must have at least 12 characters and at most 1,024 characters. This minimum does not guarantee that a password is strong; a long, unique passphrase is preferable.

## Sessions and access

The interface provides a manual lock and locks after 15 minutes of inactivity. Locking removes the active database session from application state and requires the password again. The timeout is fixed in this milestone; user-configurable policies come later. Client-data commands and backup creation reject operations without an authenticated session and enforce organization ownership independently of the selected interface record. Restore verification is deliberately available while locked and authenticates the selected backup using its password; activation requires a verified, current review and explicit confirmation.

Initial accounts do not provide separate administrator, counselor, and read-only reviewer roles. Future multi-user work must add and test explicit permissions; organization identifiers alone are not a completed permissions product.

## Privacy and limitations

The essential workflow is local. No cloud account, Google Sheets connection, telemetry service, or transmission of client data is required by this foundation. The interface and application assets are bundled locally. This is an implementation goal supported by inspection and tests listed in [development-status.md](development-status.md); installed Windows behavior while disconnected remains a separate acceptance check.

SQLCipher protects the database at rest. It does not protect against malware, a compromised operating system, screenshots, or someone using an already unlocked session. Full-disk encryption, a protected Windows account, device updates, and physical access controls remain useful safeguards. In-memory values and operating-system crash or paging behavior need further security review; do not describe locking as guaranteed erasure of every prior memory copy.

Audit records live inside the same encrypted database. The application appends operational history, but this does not make the history tamper-proof against someone with the key and direct database access. Avoid sensitive values in logs and audit descriptions.

Bounded synthetic UTF-8 CSV imports now use explicit mapping, no active-content execution, immutable provenance/history, client-scoped encrypted storage, authorized optional original-file BLOBs, atomic decisions, duplicate warnings, and approved-only exact totals. Lock clears transient previews. Existing encrypted backups include imported sources/history; restore checks approved records and reciprocal transfer links. See [csv-import-review.md](csv-import-review.md) for resource limits and unimplemented parser/review features. This does not establish installed Windows or independent security acceptance. OCR remains unselected; PDF parser dependencies are inventoried separately.

## Windows memory-lock diagnostics

SQLCipher's memory security and memory wiping remain enabled. Physical memory
locking is best effort and can fail under operating-system quotas. The locked
SQLCipher version has a Windows logging recursion bug: formatting a native
memory-lock warning can trigger another failed lock and repeat until stack
overflow. The application excludes native MEMORY diagnostics on Windows while
preserving CORE, PROVIDER, and MUTEX diagnostics and normal application errors.
This does not turn off lock attempts or integrity verification. The Windows
regression exercises encryption and password rejection under a deliberately
constrained quota in a separate process. Revisit the diagnostic filter when an
upstream fix becomes available and retain paging/key-lifecycle review as a
production requirement.

## Release-blocking security work

- Complete clean Windows offline recovery and removable-drive/power-loss drills for the implemented authenticated backups and staged restore.
- Independently review the implemented generation-based password change and retained-copy/original-backup password policy; no reset bypass or master key exists.
- Extend validated migration failure handling, corruption detection, and interrupted restore tests to installed Windows and future attachments.
- Test client and organization isolation throughout future imports, calculations, exports, and reports.
- Add role enforcement, configurable inactivity settings, safe archival/deletion, and data export.
- Review key lifecycle, memory handling, dependency vulnerabilities and licenses, installer signing, and local file permissions.
- Perform independent security and privacy review before using or selling a production release.

Never place real passwords, signing keys, tokens, or client records in Git, test fixtures, documentation, or issue reports.

Selectable-text PDFs now run in a supervised child process before workspace initialization, with sanitized environment and bounded stdin/stdout, text/page/line, memory and time limits. No PDF renderer, JavaScript evaluator, attachment extraction, OCR, or network service is invoked. This is a process/resource boundary, not a complete OS sandbox or completed independent security review. PDF source locators and authorized original BLOBs use SQLCipher and authenticated backup; every PDF record requires individual approval with source acknowledgment and a reason. See [pdf-import-review.md](pdf-import-review.md).
