# Captured planning scenarios — 0.15.0

An internal counselor assessment can now preserve a named scenario containing the
client's **saved** budget, debt, savings and dated cash-flow plans. This captures
entered assumptions and forecasts separately from approved historical activity.
It does not create a final agreed client action plan or change any financial plan,
source decision, source revision or imported amount.

## Counselor workflow

Save your budget and other planning edits first. In **Saved counselor assessments**,
enter the assessment period/title, enable **Include a saved planning scenario**,
and select **Review current saved plans**. Review the saved amounts, terms,
missing/estimated values, allocations, forecasts and warnings. Enter a scenario
label and assumptions/uncertainties. Explicitly select Snowball, Avalanche or
**No repayment projection selected**. Confirm the assessment acknowledgment and
save the record.

A saved nonempty budget is required. Debt, savings and dated cash-flow plans can
be missing; missing plans remain labeled missing and never supply invented inputs
or milestones. An unsupported/incomplete debt model cannot be selected for a
repayment projection. An available projection can still have shortfall, growth or
horizon warnings; those warnings and outcome status remain captured. This is an
internal incomplete assessment, not a guarantee of payoff or readiness to commit.

Savings uses the primary saved emergency contribution. Alternative contribution
forecasts remain comparison references. Cash-flow dates, accounts, transfers and
schedules are independently entered; the application does not automatically copy
budget/debt/savings amounts into them or assert that they reconcile.

A scenario holds all four saved plan versions and dependent contexts in one
consistent database read. If any version changes before save, the save fails and
preserves the draft. Select **Review current saved plans** again, review the new
baseline, then renew acknowledgment. No write is automatically retried. New
planning selections and reloading saved plans clear the acknowledgment.

## What the record preserves

The scenario stores its label, counselor-authored assumptions, explicit debt
strategy, calculation version, per-plan revisions and saved timestamps, canonical
input data and exact calculated outputs. The surrounding immutable assessment
provides creation time, authenticated client/firm/counselor and approved source
baseline. It includes:

- Budget lines/frequencies, monthly normalization, allocations, missing/estimated
  counts and warnings.
- Debt balances, APR, minimums, fees, entered model/verification/terms notes,
  payment changes and extras, both comparison projections and their schedules;
  the selected strategy determines the displayed repayment outcome/milestones.
- Savings pools/reserves, target months, primary and alternative contributions,
  goal amounts/deadlines, forecasts, allocation schedules and warnings.
- Dated account openings, entered schedules/recurrence/certainty/transfer endpoints,
  independent projected ledger/day balances, closing balances and warnings.

The viewer labels hypothetical values and has separate approved-data totals.
Captured schedules are read-only and paginated in groups of 50. Later plan changes
mark the older record **Needs review** without modifying its payload or checksum.
Returning active inputs to prior values does not restore the captured versions.
A new record is needed for current assumptions/projections.

The directory refreshes after a locally conflicting operation finishes. Use
**Refresh saved assessments** after changes elsewhere or in another window.
Displayed matches refer to the last checked context. Opening/exporting a record
rechecks current scoped versions; PDF review flags use the current check.

## Offline PDF and compatibility

Saved PDFs include the named hypothetical scenario, budget assumptions/allocations,
entered repayment terms and chosen outcomes/account payoff milestones, savings
inputs/goals/primary milestones, and cash-flow account/schedule assumptions,
closing forecasts and dated concerns. They clearly separate those values from the
historical approved assessment. PDFs print scenario assumptions and summary
milestones; full model schedules remain in the encrypted record and viewer.
Export is still an internal/incomplete report with the existing font, file-size,
page-count, privacy and no-overwrite rules. See [assessment-pdf-export.md](assessment-pdf-export.md).

Database schema remains 14; no migration or rewrite of existing records is needed.
New assessments use payload format 2 and optional `planning`. Format 1 records
remain readable/exportable without planning. A format 1 record cannot acquire a
planning attachment. The approved assessment calculator stays
`approved-assessment-v1`; the optional planning calculator is `entered-plans-v1`.
Future calculation changes must retain compatible v1 evaluation/readers rather
than silently reinterpreting old snapshots.

Rust checks authenticated ownership, acknowledgment and expected source/profile
and all four plan revisions inside the save transaction. IPC accepts selection,
notes and expected versions, never frontend financial outputs. Captured reads,
PDF export and backup restore validate hash/headers, client IDs, revisions/dates,
canonical inputs, derived contexts and exact outputs from the captured inputs.
Shared pure context rules prevent differences between live plan calculations and
snapshot validation. Unsupported versions, extra fields, malformed identities,
forged outcomes or inconsistent contexts fail. There is a 4 MiB scenario limit
within the existing 8 MiB record / 32 MiB client limits.

## Validation and remaining scope

New backend tests independently check entered income 3,250.00, unallocated 2,150.00,
Snowball interest 43.10 versus Avalanche 35.35, emergency target 1,500.00 and
completion 2027-01-31, alongside historical cash flow 799.99. Later source decisions
produce a new 2,000.01 record while preserving the original. Tests also cover stale
versions, missing budget, unsupported projection, scope/lock, forged outcomes and
contexts, extra fields, rejected corrupted recovery, format 1 compatibility,
restart and recovery of a legitimate historical scenario after plan edits.

Three interface tests cover explicit saved-plan review, selection-only IPC,
separate historical/planning figures, escaped assumptions, failed draft retention,
renewed acknowledgment, refresh/review flags and late/mismatched client replies.
Native synthetic tests capture an Avalanche scenario, change income, restore prior
inputs without reactivating the record, preserve 716.67 original unallocated/
35.35 interest/2028-06-30 savings milestone/3,300.00 independent cash-flow closing,
and export/recover the scenario after restart and both recovery paths.
Actual executed results are in [development-status.md](development-status.md).

Next: client action plans with three to five prioritized actions, agreed allocations,
selected milestones, follow-up and appropriate final review/disclosures. General
non-writing scenario variations and side-by-side combined comparisons, remaining
advanced models, saved session/recurring-review evidence, organization logo/contact
branding, direct printing and the full original release/commercial scope remain
open. See [remaining-requirements.md](remaining-requirements.md).
