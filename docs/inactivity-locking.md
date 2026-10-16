# Configurable inactivity locking — 0.45.0 source

Settings → Security → Inactivity locking accepts 1–60 whole minutes. Automatic
locking cannot be disabled. Existing workspaces and older backups default to
15 minutes when schema 29 adds an empty policy history. The first explicit
change creates revision 1. Choose a timeout and select **Save inactivity timeout**.

Saving counts as activity in the current instance. Other idle instances running
this version apply the latest encrypted policy without resetting their interval;
a shorter setting can lock them immediately. Close older application versions
before upgrading or changing policy: already-running older binaries cannot learn
this feature. Reopening an older application against schema 29 is unsupported.

Rust checks elapsed monotonic time against the latest saved policy before access
to workspace commands and on the existing 15-second background timer. Idle UI
clearing can occur up to 15 seconds after the limit, subject to system scheduling;
an operation at or after the limit is rejected immediately. Policy-read commands
and timer checks do not extend activity. Clicking Reload
is still normal user interaction and can generate the existing throttled activity
heartbeat. Existing app commands and throttled keyboard/pointer-down activity
keep an unexpired session active. Passive viewing is not activity. A changed active generation or unreadable/invalid policy
locks the session instead of falling back to a longer interval.

Locking drops the encrypted connection, pending import previews, restore review
and PDF handoff receipts. The existing lock event clears private clients, drafts,
searches and settings from React. Unlock requires the workspace password. Manual
locking remains available. Submitted or exported readable files outside the app
remain user-managed; locking does not revoke them.

## Saved history, concurrent changes and recovery

Schema 29 adds `workspace_security_history` with authenticated organization and
counselor scope, contiguous immutable revisions, checked minutes and UTC save time.
A save compares the exact revision and commits the new row with
`security_policy_saved` in one transaction. A stale or uncertain save requires
reload; it is never automatically replayed. Reads show the latest ten of at most
1,000 changes; all earlier rows remain encrypted and immutable. The bounded stage
rejects further changes at 1,000 rather than pruning history silently.

Policy reads validate complete retained history without writing or updating
activity. Migration extends the immutable audit vocabulary and preserves client,
source, financial and report values. Backup verification validates policy history
before activation. Backups retain the policy captured at creation: restoring an
older backup can restore an older timeout or the original 15-minute default.
Password changes preserve the policy in their verified encrypted generation.

The configured inactivity limit does not change the fixed 15-minute maximum
lifetimes for pending restore reviews and CSV/PDF inspection previews. These are
separate safety limits. There is no shared multi-user/network database support,
automatic retained-copy cleanup, password reset or new permission-role model.

## Validation boundary

Fixtures cover default/boundaries, no-refresh reads, other-instance shortening,
invalid inputs, stale revisions, immutable history, audit rollback, bounded history
and gaps, fail-closed reads/unlock, authenticated corrupt backup rejection,
schema-28 migration rollback, restart, password rotation and old/new backup policy.
The native harness exercises an actual one-minute timeout without clock overrides,
private UI clearing, restart and encrypted restore. See development status for
which runs completed. Physical power-loss and installed Windows offline checks,
independent security review and commercial acceptance remain separate.
