# Counselor report evidence — 0.17.0

Open a client and **Saved counselor assessments**. Complete the assessment period
and findings, then enable **Include saved session and recurring-payment review
evidence**. Choose **Review report evidence**, select up to ten saved session notes
from the latest 100, review the payment group/decision counts, and acknowledge the
correct client, approved decisions, estimates and missing information before saving.
A captured planning scenario remains optional and independent. Export the saved
record as an internal PDF to a private location outside workspace encryption.
Existing files are never overwritten; print the PDF in a local viewer.

The viewer and internal PDF preserve selected notes with their original author,
date and note IDs. Note dates may fall outside the financial assessment period.
Unselected and later notes are explicitly excluded. Narrative amounts do not enter
approved financial totals. Evidence times are displayed in America/Chicago, with
original UTC timestamps retained in the immutable payload.

The report captures approved merchant/payment groups across **all imported dates**,
separately from the assessment period. It shows recorded account, source dates,
exact historical total/range, advisory cadence, captured applicability, warnings
and immutable approval/source/page/line bindings. The latest counselor decision
for each reviewed group retains label, frequency/subscription, reason, author,
time, decision ID, original evidence digest and original reviewed amounts/bindings.
Older superseded decisions stay in the live encrypted review history. Latest
historical decisions without a current approved group remain disclosed.

A **Needs review** group is not a current confirmation. Identical source values
returning after reversal never silently revive invalidated decisions. Reviewed
frequency/subscription is historical counselor context, not verification of future
obligations, and never creates automatic budget/debt/cash-flow entries. Client PDFs
continue to exclude these internal session/payment evidence sections.

## Persistence and validation

Assessment payload format 3 adds optional `supporting` with calculator
`counselor-evidence-v1`, the saved note count and selected immutable notes. Formats
1 and 2 remain readable/exportable and their bytes are not rewritten; assessments
without this option continue to save format 2. Schema remains 15. The report reuses
append-only assessment storage and its atomic `assessment_recorded` audit event.
No dependency or installer is added. Older app versions reject the new report
format; use 0.17.0 or later to open/restore it.

IPC supplies selection only: expected note count and note IDs. Rust reads the
scoped original notes and current source inside the assessment save transaction,
checks source/profile/plan versions, rejects changed note counts, duplicates,
foreign notes and more than ten selections, and preserves failed drafts. The
preview contains at most 100 notes; supporting reports allow up to 10,000 available
notes. Existing assessment limits remain 50 records, 8 MiB/record, 32 MiB/client.

Capture retains the latest decision per recurring group and every older approval
binding required by that decision, in addition to current approvals and financial
classifications. Captured groups and their exact totals/states are recomputed from
captured inputs during validation. Reads, export and recovery check note
scope/canonical text/time, original note equality against append-only storage,
source/decision identity and financial arithmetic. Altered notes or rehashed
forged group outputs fail before export/recovery activation.

New session notes flag supporting reports for review without rewriting selected
original context. Source/profile/firm/plan changes retain existing review behavior.
Legacy reports do not claim to include notes and are not newly flagged just because
a note is appended. Action plans referencing a supporting assessment inherit its
review state. Reload evidence after adding notes and renew acknowledgment; selected
IDs are retained, and unavailable selections block saving rather than silently
changing the chosen evidence.

## Remaining scope

This expands internal counselor reports; complete financial coverage is still
unverified. It adds no electronic signatures, automatic reminder/action progress,
logo/contact branding, direct application printing, new advanced financial model
or commercial release acceptance. Complete-basis final validation, remaining
models/scenarios and release/security work remain in
[remaining-requirements.md](remaining-requirements.md).
