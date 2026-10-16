# Open an exported report for Windows printing — 0.40.0

In **Reports → Print & presentation**, select the saved internal or final report,
review its original identity, figures and current warnings, and export either PDF
layout to a new private filename. Exporting never opens a viewer or prints.

After a successful export, **Open saved PDF to print** shows the actual saved
filename. Read the confidentiality notice, select the explicit opening checkbox,
then click the button. Windows is asked to open that exact PDF with its installed
`.pdf` association. Choose **Print** and your printer inside that viewer. For a
client handout, continue to use the separate client action-plan export workflow;
these counselor reports may include private notes and evidence.

The app reports only that Windows accepted the opening request. It cannot confirm
that a viewer displayed the file, a print dialog appeared, a printer exists, a job
was accepted or any pages printed. Print cancellation, missing printers, paper
errors and spooler failures belong to the viewer/Windows printing workflow.
This stage does not submit a print job or implement an in-app printer dialog.

## File lifetime and failure behavior

The file is the explicit export in your chosen folder, outside workspace
encryption. No additional temporary PDF or cleanup schedule is introduced.
The file survives workspace lock, sign-out, restart and backup recovery. A viewer
may keep its own readable copy or recent-file history. Close that viewer and
manage the export yourself when finished; locking this app cannot retract it.

The opening shortcut is limited to the most recent presentation export in the
current unlocked session, for ten minutes. After Windows accepts it, the request
is consumed. Another export attempt revokes the previous request. Page/client,
report/layout/destination changes and preview refresh clear the visible shortcut
and consent. Closing or locking the workspace discards its in-memory request;
backup/recovery never saves or restores it. Open an earlier export manually, or
export a new file for a fresh shortcut.

Rust reauthorizes the original client, reloads the scoped report context and
checks it against the export's receipt. A changed context requires explicit
refresh/re-export. It verifies the actual file against the exported bytes using
SHA-256, a PDF header, a regular-file check and the existing 32 MiB output bound.
Missing, modified, oversized or symbolic-link files fail before dispatch. Nothing
is rewritten, approved, deleted or added to financial/audit history by opening.
On Windows a read handle denies writes/deletion while verification and dispatch
run. The external viewer owns subsequent reads; the app cannot promise file
immutability after dispatch. Lock/restore on this instance cannot interleave
the guarded dispatch because it holds the same managed workspace mutex.

The native adapter calls `ShellExecuteExW` with the fixed `open` verb, one literal
UTF-16 filename and no parameter string. It does not use `cmd`, PowerShell,
`start`, a URL opener or a general renderer shell permission. Drive paths support
spaces, Unicode and literal punctuation. UNC/network, device, URL, alternate data
stream and non-PDF paths are rejected by this opening action; manual exports
retain their existing writer rules. Windows association configuration remains
under the operating system's control.

An OS-reported cancellation is shown as cancellation. Missing associations,
missing file/viewer, denied access and other rejected requests have actionable
failure messages. Cancellation/failure leaves the export unchanged and permits
an explicit retry while the request remains valid. A request is never retried
automatically. Non-Windows builds show manual local-viewer instructions and do
not offer a Windows opening button.

## Verification and remaining Windows acceptance

Automated fixtures cover literal UTF-16 path preparation, rejected path forms,
OS-result mapping, byte/file bounds, Windows write-sharing denial, original scope,
stale receipts, forged/foreign/expired/consumed requests, missing/changed exports,
read-only history, lock/unlock/restart, explicit consent, cancellation/error retry,
client switches, unsupported platforms and late responses. Adapter result tests
use a fake launcher: they do not claim to run an installed PDF viewer or printer.
The complete Linux native workflow still exports and inspects real report PDFs;
it checks the actual path, lifetime notice, platform fallback and stale-path
clearing. Actual executed results appear in [development status](development-status.md).

Still required on a clean Windows 11 installation, with fictional data and the
network disconnected:

1. Install/select a local PDF viewer; export both layouts and verify the exact
   saved PDF opens only after explicit consent, including Unicode/spaces.
2. Test missing/broken associations and denied or changed files. Check failure
   messaging and that no substitute file or unexpected window is opened.
3. Print to a real printer and Microsoft Print to PDF. Cancel in the viewer;
   test missing/offline printers and a rejected/spooler-failed job. The app must
   never claim print success in any of these cases.
4. Lock/close/restart/restore while reviewing exports. Verify session shortcuts
   disappear and that readable files/viewer copies remain as disclosed.

Windows source tests are separate from these installed viewer/printer checks.
No new installer is authorized by this stage. Direct in-app printer selection
and job tracking remain future work if required beyond this local-viewer path.
