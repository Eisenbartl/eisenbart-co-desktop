# Counselor report presentation — 0.39.0

Open **Reports → Print & presentation**, select a client and a saved assessment,
then select its internal assessment or an existing final counselor approval.
Choose **Summary with evidence appendix** or **Original detailed layout**. Enter
a new private PDF filename or use the native destination dialog to choose and
export. Cancelling the dialog creates no file. Existing files are never replaced.

The first summary page contains the original firm identity/logo, full client
name, assessment period, original report date, approval/internal/historical status
and exact saved financial figures. Long identities use compact typography rather
than truncation. The full report title, contact details, captured firm disclosure,
counselor notes, source evidence, scope decisions and approval integrity details
remain in the clearly marked evidence appendix. The summary is bounded to one
page; unsupported typography or output bounds fail without a partial PDF. The
original detailed layout remains available, including through the older export
controls. Already exported files are retained unchanged.

Both formats are counselor reports and can include private internal context.
Use the separate client action-plan workflow for a client handout. Neither the
new page nor its summaries copy internal evidence into client plans.

## Financial and historical interpretation

The presentation version is `counselor-summary-v1`. It is separate from assessment
capture formats, approval policies and financial calculators. No schema migration,
new financial calculations, approval mutation or dependency is introduced.

- Internal assessments remain explicitly incomplete and show their original
  bank-only approved activity. Captured card supplements remain in the appendix;
  an internal format-9 report does not inherit a separate final combined summary.
- Existing v1 final approvals retain their bank-only basis and original figures.
- V2 final approvals display the original approved combined necessary,
  discretionary and total spending, income less combined spending, bank cash,
  matched repayments excluded from spending, and existing debt counted once.
- Unknown balance facts remain Unknown. Balance-date facts are labeled separately
  from period activity; recorded balances do not imply a complete inventory.
- Later client, firm, source, planning or review changes retain original identities
  and figures, with current-review flags visible on the first page and reasons in
  the preview/appendix. The export time is the context-check time, not a new report
  date or financial approval.

## Scope, export and printing

Rust reauthorizes the selected client and reads validated assessment/final
records inside one transaction. The associated final approval is looked up within
the authenticated organization/counselor/client. A preview receipt binds the
selected capture and displayed current-review context; export reloads and checks
it before rendering. A changed receipt requires explicit refresh/export, with no
file written. All paths use the existing bounded, atomic, no-overwrite PDF writer,
including its workspace-path protections. Preview and export add no financial or
audit rows. Client changes/lock clear private page data and ignore late replies;
navigation is held while a destination/export operation is running.

Exported PDFs contain readable confidential information outside workspace
encryption. Print an exported PDF with your Windows PDF viewer's **Print** command.
Direct printing from inside this app is still planned and is not claimed as
verified by Linux native smoke or Windows source tests. No installer was requested.

## Verification

Focused backend fixtures check first-page content/order, full appendix, bank-only
v1 and combined v2 independent cents, debt shown once, original payloads/digests,
reversal/history flags, unknown balances, legacy format 1, long identities/logo
and firm disclosure, stale previews, absent/foreign/locked reports, read-only
change counts, protected paths, no-overwrite, restart and encrypted recovery.
UI tests cover both layouts, exact receipt submission, native cancellation,
explicit stale retry, client switches, busy navigation, lock, empty/unapproved
states and literal escaped text. Native tests export real PDFs through visible
controls and inspect page-one text, original detailed layout, existing-file
protection, identity, restart and recovery. Actual results are recorded in
[development status](development-status.md).
