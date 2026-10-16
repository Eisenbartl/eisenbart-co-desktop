# Development status

## Current stage — local category suggestions (0.41.0, schema 25 unchanged)

Transactions → Classification assistance lists missing-category bank expenses
and fees in stable 25-record pages, with literal search and exact current approval
evidence. Two to five matching approvals on distinct dates must agree; conflicts,
possible duplicates, unknown source facts and larger groups stay unresolved.
Same-account/type merchant matching reuses the existing case/space normalization.
No income/transfer/refund/debt/card role is guessed.

Using a suggestion fills only an unsaved category field in the existing review
form, preserving date, amount, type and unapproved status. Explicit save retains
all original audit, history, warning/approval and revision checks. Scoped read-only
queries/preparation use source fingerprints; no new persistence layer, schema,
financial calculator, dependency or automatic approval is added. See
[usage and limits](classification-assistance.md).

Validation in progress on 2026-10-16 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing environment configuration reused |
| Production frontend / UI | Passed build and 231 tests |
| Focused backend | Passed complete classification fixture, including read-only scope, atomic decisions and recovery |
| Complete Linux backend | Passed 224 library + 2 PDF-worker tests |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build / complete native workflow | Build starting; full smoke pending |
| Windows source validation | Starting with feature push; installer steps remain disabled |
| Installer | Not created, per user instruction |

Next priority: **reusable local category-rule CRUD**, with explicit scope,
version/history, conflict handling and archived/reactivated rules. Rules will
continue to fill review drafts without automatically approving transactions.
See [next-stage notes](classification-rules-next-stage.md). Larger evidence groups,
other bank/card roles, OCR/adapters, advanced models, roles/security/commercial
work and clean installed Windows offline acceptance remain open requirements.

## Windows saved-PDF print workflow (0.40.0, schema 25 unchanged)

Reports → Print & presentation now offers **Open saved PDF to print** after an
explicit export, with a separate confidentiality acknowledgement. The Windows
adapter requests the installed `.pdf` viewer using one literal Unicode filename
and a fixed open verb, without a command interpreter or generic shell permission.
The counselor chooses Print in that viewer. Windows acceptance of opening is
never presented as successful printing or even confirmed viewer display.

The Rust session retains one ten-minute request bound to the client, source
context, actual saved filename and exact exported bytes. Dispatch reauthorizes
scope/context, rejects changed/missing/oversized/link files and holds the workspace
mutex through verification/dispatch. Windows denies file writes/deletion during
that interval. Lock/restart/restore discard the request; no readable temporary
copy or financial/database write is introduced. The explicit export and external
viewer copies remain readable after lock, as disclosed. Other platforms show
manual local-viewer instructions. See [usage, failures and acceptance](windows-pdf-print-workflow.md).

Validation completed on 2026-10-15 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / UI suite | Passed corrected-source production build and 223 tests |
| Focused handoff fixture | Passed final scope/receipt/file/expiry/lock/restart/external-restore fixture |
| Full Linux backend | Passed 223 library + 2 PDF-worker tests; final expanded handoff fixture also passed separately |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed corrected-source debug build, no bundles |
| Complete native workflow | Passed, exit 0 and final completion marker: all earlier financial/source/recovery checks plus saved-PDF path, lifetime notice, truthful non-Windows fallback and clearing on layout/client changes |
| Native visual inspection | Passed actual saved-PDF workflow screen; Windows consent/printer UI still needs installed acceptance |
| Windows source suite | Passed: 223 UI + 224 library + 2 PDF-worker tests on corrected source; five installer/checksum/upload steps skipped |
| Installed Windows viewer/printer acceptance | Not run in this Linux cloud environment |
| Installer | Not created, per user instruction |

Review found the new Windows consent checkbox inheriting the text-input width.
Scoped checkbox sizing and wrapping for long saved paths correct that layout.
The first native/Windows checks were deliberately stopped/superseded; incomplete
runs are not counted as passes. Corrected-source checks above completed successfully.

Feature commit: `289b4363939b941e4ae308792962dc4f7d0f0ba4`.
Corrected application source: `add67a48ed898741ce0377fdc1ad6fc5d3470068`.
[Windows run 38844031485](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38844031485)
passed on that exact source. Its library tests completed in 842.72 seconds. Raw
metadata/logs were checked for the exact SHA, actual counts and five skipped
installer steps. Source tests use a fake launcher for dispatch outcomes; they do
not prove an installed viewer or printer worked.

Native artifacts: `/workspace/shared/desktop-smoke-printing`. Logs and verified
Windows evidence use `/workspace/shared/printing-`; the corrected native run has
both exit `0` and the final complete-workflow marker. The interrupted first native
run is retained under `before-layout` and explicitly marked incomplete. No Cargo
checks ran after the standalone build or during the corrected native workflow.

Clean source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-windows-pdf-print-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-windows-pdf-print-source.bundle`; the
checkpoint helper checks every tracked file against final clean HEAD and remote
main. Only this final validation record follows the application source checked
on Windows.

No schema migration or new package was added. Existing `windows-sys` enables the
Shell/COM/Registry/window API features required by the narrow native adapter.
Direct in-app printer selection, job submission and spooler tracking are not
implemented. The last downloadable installer remains 0.23.0.

Next implementation priority: **bounded local transaction classification
assistance**, beginning with reviewable bank-transaction suggestions and explicit
counselor application using existing immutable review decisions. Do not duplicate
merchant grouping, recurring review, source correction or financial calculators.
See [next-stage notes](classification-assistance-next-stage.md). Installed Windows
viewer/printer and broader offline/commercial acceptance remain open release
checks, independently of this source stage.

## Counselor report presentation (0.39.0, schema 25 unchanged)

Reports → Print & presentation offers a summary-first PDF with original firm,
client, period, review status and exact figures, followed by a full evidence
appendix. Original detailed exports remain available. Internal bank-only, v1
bank-only final and v2 combined final bases remain distinct. Scoped read-only
preview receipts, stale-export checks and the existing no-overwrite writer protect
the workflow. See [usage and limits](report-presentation.md).

Validation on 2026-10-15 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / complete UI suite | Passed build and 218 tests |
| Complete Linux backend | Passed 219 library + 2 PDF-worker tests on corrected source |
| Focused PDF checks | Passed four tests, including first-page bounds and explicit logo/glyph clearance |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles on corrected source |
| Complete native workflow | Passed, exit 0: all earlier workflows plus internal/v1/v2 first-page figures, full appendix, original detailed option, no-overwrite, identity, lock/client isolation, restart and encrypted recovery |
| Visual inspection | Passed corrected internal/v1/v2 summary pages, appendix and report preview |
| Windows source validation | Passed: 218 UI + 220 library + 2 PDF-worker tests; all five installer/checksum/upload steps skipped |
| Installer | Not created, per user instruction |

Feature commit: `2a4fb123576c531a17eec973175cbbae3f9cf56c`.
Logo-spacing correction: `caf1db529b33c8edb5751b76ac05278dcb1cda18`.
Application source is unchanged by subsequent UI-fixture commits. Final Windows
source validation passed for `70721230a6fb6eddca64f529c9768725f20507a7` in
[run 38840593917](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38840593917).
Its Rust library suite completed in 742.53 seconds. Raw result metadata/logs were
checked for that exact source, actual test counts and five skipped installer steps.

The long-identity fixture initially exposed a first-page overflow. Compact full
identities, a compact logo, fixed summary rows and moving the full title and firm
disclosure to the appendix now preserve page-one financial figures even with
maximal names, a logo and a current-review warning. The focused rerun passed.

The first complete native workflow passed. Subsequent image inspection exposed
logo/firm-name overlap, which text extraction cannot detect. The new presentation
now reserves explicit logo clearance on summary/appendix pages; original detailed
layout geometry is unchanged. A geometry regression test checks glyph bounds and
the top margin. Corrected source passed the complete backend and native workflow;
rendered summary and appendix images were visually checked for clear spacing.

Windows initially passed the first feature source. The corrected-source rerun
exposed an older branding fixture observing the dirty callback before its effect
completed; test-only commit `4383666` waits for that callback. A subsequent run
exposed the older v1 approval fixture clicking readiness before the selected
report's child initialization completed; test-only commit `7072123` flushes that
saved-report initialization before clicking. Every original assertion remains.
The complete 218-test UI suite and all Windows backend tests now pass on the
latest run. The two failed-attempt logs are retained; no assertion was weakened
and no financial, authentication or persistence behavior changed for those fixes.


Native artifacts: `/workspace/shared/desktop-smoke-presentation`; validation logs
use `/workspace/shared/presentation-`. The corrected native exit file records `0`.
First-pass PDFs/screens/logs are preserved separately under `before-spacing`.
No dependency, schema migration or installer was added. Direct Windows printing,
installed Windows acceptance and commercial readiness remain open; the last
installer preview is still 0.23.0.

Clean source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-report-presentation-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-report-presentation-source.bundle`, verified
against the final clean commit and remote main. Only validation/development notes
follow the exact source tested on Windows.

Next priority: bounded native Windows printing, with explicit file lifecycle,
user action, cancellation/failure handling and Windows acceptance. Direct in-app
printing remains planned; exported PDFs can be printed in a Windows PDF viewer.
See [next-stage notes](native-printing-next-stage.md).


## Searchable transaction activity (0.38.0, schema 25 unchanged)

Transactions → Activity history adds a bounded read-only bank/card import and
decision timeline, account/source/date/status and literal text filters, stable
25-event pages, original/corrected evidence, exact counterpart decision locators
and explicit current/obsolete relationship labels. Existing source review and
financial calculations remain unchanged. See [usage and limits](transaction-activity-history.md).

Validation on 2026-10-15 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / complete UI suite | Passed build and 211 tests |
| Complete Linux backend | Passed 215 library + 2 PDF-worker tests |
| Focused history fixtures | Passed all three mixed-history, scope/recovery and relationship/error fixtures |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles |
| Complete native workflow | Passed, exit 0: all earlier workflows plus stable forward/backward pages, unchanged review counts, original/corrected card evidence, source/date/status filters, reciprocal transfer history, exact repayment decisions, archived account identity, empty results, client isolation, lock and full process restart over recovered records |
| Windows source validation | Passed: 211 UI + 216 library + 2 PDF-worker tests; all five installer/checksum/upload steps skipped |
| Installer | Not created, per user instruction |

Feature commit: `ec562c7f4b7e4d937996557776a964a1d80e15a4`.
Windows source validation passed for that exact application source in
[run 38828170921](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38828170921);
its Rust library tests completed in 746.47 seconds.

The first native run passed every earlier workflow, then its new pagination
fixture assumed the selected synthetic client had more than 25 timeline events.
The disabled Next button correctly prevented paging. Test-only commit `0cb9651`
imports 31 pending fictional rows through the existing Documents UI, verifies
page navigation does not change review counts, and uses the original fixture's
actual incoming-transfer description. All earlier assertions and application
source remain unchanged. The complete rerun passed with native exit 0.
First-attempt diagnostics are retained separately. An existing approval UI test
also now waits for the loaded report selector and flushes readiness before
editing, retaining its original privacy/navigation assertions.

Native artifacts: `/workspace/shared/desktop-smoke-activity`; validation logs use
`/workspace/shared/activity-`. The history filters and expanded one-cent-correction
screens were visually inspected. Windows result metadata and raw logs were
checked for the exact feature SHA, actual test counts and five skipped installer
steps. No dependency, schema migration or installer was added.

Clean source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-transaction-activity-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-transaction-activity-source.bundle`, verified
against the final clean commit and remote main. Follow-up commits contain only
native fixtures and development/validation notes; application source remains the
exact feature commit validated on Windows.

Installed Windows/offline printing and full commercial acceptance remain open.
The last downloadable preview remains 0.23.0. This stage completes transaction
history within the documented bounds, not the whole product specification.

Next priority: Reports → Print & presentation, placing client/period and financial
summary first, with approval audit detail in an appendix and separate Windows
printing acceptance. Unsupported loan/card-policy cases remain blocked.


## Card-inclusive final approval (0.37.0, schema 25 unchanged)

Reports → Final report checklist now selects policy v2 for matching format-9
assessments. Current bank/card completeness, exact approval-bound roles, repayment
counterparts, known required payments and original existing-debt mappings gate
explicit counselor approval. The separate immutable combined summary/PDF keeps
cash movement, spending and debt distinct. Old v1 approvals and original internal
report/calculator bytes retain their interpretation. See
[card-inclusive-final-approval.md](card-inclusive-final-approval.md).

Validation on 2026-10-15 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / complete UI suite | Passed build and 205 tests |
| Complete Linux backend | Passed 212 library + 2 PDF-worker tests |
| Expanded policy/PDF checks | Passed all three focused tests, including ten blocked-evidence fixture cases and final bank-only basis labels |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles |
| Complete native workflow | Passed, exit 0: all earlier workflows plus v2 approval, independent combined figures, explicit repayment renewal/card confirmation, immutable PDFs, later-review flags, restart, v1/v2 encrypted recovery and client isolation |
| Windows source validation | Passed: 205 UI + 213 library + 2 PDF-worker tests; all five installer/checksum/upload steps skipped |
| Installer | Not created, per user instruction |

Feature commit: `34331f501449dd5202122956864058742f04d311`.
Windows source validation passed for that exact source in
[run 38804540401](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38804540401).
Its Rust library tests completed successfully in 762.03 seconds.

The first full native run passed all earlier workflows, then exposed the new
fixture clicking a section while its selected client's existing section was
still loading. Test-only commit `a56951b` waits for real enabled section controls
and loaded report selectors before navigating. A later run reached the updated card evidence but exposed two new fixture
selectors assuming a success-message CSS class that the older completeness
component does not use. Test-only commit `a559051` checks the actual notice
container with the same required text. No assertion or application behavior
changed. An older intake fixture also needed to wait for the report's
post-save directory refresh before editing an adjacent field; test-only commit
`88c42a3` waits for its enabled selector and real native field focus. The final
complete native rerun passed with all original assertions retained.

Native artifacts: `/workspace/shared/desktop-smoke-card-final`; validation logs
use `/workspace/shared/card-final-`. Failed-attempt diagnostics are retained
separately. The native exit file records `0`. Current and restored historical
approval screens were visually inspected; actual PDF text extraction verified
independently known figures and the explicit original-bank-only basis labels.
No dependency, schema migration or installer was added.

Clean source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-card-final-approval-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-card-final-approval-source.bundle`, verified
against the final clean commit and remote main. Follow-up commits change only
native test fixtures and validation/development notes; application source remains
the exact feature commit validated on Windows.

The supported scope remains bounded: other loan/debt obligations, unsupported
bank-refund roles, card credits, early opening/closure and split/cross-period
repayments stay blocked. Installed Windows and commercial acceptance remain open.
The last downloadable preview remains 0.23.0.

Next priority: searchable client-wide bank/card transaction history, using the
existing retained sources and immutable decisions with bounded filters/pagination,
provenance and client isolation. Unsupported loan obligations and other card-policy
cases remain blocked pending their own explicitly modeled evidence workflows.


## Reviewed card completeness evidence (0.36.0, schema 25)

Financial Review → Card completeness records known/unprovided cards, dated
applicability, selected source coverage, spending roles bound to exact approvals
and comparisons to distinct existing verified debts. Immutable format-9 internal
report/PDF captures preserve those choices and findings. Old formats 1–8 retain
their original bytes. Mappings never add debt or change planning/bank figures.
See [card-completeness-evidence.md](card-completeness-evidence.md).

Validation on 2026-10-15 (UTC):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / complete UI suite | Passed build and 201 tests |
| Complete Linux backend | Passed 209 library + 2 PDF-worker tests on final feature source |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles |
| Complete native workflow | Passed, exit 0: all earlier workflows plus card inventory/applicability, approval-bound roles, no duplicate debt, one-cent discrepancy, read-only history, format-9 report/PDF, blocked final approval, restart, encrypted recovery and isolation |
| Windows source validation | Passed: 201 UI + 210 library + 2 PDF-worker tests; all five installer/checksum/upload steps skipped |
| Installer | Not created, per user instruction |

Feature commit: `a425c9e92f722c7b7954a7d34f72a9de4e8ad473`.
Windows source validation passed for that exact source in
[run 38797595959](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38797595959).
The complete backend suite was repeated after the final comparison display and
distinct-debt mapping checks. The first native run passed the earlier workflows
and saved an incomplete card review, then exposed WebKit rejecting the smoke
driver's empty send-keys parameter. Test-only commit `3b34efe` uses a normal
select-all/delete keyboard gesture and verifies the empty value. No assertion or
application behavior was changed. A second run exposed an older final-approval
fixture reading the report selector before its refreshed directory contained the
saved record. Test-only commit `a9d3e56` waits for the real selected title/ID,
preventing the empty placeholder from being treated as the report. The complete
native rerun passed with both fixes and all earlier assertions retained.
No dependency was added. Native artifacts are under
`/workspace/shared/desktop-smoke-card-completeness`; validation logs use the
`/workspace/shared/card-completeness-` prefix.

The native review and restored historical-report screens were visually inspected.
The exported PDF was checked through real text extraction for the independently
known role totals and original debt comparison. The explicit native exit file
records `0`. Failed-attempt diagnostics are retained separately.

Clean source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-card-completeness-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-card-completeness-source.bundle`, verified
against the final clean commit and remote main. Follow-up commits change only
native test fixtures and these validation records; application source remains
the exact feature commit validated on Windows.

This remains a development build. Recorded card evidence does not authorize final
approval; bank completeness, repayments and other household obligations retain
their separate requirements. Installed Windows and commercial acceptance remain
open. The last downloadable preview is still 0.23.0.

Next priority: a separately versioned card-inclusive final approval policy,
combining current bank completeness, this evidence and reviewed repayment
semantics without changing existing historical calculators or approval receipts.


## Provisional bank/card review (0.35.0, schema 24)

Financial Review → Bank & card activity adds durable card account CRUD/archive,
explicit statement assignments, one-to-one evidence-bound repayment matching,
permanent stale-link flags and exact calendar review. Format-8 internal assessments
optionally capture the reviewed card ledger and account/repayment history; internal
PDFs render the supplement. Formats 1–7 retain their old bytes/digests. Bank cash
and card spending remain distinct, with unavailable combined figures for review
issues. See [bank-card-review.md](bank-card-review.md).

Validated 2026-10-15 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing setup configuration reused |
| Production frontend / complete UI suite | Passed build and 193 tests |
| Complete Linux backend | Passed 205 library + 2 PDF-worker tests |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles |
| Complete native workflow | Passed, exit 0: all earlier workflows plus durable card account/source/repayment controls, independent cash-versus-spending figures, format-8 report/PDF capture, permanent stale bindings, rename/archive, restart, encrypted recovery and isolation |
| Windows source validation | Passed: 193 UI + 206 library + 2 PDF-worker tests; all five installer/checksum/upload steps skipped |
| Installer | Not created, per user instruction |

Feature commit: `040b8c7ee42a689ccbd57ebde9bc7c0d176f7fd3`.
Windows source checks passed in
[run 38791356512](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38791356512).
The Windows Rust step rebuilt for 11m03s and then passed its library tests in
657.37 seconds; its longer duration was not an unresolved test failure.

The first full checks exposed an older migration test fixture that needed to drop
the new table when simulating schema 22, and an existing budget navigation test
clicking before the save/busy transition had finished. Both fixtures were corrected
without weakening assertions; complete reruns passed. The focused PDF check uses
real text extraction rather than searching encoded PDF bytes. No dependency was
added. Native review and historical report screens were visually inspected.

Native artifacts: `/workspace/shared/desktop-smoke-bank-card`; logs and explicit
native exit file: `/workspace/shared/card-integration-`. The native exit is `0`.
The clean source/history checkpoint is retained at
`/workspace/shared/checkpoints/eisenbart-bank-card-review-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-bank-card-review-source.bundle`, verified
against the final clean commit and remote main. An earlier native-verified
checkpoint also preserves `dce59b62c5bd7a44918f8c1cbd20f206511c18e4` while Windows
was still running. Subsequent commits change documentation/validation records only.

This stage remains a development build. Its supplement is provisional; bank
figures and planning assumptions keep their original basis and card-inclusive
final approval stays blocked. The last downloadable preview remains 0.23.0.

Next priority: card-inclusive household inventory, reviewed balance/fact mappings
and a versioned completeness policy before final card approval. Keep v1 policy
and the new format-8 calculator stable for historical reports.


## Supervised credit-card CSV review (0.34.0, schema 23)

Documents → Credit-card statements now stores signed card liability sources,
explicit individual decisions, correction/rejection history and exact source versus
approved statement arithmetic. Payments reduce debt without becoming another
expense or income. Existing bank amounts remain unchanged. Known card evidence
flags bank-based reports and blocks v1 final approval until integrated card
reporting is implemented, preserving old payloads/digests. See
[credit-card-review.md](credit-card-review.md) for workflow, bounds and rules.

Validated 2026-10-15 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / complete UI suite | Passed build and 186 tests |
| Complete Linux backend | Passed 200 library + 2 PDF process tests |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles |
| Complete native workflow | Passed, exit 0: all prior workflows plus card mapping/decisions, exact liability/spending/payment totals, correction/rejection history, bank-report guard, restart, encrypted recovery and isolation |
| Windows source checks | Passed: 186 UI + 201 library + 2 PDF process tests; all five installer/checksum steps skipped |
| Installer | Not created, per user instruction |

The initial full UI run exposed an existing asynchronous test clicking the payment
review control while it was disabled during loading. It now waits until enabled,
retaining every assertion; the full rerun passed. Initial card recovery fixtures
used an in-workspace backup destination and were corrected to use a separate
folder, preserving the application's backup safeguard.

Feature commit: `2f955258fa40d2aa2234d0a690f64288cf0ae593`.
Windows source validation passed in
[run 38765328851](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38765328851).
All five focused card/report tests also passed after the final input-revision
boundary check; Clippy and the standalone build passed afterward. Native card
reconciliation and restored-history screens were visually inspected.

Native artifacts: `/workspace/shared/desktop-smoke-card-review`; logs:
`/workspace/shared/card-`. The final native exit file records `0`.
Clean source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-card-review-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-card-review-source.bundle`, verified
against the final clean commit and remote main. The final follow-up commit changes
only documentation and validation records.

This stage supports bounded signed UTF-8 CSV input, not card PDFs or combined
bank/card reports. The source remains a development build; the downloadable
preview remains 0.23.0. No dependency or installer was added.

Next priority: integrated card financial review and versioned report capture,
including explicit bank repayment links, cash versus spending views and complete
basis gates before card-inclusive final approval.

## Final counselor report approval (0.33.0, schema 22)

Implemented required readiness checks and explicit counselor approval in Reports
→ Final report checklist. A separate scoped immutable approval binds the exact
original assessment, inventory, account exceptions, professional decisions and
scenario/assumption agreement. Atomic audit, refreshed readiness receipts and
current-context flags preserve history. Offline final PDFs retain original
financial evidence, limitations and any later review flags. Complete financial
gates cannot be waived. See [workflow and bounded v1 policy](final-report-approval.md).

Validated 2026-10-15 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / complete UI suite | Passed build and 179 tests |
| Complete Linux backend | Passed 195 library + 2 PDF process tests |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles |
| Full desktop workflow | Passed, exit 0: original financial workflows, blocked incomplete case, complete final approval, exact PDF, changed-context flags, restart and encrypted recovery |
| Windows source checks | Passed: 179 UI + 196 library + 2 PDF process tests; all five installer/checksum steps skipped |
| Installer | Not created, per user instruction |

Feature commit: `3768fd7537f2867307d96a8fa1f4866b685c51d1`.
An initial standalone build exposed an incomplete typed UI test fixture. The
fixture was corrected in `4994305576567b50cece804f76df937cd137a019`;
its six tests and the production/standalone builds then passed. The superseded
Windows run was cancelled; validation targets the corrected source. Application
behavior did not change in the fixture correction.

Windows source validation passed for the corrected application in
[run 38758710446](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38758710446).
The final follow-up changes only documentation and test fixtures: live approval
fixtures now use the previous completed month; explicit freshness boundaries
remain independently checked. All three focused backend tests passed again;
Clippy and the standalone build were repeated. Application behavior is unchanged
from the source validated on Windows and in the full native workflow.

Native artifacts: `/workspace/shared/desktop-smoke-final-report`; logs:
`/workspace/shared/final-report-`. Approval confirmations, restored history and
the exported PDF's first page were visually inspected. The final native exit file
records `0`. No new dependency or installer was introduced. Clean source/history
checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-final-report-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-final-report-source.bundle`, verified
against the final clean commit and remote main.

This source remains a development build; the downloadable preview remains 0.23.0.
Final counselor approval under this policy does not certify the application for
commercial use or finalize an existing client action plan. Clean installed Windows
acceptance, broader evidence types and remaining security/release work are separate.

Next priority: additional document types, starting with supervised credit-card
statement review and independently checked liability/sign/repayment semantics.

## Saved completeness reviews (0.32.0, schema 21)

Implemented encrypted append-only completeness reviews with expected known and
unprovided accounts/obligations, applicable dates, selected coverage sources,
inventory notes, missing information and exceptions. Historical versions are
read-only; scoped optimistic saves, atomic audit, canonical validation and
recovery preserve them. Optional format-7 reports capture the exact latest review
for the same client/source/profile/period. Later review changes flag original
reports without replacing their bytes. Internal views/PDFs show captured context;
client-plan PDFs omit private review notes. See
[saved-completeness-reviews.md](saved-completeness-reviews.md).

Validated 2026-10-15 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / complete UI suite | Passed build and 172 tests |
| Complete Linux backend | Passed 192 library + 2 PDF process tests |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles |
| Full native workflows / saved completeness reviews | Passed, exit 0: financial flows, both recovery paths, review create/amend/read-only history, original report capture, restart, encrypted review recovery and historical PDF |
| Windows source checks | Passed: 172 UI + 193 library + 2 PDF process tests; installer build/install/upload steps skipped |
| Installer | Not created, per user instruction |

Feature commit: `73afb25e2d0004e3038218096a38e97ccf80b4b5`.
Windows source validation passed in
[run 38753167954](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38753167954).
Both saved-review and captured-report screens were visually inspected. Native
artifacts use `/workspace/shared/desktop-smoke-coverage-review`; logs use
`/workspace/shared/coverage-review-`. Source 0.32.0 remains a development build;
the downloadable preview remains 0.23.0. Installed Windows/offline and commercial
acceptance remain separate. Finalization remains unavailable: recording a review
does not certify complete financial coverage.

Next priority: final-report readiness and professional approval, with explicit
applicability/exception gates and immutable audited final exports.


## Financial completeness diagnostics (0.31.0, schema 20)

Financial Review now has a working completeness page for declared account/source
coverage, uncovered and overlapping dates, unknown periods, undecided rows and
arithmetic/reconciliation issues. Reports has a working final-report checklist
using each assessment's original captured evidence and current context flags.
Internal PDF exports include this review. Original report payloads/digests and
approved-only financial figures remain unchanged; no schema/dependency changes.
See [financial-completeness.md](financial-completeness.md).

This is a diagnostic stage: finalization remains blocked until expected inventory,
applicable periods, exceptions and professional review can be recorded. It does
not assert that every real-world account or obligation is present.

Validated 2026-10-15 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / complete UI suite | Passed build and 164 tests |
| Complete Linux backend | Passed 188 library + 2 PDF process tests |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles |
| Full native workflows / new review pages | Passed, exit 0: financial workflows, restart, both encrypted recovery paths, branding, live coverage, captured report checklist and historical PDF disclosures |
| Windows source checks | Passed: 164 UI + 189 library + 2 PDF process tests; installer steps skipped |
| Installer | Not created, per user instruction |

Feature commit: `411fa22a32b4a8599117212eff08fa6ce65abc1a`.
Windows source validation passed in
[run 38748867369](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38748867369).
Existing UI tests now await post-save dirty callbacks and enabled settings
controls before checking them; all assertions remain. The initial native run
selected the OCR tab by its old position after working subpages moved earlier.
The harness now selects that tab by name and retains the original planned-OCR
assertion. Application code is unchanged by this harness correction. The full
rerun passed with exit 0; both new pages were also visually inspected.
Native artifacts use `/workspace/shared/desktop-smoke-completeness-final`; logs
use `/workspace/shared/completeness-`. The prior failed run is preserved under
`desktop-smoke-completeness`. The downloadable preview remains 0.23.0; source
0.31.0 remains a development build. Installed Windows/offline and commercial
acceptance require separate release work.

That priority is implemented in 0.32.0 above; final-report approval remains separate.


## Firm/report branding (0.30.0, schema 20)

Settings → Branding now edits the firm display name, optional contact details,
normalized local PNG logo and disclosure with preview and version history. Rust
owns bounded image/text validation, organization scope, optimistic versions,
atomic audit/name updates and recovery validation. Report forms preview the saved
firm version; format-6 assessments capture it exactly and linked client plans
inherit it. Older report formats retain their payload bytes/digests; later edits
flag historical context without substituting current branding. Product branding
remains separate. See [firm-report-branding.md](firm-report-branding.md).

Validated 2026-10-15 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / complete UI suite | Passed build and 158 tests |
| Complete Linux backend | Passed 184 library + 2 PDF process tests |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles |
| Full native financial/branding/recovery workflows | Passed, exit 0: firm name/contact/PNG/disclosure, report and client-plan capture, restart, encrypted recovery and historical PDF export |
| Windows source checks | Passed: 158 UI + 185 library + 2 PDF process tests; installer build/install/upload steps skipped |
| Installer | Not created, per user instruction |

The feature commit is `cd56f10c9e2c81dec59aea143a1244a25536570b`.
Windows source validation passed in
[run 38685047534](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38685047534).
Native automation initially used a clipped text endpoint for the firm name and
then read a report selector before its asynchronous directory loaded. The test
now reads complete identity text and waits for the report control; application
behavior and assertions were preserved. The complete desktop rerun passed with
exit 0; the settings screen was also visually inspected. Application code was
unchanged by these native-test corrections.
Native artifacts use `/workspace/shared/desktop-smoke-branding-final`; validation logs
use `/workspace/shared/branding-`. PNG/base64 now directly name versions already
present in the lockfile; no package versions were upgraded.
The previously requested installer remains 0.23.0; source 0.30.0 is a development
build, with installed Windows/commercial acceptance still separate.

This next priority is implemented as conservative diagnostics in 0.31.0 above;
recorded inventory review and finalization remain separate work.


## Statement account management — 0.29.0 source (schema 19)

Implemented standalone checking/savings account creation, current-name editing,
archive/reactivate, source counts and read-only metadata history in Documents →
Statement accounts. Rust enforces client/organization/counselor scope, stale-write
checks, canonical/unique names, fixed original types and atomic audit. New imports
exclude archived accounts and staging rechecks status transactionally. Original
source identity, collection links, financial totals and saved report bytes remain
unchanged by metadata edits. Encrypted recovery validates every metadata version.
See [statement-account-management.md](statement-account-management.md).

Validated 2026-10-14 (America/Chicago):

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing configuration reused |
| Production frontend / complete UI suite | Passed build and 151 tests |
| Complete Linux backend | Passed 180 library + 2 PDF process tests |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed debug/no bundles |
| Full native financial/account/recovery workflows | Passed, exit 0; account create/rename/archive/reactivate/history and client isolation checked initially, after restart and after both restore routes |
| Windows source checks | Passed: 151 UI + 181 library + 2 PDF process tests; all installer build/install/upload steps skipped |
| Installer | Not created, per user instruction |

The initial smoke launcher lacked the activated desktop environment; it stopped
before application launch. Sourcing `scripts/cloud-env.sh` corrected the launcher,
and the complete workflow then passed. Native artifacts are in
`/workspace/shared/desktop-smoke-accounts`; validation logs use the
`/workspace/shared/accounts-` prefix. The account screen was visually inspected.
The feature commit is `d5c5c1626cd9a41ab587058943fc3b07076d7a1b`.
Windows source validation passed in
[run 38682529849](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38682529849).
This verifies Windows source compilation/tests; Windows installer/offline runtime
acceptance remains a separate release check when an installer is requested.

The latest previous installer remains 0.23.0; source 0.29.0 is a development build.
No dependency or immutable report-format changes were introduced.

Next priority: firm/report branding with preserved historical report payloads.
Remaining specification work is tracked in
[prototype-development-map.md](prototype-development-map.md).


## Document collection checklist — 0.28.0 source

Documents → Collection checklist now supports saved create/read/edit requests,
missing/received status with explicit receipt dates, optional scoped account and
source links, archive/reactivate, filters/counts and read-only version history.
Receipt remains separate from financial approval. Rust owns validation, assigned
IDs, authorization, revision checks and atomic audit. Schema 18 adds append-only
snapshots; encrypted restore validates original versions and links before
activation. No financial models, report payload formats or dependencies changed.
See [document-collection.md](document-collection.md).

| Check | Result |
|---|---|
| Locked setup | Passed; existing configuration/dependencies reused |
| Focused backend / interface | Four Rust and 31 checklist/desktop interface checks passed |
| Production frontend / complete UI suite | Passed build and 144 tests |
| Complete backend | Passed 176 library + 2 PDF process tests |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone native build | Passed, debug/no bundles |
| Native collection/financial/recovery workflows | Passed, exit 0: initial save, restart, encrypted restore and error-screen recovery |
| Windows source checks | Passed: 144 UI + 177 library + 2 PDF process tests; installer steps skipped |
| Installer | Not created, per user instruction |

Initial compiler errors in new record debug derivation
and a test session borrow were corrected without weakening assertions. Existing
legacy migration fixtures now also remove the newly added table when simulating
older schemas. The development map's earlier description of saved follow-up
removal was corrected: only unsaved follow-ups can be removed; saved items can
be cancelled and remain retained. No follow-up behavior changed. Logs use `/workspace/shared/collection-*.log`;
synthetic desktop artifacts are in `/workspace/shared/desktop-smoke-collection`.
The existing Vite single-chunk size advisory remains; no checks were weakened.
No environment draft changes were needed.
The new native request creation/receipt/archive/history/source/client-isolation
checks passed initially. A 1200px native screenshot was visually inspected: the
collection navigation, saved counts, filters, history and request table fit the
card and remain readable. Existing financial/PDF assertions continue in the full
restart and recovery workflow. The complete run passed (exit 0), including
all four collection source/history/client-isolation checks.

[Windows source run 38680069267](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38680069267)
passed for feature source `cb2938285c999117fb3f52528beb0c044b7a4c81`.
Installer build, installation smoke, metadata and both upload steps were skipped.
This establishes Windows source checks, not a new installed Windows build or
clean Windows 11 offline acceptance. The final follow-up commit records validation
only; application source is the tested feature checkpoint. A tracked-source
archive and complete-history bundle are retained under
`/workspace/shared/checkpoints/eisenbart-collection-source.*`, verified against
final clean/pushed HEAD. These are source checkpoints, not client-data backups.

Next priority: standalone statement-account metadata/lifecycle CRUD, then firm
branding and the remaining mapped functionality. No installer was created.

## Prototype structure: all main sections and future areas — 0.27.0 source

The user reprioritized a coherent CRUD prototype and complete page structure.
All twelve original main sections are now navigable. Focused client-context
pages reuse saved financial/document/report workflows; client profile/intake/
notes/history subsections and the complete record remain available. Settings
now has workspace, security, backup/recovery, branding, counselor/role, data,
licensing and help areas. Unimplemented features have named **Planned** pages.
The durable [prototype development map](prototype-development-map.md) records
current CRUD coverage, implementation entry points and per-feature acceptance
criteria for returning to each unfinished part. No backend/schema/dependency or
financial calculation change is introduced. No installer is being created.

| Check | Result |
|---|---|
| Locked cloud setup | Passed; unchanged configuration/dependencies |
| Production frontend / full interface suite | Passed build and 138 tests, including six new navigation/CRUD/administration checks |
| Complete backend | Passed 172 library + 2 PDF process tests |
| Formatting / Clippy | Passed, all targets with warnings denied |
| Standalone native desktop build | Passed, debug/no bundles |
| Native focused pages and integrated workflow | Passed, exit 0: all twelve pages, financial/PDF workflows, restart and both recovery paths |
| Windows source checks | Passed: 138 interface + 173 library + 2 PDF process tests; installer steps skipped |
| Installer | Not created, per user instruction |

An incorrect test label, unsupported role-query options and an incomplete saved-
budget fixture were corrected; behavior assertions stayed enabled. Vite reports
an advisory about the existing single application chunk exceeding 500 kB. Logs
use `/workspace/shared/prototype-*.log`; synthetic desktop artifacts are retained
in `/workspace/shared/desktop-smoke-prototype`. No environment draft change was
needed. Source checks do not establish clean installed Windows acceptance.
The native page check passed for all twelve sections, original saved planner
values, switching to an isolated empty client and back, planned OCR/branding
areas and report/admin separation. A 1200px desktop screenshot was visually
inspected for navigation/context/layout. Focused-page descriptions were then
clarified (text only); the production frontend build and all 40 App/import
interface checks passed again. Native functional and Windows checks target the
feature checkpoint; this later text/documentation refinement is tracked separately.
The complete native run finished with exit 0, including encrypted restore and
fresh-process error-screen recovery; all prior assertions remained enabled.
The final guidance-only revision also rebuilt successfully as a standalone native
application without bundles.
`AGENTS.md` now directs future development to consult and update the map.

[Windows source run 38678099058](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38678099058)
passed for feature source `5db67832f314b5116bc97d95639afc3c0c43a848`.
All installer build/install-test/metadata/upload steps were skipped. This does
not establish a new installed build, clean Windows 11 offline acceptance or
commercial readiness. The final refinement changes focused-page guidance and
development documentation only; no workflow/financial behavior was altered.
Source recovery checkpoints are retained under
`/workspace/shared/checkpoints/eisenbart-prototype-source.*`, verified against
final clean source and remote main; they are not client-data backups.

Next priority: persistent document collection checklist CRUD, followed by account
metadata/lifecycle work and remaining report/firm branding requirements. Page
coverage does not mean all original functionality or production release is done.

## Stage 3M: selected intake in immutable counselor reports — 0.26.0 source

Internal assessments now optionally capture explicitly reviewed address,
household and/or employment/income sections from a saved intake version. Source
validation binds captured values to original scoped care history; selected-field
changes permanently flag the original report even after reversion. Unselected
address/task changes do not flag intake, and client action-plan PDFs exclude it.
Financial calculations are unchanged. Schema 17/dependencies are unchanged;
format 5 adds intake while preserving absent-field legacy serialization.
See [report-intake-context.md](report-intake-context.md).

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing setup/dependencies reused |
| Focused backend / report interface | Three backend and 19 report interface tests passed |
| Production frontend / full interface suite | Build and all 132 tests passed |
| Complete backend | 172 library + 2 PDF process tests passed |
| Formatting / Clippy | Passed; all targets with warnings denied |
| Standalone desktop build | Passed, debug with no bundles |
| Integrated native workflow | Passed, exit 0: initial save, restart, encrypted restore and storage-error recovery |
| Windows source checks | Passed: 132 interface + 173 library + 2 PDF process tests; all installer steps skipped |
| Installer | Not created, per user instruction |

Initial fixture compilation, PDF whitespace assertions and a test initializer
lint were corrected without removing behavior checks. The PDF assertion now
normalizes extracted line wrapping while retaining the exact disclosure phrase.
Logs use `/workspace/shared/intake-report-*.log`; synthetic desktop artifacts
are in `/workspace/shared/desktop-smoke-intake-report`. No environment draft or
dependency changes were required. The new native intake selection, ignored
unselected-address/outcome edits, selected employer change/reversion and internal
versus client PDF checks passed before restart. The rendered internal intake PDF
page was visually inspected: original selected household/employment, version and
disclosure are readable within the page margins, above separate approved totals.
The attempted native intake screenshot captured the neighboring planning section;
it is not evidence of visual intake layout. DOM/value checks and the PDF checks
are the behavior evidence. The complete workflow passed (exit 0), including
selected intake and internal/client PDF assertions before restart and after
restart, encrypted restore and missing-metadata recovery. Existing financial
expected values and source-review assertions remained enabled.

[Windows source run 38675974295](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38675974295)
passed for feature source `4d8b0328d719390d5b4baa7035a63a7c5914b895`.
Installer build, installation smoke, metadata and artifact uploads were skipped.
This verifies source checks on Windows; clean installed Windows 11 offline
acceptance and commercial readiness remain open. This final follow-up commit
only records validation; application source is the tested feature checkpoint.
A tracked-source archive and complete Git-history bundle are retained at
`/workspace/shared/checkpoints/eisenbart-intake-report-source.tar.gz` and
`eisenbart-intake-report-source.bundle`, verified against final clean/pushed HEAD.
These are source recovery checkpoints, not client-data backups.

Next priority: organization contact details and branding in saved reports/client
plans, followed by remaining report/model/document and release requirements.

## Stage 3L: practice appointment/follow-up overview — 0.25.0 source

The dashboard now shows latest saved tasks/appointments across scoped clients,
exact overdue/today/upcoming/closed counts, explicit date/kind/name-title filters,
optional archived clients and stable 25-item pagination. Opening a row navigates
to its client and focuses the exact saved item. Existing care revision checks,
draft guards, private-state clearing and encrypted history remain in effect.
One authenticated read snapshot supplies client identity and current care data;
unreadable data/bounds fail explicitly. No schema/dependency or financial-model
change is required. See [follow-up-overview.md](follow-up-overview.md).

| Check | Result |
|---|---|
| Locked cloud setup | Passed; unchanged setup and dependencies |
| Focused Rust | Three passed: day/filter/page behavior and latest scoped data, archive exclusion, corrupt data and no writes |
| Focused interface | 24 passed across overview and desktop integration |
| Production frontend / complete interface suite | Passed build and 129 tests |
| Complete backend / formatting / Clippy | Passed: 169 library + 2 PDF process tests; formatting and warnings-denied Clippy |
| Standalone native build / integrated workflow | Passed: complete workflow, lock/restart, encrypted restore and missing-metadata recovery |
| Windows source checks | Passed: 129 UI tests, 170 library + 2 PDF process tests; all installer steps skipped |
| Installer | Not created, per user instruction |

Older generated application compiler caches were pruned before building; recent
incremental caches, dependencies, source and recoverable checkpoints were retained.
No environment configuration change was needed. Logs use
`/workspace/shared/followup-overview-*.log`; native artifacts remain in
`desktop-smoke-followup-overview`. The initial native overview checks passed:
opening the exact overdue task before completion, latest counts, closed-history
filtering, client/kind/date filters and appointment focus. Visual inspection at
1200px confirmed readable controls, counts, rows and pagination within the card.
The full workflow passed (exit 0), including all four overview checks through
restart, encrypted restore and storage-error recovery. It extends the intake example by
opening the overdue task from the dashboard before completion, then verifies
current counts/closed history, name/kind/date filters and exact appointment focus
after restart and both recovery paths, retaining prior financial/PDF assertions.

[Windows source run 38673896038](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38673896038)
passed for feature source `9f574460981c9baf6191ba8eb5cd8ab2570e1e04`.
Build/tests ran; installer build, installation smoke, metadata and artifact-upload
steps were all skipped. This does not establish a new installed Windows build
or clean Windows 11 offline acceptance. The final follow-up commit changes only
these validation notes; application source is the tested feature checkpoint.
A source archive and complete-history bundle are retained under
`/workspace/shared/checkpoints/eisenbart-followup-overview-source.*` and verified
against the final clean, committed source and remote main.

Next priority: selected intake context in immutable counselor reports, followed
by remaining report/model/document and release requirements. Background Windows
notifications, calendar sync and configurable practice time zones are not part
of this stage. No new installer will be made until the user asks for one.

## Stage 3K: optional client intake and dated follow-ups — 0.24.0 source

Optional address, household composition, employment/employer/occupation and
income context now sit beside each saved client. Tasks and appointments retain
explicit dates, optional appointment times, open/completed/cancelled status and
outcome notes. Rust calculates overdue/due-today counts using America/Chicago.
Saved items stay in append-only version history with scoped authorization,
optimistic revisions and atomic audit. Schema 17 migrates transactionally;
encrypted recovery validates every stored version before activation. Financial
models, approved facts and immutable reports retain their existing inputs.
See [client-intake-followups.md](client-intake-followups.md) for limits and usage.

| Check | Result |
|---|---|
| Locked cloud setup | Passed; existing dependencies/configuration reused |
| Focused backend | Five tests passed: validation/counts, scoped storage, migration/audit rollback, reopen/recovery and corrupted history |
| Focused interface | Five component checks passed |
| Production frontend / full interface suite | Final build and 123 tests passed, including intake navigation/lock privacy |
| Complete backend / formatting / Clippy | Passed: 166 library + 2 PDF process tests; formatting and warnings-denied Clippy |
| Standalone native build / workflow | Passed: full workflow, restart, encrypted restore and missing-metadata recovery; final spacing build/save/layout check also passed |
| Windows source checks | Passed: 123 UI tests, 167 library + 2 PDF process tests; installer steps intentionally skipped |
| New Windows installer / clean Windows 11 offline acceptance | Not run; existing 0.23.0 package remains separate |

Old-schema test fixtures now remove the schema-17 table when constructing an
earlier database, and migration success checks use the current schema constant.
No assertion about rollback, original financial records or report integrity is
removed. The first complete backend run passed 165 tests and failed only an
older migration assertion expecting schema 16 instead of the actual schema 17.
That expectation now uses the current schema constant; the full rerun passed all
166 library and both PDF process checks.
The original failed log remains `intake-backend.log`.

The first native run reached intake saving but submitted invalid years from the
Linux date widget: typing ISO produced `0003-01-02` in an isolated probe; native
month/day/year and segmented typing both produced the intended `2100-02-03`.
The application rejected the invalid input. The harness now uses the observed
widget format and verifies the complete ISO value before any save, without
retrying typing or saves. First-run logs/screenshots and the successful isolated
probe are retained; the fresh full native run passed (exit 0), including all four
intake/appointment/task checks and all pre-existing financial/PDF assertions.
The feature source checkpoint is `3856e38b21bc14b2b28886f43603bbfec15cf951`;
Windows source checks use that commit.
[Run 38671790102](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38671790102)
completed successfully; it produced no 0.24.0 installer.

Visual review found the intake card needed the same inner spacing as adjacent
panels. The final follow-up changes only CSS spacing/disclosure display, the date
entry harness and these verification notes. The standalone app was rebuilt with
that CSS, then a separate scoped native check saved intake and a dated follow-up,
verified overdue-to-completed behavior and captured both form sections for visual
inspection. The complete financial/recovery workflow ran before this CSS-only
adjustment; the final scoped check is not a second full workflow claim.
Logs/screenshots use `intake-final-layout*`. No backend rule or dependency changed.
A source archive and verified full-history bundle are retained under
`/workspace/shared/checkpoints/eisenbart-client-intake-source.*`, checked against
the final committed source. The repository and remote main point to that final
checkpoint after the results are recorded.
Logs use `/workspace/shared/intake-*.log`; native artifacts remain in
`desktop-smoke-client-intake` (first run) and `desktop-smoke-client-intake-verified`
(fresh run). No new dependency or cloud runtime is introduced.

Next priority: a practice-wide appointment/follow-up overview, then report/intake
integration and remaining original model/document/release requirements. These
client reminders currently appear inside the opened record; background Windows
notifications and a configurable practice time zone are not part of this stage.

## Stage 3J: temporary income funding/recovery and Windows preview — 0.23.0

Bounded temporary income reductions now retain explicit month ranges, reduced
per-payment assumptions, monthly funding deficits and normal recovery. Normal
budget/debt/savings forecasts remain conditional on continuing commitments;
no savings drawdown or automatic payment cut is inferred. Explicitly selected
matching paycheck schedules change only affected dated receipts, preserving
original dates and normal amounts outside the period. Saved scenarios/reports
and recovery validate captured period results and receipt provenance. Client
agreements require a separate reviewed monthly-allocation report. Older empty-
extension records remain unchanged; schema 16 and dependencies are unchanged.
See [combined-scenario-comparisons.md](combined-scenario-comparisons.md).

The native integrated example keeps all prior expense/extra/goal assertions,
then models November at 800.00 biweekly: monthly income 1,733.33 and funding
-975.00; December normal recovery buffer 858.34. Explicit receipt selection cuts
dated closing from 3,225.00 to 1,825.00. Approved activity stays 1,820.02. Earlier
normal plans and conditional payoff/goal milestones stay unchanged.

| Check | Result |
|---|---|
| Locked setup | Passed; existing configuration/dependencies reused |
| Focused financial checks | Three temporary-income comparison checks passed |
| Focused saved/report/recovery check | Passed; original bytes/period integrity, PDF, restart/recovery and agreement guard |
| Focused interface checks | 22 passed across comparison and client-plan components |
| Production frontend / complete interface suite | Passed build and 117 tests |
| Complete Linux backend / PDF processes | Passed: 161 library + 2 process tests (163 total) |
| Formatting / Clippy, warnings denied | Passed |
| Standalone native build / complete workflow | Passed: complete fresh native run, restart, encrypted restore and missing-metadata error-screen recovery |
| Windows installer workflow | Passed: 117 UI, 162 library + 2 process tests; NSIS build, install, responsive x64 window, uninstall and download metadata |
| Clean Windows 11 offline/full GUI release acceptance | Not run |

The first saved-record test compile failed because its error assertion required
Debug on the successful review type. The fixture now inspects the error Option
without changing production types or relaxing the assertion; its focused rerun
passed. The first compile log is retained. No financial rule or timeout changed.

The first full native run passed the new income controls/report but failed a
later retained PDF-destination replacement: the app reported an unavailable
parent folder, and the screenshot still began with the earlier export path.
The exact submitted value was not captured, so the cause is not established.
An isolated controlled-text-field probe passed both clear and keyboard replacement.
The harness now uses one native select-all/delete gesture, observes the empty
field, types once and checks the entire destination before export; no save,
financial mutation or export is replayed. The fresh full run passed that
previously failing export and all restart, encrypted restore and error-screen
recovery checks (exit 0). Application and installer source remain `bd2d1a88544ea941e69373f6312cf275d3767186`;
this follow-up changes the test harness and documentation only. The first log,
screenshot and failed exit status remain preserved; no assertion was removed.
Logs use `/workspace/shared/income-period-*.log`; first native artifacts remain
in `desktop-smoke-income-periods`, fresh-run artifacts in
`desktop-smoke-income-periods-verified`. Current Windows/evaluation/download
instructions use schema 16 and the actual recovery layout.

[Windows packaging run 38668750314](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38668750314)
completed successfully for the source above. All packaging, installed-window,
uninstall, metadata and upload steps passed.
[Download artifact 11943645145](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38668750314/artifacts/11943645145)
expires October 29, 2026 at 02:12 UTC. The downloaded ZIP passed its GitHub digest
and CRC checks; its README identifies version 0.23.0, that source and schema 16.
The installer is `Eisenbart Counseling Suite_0.23.0_x64-setup.exe` (222,803,905
bytes), unsigned, with SHA-256:
`83da0ac3773869ada8e8d8b5099a54ffbf8a4bfca58b3b42f1a63a723dc2aed1`.
The checksum matches its included `SHA256SUMS.txt`; trusted archive inspection
passed and found the 214,090,448-byte WebView2 offline redistributable. Its
presence does not establish installation on a clean offline Windows 11 machine.
A source archive and complete-history Git bundle are retained under
`/workspace/shared/checkpoints/eisenbart-income-period-source.*`; both are verified
against the final committed source after documenting these results.
Windows logs confirm installer header 0x014C, installed application 0x8664,
installation/uninstallation exit 0 and a responsive titled window. No counselor
setup/client workflow was entered on Windows; those limits remain open.

Next priority after this preview: practical household intake and follow-up
workflow for pilot use, then remaining model/report requirements and Windows
acceptance feedback. Seven major roadmap areas remain; this is an evaluation
build, not completion of the original specification or commercial readiness.

## Stage 3I: recurring-expense proposals and explicit dated bills — 0.22.0 source

New necessary/discretionary recurring expenses now recalculate combined budget,
debt capacity, emergency targets and savings affordability. Optional explicit
account/first/last/second-day inputs add estimated bills to the proposed dated
forecast with the same amount/frequency. Existing obligations stay in effect;
no live plan, approved fact or agreement changes. Saved scenarios and selected
reports retain classifications, payment bindings, canonical inputs and outcomes.
Earlier empty-extension records/warnings remain compatible; schema 16 is unchanged.
See [combined-scenario-comparisons.md](combined-scenario-comparisons.md) for scope,
exact examples, calendar limits and monthly versus dated forecast assumptions.

The integrated native example retains earlier extra-payment/goal controls and
adds 75.00 child care: buffer 833.34 → 758.34, emergency target 5,124.99 with
December 2027 completion. Explicitly scheduling the November bill changes closing
3,300.00 → 3,225.00; debt interest stays 12.20, goal completion December 2026 and
approved operating activity separately 1,820.02. Budget-only additions preserve
dated closing. A separate weekly 12.01 example verifies monthly 52.04 versus four
actual payments totaling 48.04. Unknowns, zero, deficits, limits and out-of-period
schedules remain explicit. Annual expenses are budget-only.

| Check | Result |
|---|---|
| Locked setup | Passed; existing environment configuration/dependencies reused |
| Focused backend | Three new comparison checks and one saved-record/PDF/recovery check passed |
| Focused interface | 14 comparison/control tests passed, including three new expense checks |
| Production frontend / complete interface suite | Passed build and 113 tests |
| Complete Linux backend / PDF workers | Passed: 157 library + 2 PDF-process tests (159 total) |
| Formatting / Clippy, warnings denied | Passed |
| Standalone native build / complete workflow | Passed build and complete direct native run, exit 0: four expense comparisons and integrated report PDFs, restart and both recovery paths; earlier workflows retained |
| Windows source CI | [Run 38666523905](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38666523905) passed: production frontend build, 113 interface tests, 158 library + 2 PDF-process tests (160 total); installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

The existing integrated native proposal/report workflow is extended rather than
creating another parallel report workflow. Prior extra/goal assertions run before
the expense is added; older proposals and client reports remain separate. Full
regression checks still follow repository requirements. Feature details are
consolidated in the existing combined-scenario document.

Validation logs use `/workspace/shared/recurring-expense-*.log`; native completion
is recorded in `recurring-expense-native.log`/`.exit` and artifacts in
`desktop-smoke-recurring-expenses`. The original extra/goal warning
was separately checked byte for byte against the prior source for proposals
without new dated expenses. Four integrated `recurring-expense-*.pdf` exports
preserve 758.34/3,225.00, explicit bill bindings, earlier extra/goal assumptions and
separate approved totals. The rendered initial PDF and native screenshot were
visually checked for readable payment assumptions, comparison figures and target
milestones. No test failure or test deadline/assertion relaxation was required.

Application source checkpoint: `c2afa2cb06e0c216e19e62a8dfbbc6d73aff382a`.
All application checks use this unchanged source. Windows source validation
completed successfully on this exact commit in
[run 38666523905](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38666523905),
including its Windows-specific library check. Evidence includes
`recurring-expense-windows-validation.log`, `recurring-expense-windows-job.log`
and `recurring-expense-windows-result.json` under `/workspace/shared`.
No installer was built or uploaded.
Internal source/full-history checkpoints are
`checkpoints/eisenbart-recurring-expense-source.tar.gz` and
`checkpoints/eisenbart-recurring-expense.bundle` under `/workspace/shared`.
Final verification documentation does not change application code. The source
archive is verified against all 157 tracked files byte for byte; the bundle
retains complete history. Clean Windows installation/offline release acceptance
remains a separate pending requirement.

Next priority: **temporary income-reduction scenarios** with explicit period scope,
then remaining financial-model/report requirements. Seven major roadmap areas
remain; this checkpoint does not complete the master specification or commercial
release acceptance. No new installer is created during feature work.

## Stage 3H: extra debt payments and savings-goal proposal controls — 0.21.0 source

Combined proposals now support additive dated debt extras and explicit monthly
contribution changes to existing savings goals. Rust validates original owned
baselines and all four versions, retains saved debt events/goal targets/reserves,
recalculates existing exact-cent models and returns canonical changes. Goal
contributions, completion dates, forecast status and target applicability appear
side by side. Saved scenarios and selected report PDFs retain those assumptions,
including explicit added-payment/changed-goal descriptions. No live plan,
approved fact, agreement or dated cash-flow schedule is changed.

Schema 16 and existing payload/calculator versions remain compatible: new arrays
default empty and are omitted when empty, preserving legacy field order and the
original six warnings. Extended proposals append a funding/commitment warning.
Extras follow the shared strategy budget, do not target a named account, and
reserve funding conservatively even after an early projected payoff. Unknown
contributions remain unknown; zero is explicit. See
[scenario-payment-goal-controls.md](scenario-payment-goal-controls.md).

Independent zero-interest example: income 3,250.00, spending 1,200.00, debt 100.00,
emergency 200.00 and goal 50.00 yield a 1,700.00 first-month buffer. Adding a
150.00 November extra and changing the goal to 100.00 yields 1,500.00. Retaining an
existing 25.00 December extra, the 1,000.00 loan finishes in July instead of August;
the goal's remaining 200.00 finishes in December instead of February. Target and
reserve remain unchanged. The native fixture's higher 400.00 budget debt reserve
produces 983.34 → 833.34 when a 300.00 extra makes model debt 500.00 and the goal
changes to 100.00; proposed interest is 12.20 and payoff December 2026. Approved
operating activity remains separately 1,820.02 and dated closing 3,300.00.

| Check | Result |
|---|---|
| Locked setup | Passed; existing reusable setup remains sufficient, no new environment configuration or dependency |
| Production frontend build / interface suite | Passed build and 110 tests; three added control/save/unknown/reset/limit checks |
| Focused backend checks | Nine comparison and nine saved-record/report/recovery checks passed, including four new tests |
| Complete Linux backend / worker suite | Passed: 153 library + 2 PDF worker tests (155 total) |
| Formatting / Clippy, warnings denied | Passed |
| Native compile / complete workflow | Passed standalone debug build and complete direct native run, exit 0: four extended comparison/PDF checks, restart, normal encrypted restore and damaged-workspace recovery; all prior workflows retained |
| Windows source CI | [Run 38664552864](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38664552864) passed: production frontend build, 110 interface tests, 154 library + 2 PDF worker tests (156 total); installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

Initial focused checks exposed two fixture errors: the absent-plan test needed a
saved budget to reach the intended validation branch, and the frontend mock
needed canonical returned money strings rather than echoing unnormalized input.
Both fixtures were corrected without changing assertions, timeouts or financial
calculations. Their first failure logs remain available; focused reruns passed.

Logs under `/workspace/shared`: `scenario-controls-setup.log`,
`scenario-controls-ui.log`, `scenario-controls-ui-tests.log`,
`scenario-controls-ui-focused.log`, `scenario-controls-backend-focused.log`,
`scenario-controls-store-focused.log`, `scenario-controls-backend.log`,
`scenario-controls-fmt.log`, `scenario-controls-clippy.log`,
`scenario-controls-final-comparison.log`, `scenario-controls-final-compatibility.log`,
`scenario-controls-ui-final.log` and `scenario-controls-native-build.log`. Final
comparison/recovery tests were repeated after the last warning/PDF validation
wording adjustment. No arithmetic or model limits changed.
The two initial focused failures have the `-first.log` suffix. Complete native
results are `scenario-controls-native.log`/`.exit`; artifacts are in
`desktop-smoke-scenario-controls`. Four `scenario-controls-*.pdf` exports preserve
833.34, 12.20 interest, December payoff/goal completion and separate approved
1,820.02 operating activity. The earlier four selected-proposal PDFs and all
prior report/client-plan/source/recovery checks also passed in the same run.
Rendered PDF inspection confirmed readable added-payment/goal descriptions,
scenario identity and comparison figures; the native screenshot confirmed the
new goal comparison rows and 500.00 debt reserve.

Application source checkpoint:
`381fc8a6db58a7c2601f9d283c9e991ddd82fbe3`. Linux full-suite checks passed before
the final explanatory warning and defensive PDF goal lookup adjustment; affected
comparison and saved-record/PDF/recovery checks were repeated successfully on
this final source. Formatting/Clippy also passed, followed by the standalone
native build/run.
Windows source validation completed successfully on that exact commit in
[run 38664552864](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38664552864),
including the Windows-specific library check. Logs under `/workspace/shared`:
`scenario-controls-windows-validation.log`, `scenario-controls-windows-job.log`
and `scenario-controls-windows-result.json`. No installer was built or uploaded.
Internal source/full-history checkpoints are
`checkpoints/eisenbart-scenario-controls-source.tar.gz` and
`checkpoints/eisenbart-scenario-controls.bundle` under `/workspace/shared`.
Final documentation updates do not change the validated application source.
The final source archive is checked against all 156 tracked files byte for byte;
the bundle retains complete history. Clean Windows installation/offline release
acceptance remains a separate pending requirement.

Next priority: **new recurring-expense proposals**, then temporary income changes,
with exact monthly/datetime scope and unchanged historical facts. Seven major
roadmap areas remain; this checkpoint does not finish the original specification
or establish commercial readiness. No installer is built during feature work.

## Stage 3G: saved combined scenarios and report selection — 0.20.0 source

Reviewed combined proposals can now be recorded independently of live plans and
reopened as immutable baseline/proposed comparisons. Records retain client/time,
all four baseline versions, canonical changes/assumptions, complete model results
and warnings. Later live-plan revisions flag history Needs review without
rewriting values. Counselor assessments can explicitly select a current saved
scenario by ID/digest, capture its proposed model set and export selected
comparison context and milestones as an offline internal PDF. Approved activity
remains separate. See [saved-combined-scenarios.md](saved-combined-scenarios.md).

Schema 16 adds append-only owned scenario records and compound report links. A
scoped immediate transaction checks history count and baseline versions, performs
Rust calculations, and records payload/digest/audit atomically. The count guards
uncertain save responses; mutations are never automatically retried. Read/recovery
recompute captured models and verify scope/metadata/digest/arithmetic/linkage.
Failed audit, stale context or invalid selections leave no partial record/link.
Format 4 reports support selected proposals and optional session/payment evidence;
formats 1–3 and existing client agreements remain unchanged. Unknown projections
stay unavailable; unsupported selected repayment projections cannot enter reports.
No dependency or installer is introduced.

The earlier native blocker is resolved: a direct WebKit driver connection uses
the same binary capabilities and automation flags as tauri-driver, removing its
pooled forwarding hop. The complete pre-stage 0.19.0 workflow passed once, exit 0,
including the formerly interrupted final report check and both recovery paths.
Assertions and no-replay rules remain intact; the older forwarding mode stays
available only for diagnosis. Its previous failure logs are preserved.

Independent backend example: income 3,250.00 → 3,466.67; buffers 1,700.00 →
1,816.67; proposed snowball interest 23.01/payoff February 2027/emergency
completion May 2027, while approved report activity stays 0.00. Native data has
budget debt reserve 400.00, preserving 766.67 → 983.34 buffers and 35.35 → 23.01
interest, with November 2027 emergency completion and 3,300.00 dated closing.
The selected report retains separate approved operating activity 1,820.02.

| Check | Result |
|---|---|
| Locked setup / production frontend build | Passed |
| Interface suite | 107 passed; five new history/save/selection/stale/client-isolation checks plus prior financial/private-data guards |
| New backend checks | Eight scenario store/report/PDF/recovery/upgrade/corruption/audit/limits checks and nine migration-focused checks passed |
| Complete Linux backend / worker suite | 149 library + 2 worker tests passed (151 total); new scenarios, report/PDF, recovery, schema 1–16 compatibility and all earlier financial workflows |
| Formatting / Clippy, warnings denied | Passed |
| Native compile / complete new workflow | Passed standalone debug build and complete direct native run, exit 0: saved scenario/report/PDF selection, unchanged live/approved data, historical flags, restart and both recovery paths, with all previous workflows |
| Windows source CI | [Run 38648010362](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38648010362) passed: production frontend build, 107 interface tests, 150 library + 2 worker tests (152 total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

Initial complete backend validation found 15 legacy migration fixtures that still
held schema 16 tables while declaring an older version. Fixtures now remove both
new tables before simulated downgrades; collision/rollback, original-data and
append-only assertions are unchanged. The failed run is preserved separately.
The frontend global-lock fixture now mocks the added history read, and an existing
statement-review fixture awaits its asynchronous mount before editing, matching
the file’s established setup helper. No financial assertion or timeout is relaxed.

Native verification created four selected-scenario PDFs (initial/restart/normal
restore/damaged-workspace recovery), preserving 766.67 → 983.34 buffers and the
separate 1,820.02 approved report basis. Saved comparisons reopened with original
versions/outcomes and Needs review after live budget changes. All prior source,
report, agreement, lock and recovery workflows passed in the same complete run.
Rendered PDF inspection confirmed readable selected identity, comparison values,
Chicago time, preserved warnings and separate planning assumptions.

All application checks validate source commit
`02dfd5b83c2104c0796c9d71176e584235e5b5b5`. Windows source CI completed successfully
on that exact commit; it includes the Windows-specific library check. The final
checkpoint only updates this verification documentation. No installer was built,
uploaded or downloaded, and clean Windows installation acceptance remains open.

Logs under `/workspace/shared`: `saved-scenarios-setup.log`,
`saved-scenarios-baseline-native.log`/`.exit` (complete pre-stage check),
`saved-scenarios-ui.log`, `saved-scenarios-ui-tests.log`,
`saved-scenarios-backend.log`, `saved-scenarios-new-tests.log`,
`saved-scenarios-migration-focused.log`, `saved-scenarios-fmt.log`,
`saved-scenarios-clippy.log`, `saved-scenarios-native-build.log`,
`saved-scenarios-native.log`/`.exit`, `saved-scenarios-windows-validation.log`,
`saved-scenarios-windows-job.log` and `saved-scenarios-windows-result.json`.
The initial fixture failure run is
`saved-scenarios-backend-first.log`. Native artifacts are in
`desktop-smoke-direct-baseline` and `desktop-smoke-saved-scenarios`.
Source/full-history checkpoints are `checkpoints/eisenbart-saved-scenarios-source.tar.gz`
and `checkpoints/eisenbart-saved-scenarios.bundle` under that same shared directory.

Next priority after this checkpoint: **further scenario proposal controls**,
including extra payments, new recurring expenses, goal contributions and temporary
income changes, preserving exact calculations and immutable report provenance.
Seven major roadmap areas remain; this advances financial models/reports without
completing the original specification or establishing commercial readiness.

## Stage 3F: temporary combined scenario comparisons — 0.19.0 source

Counselors can now explicitly load all four saved models and compare them side by
side with a temporary proposal. Existing budget amounts/frequencies, debt base
payment and strategy, primary emergency contribution, and explicit dated-entry
amounts/dates can be adjusted. The comparison shows allocation buffers, repayment
cost/payoff/status, savings target/completion/status and dated closing/warning
counts, with canonical changes and both sets of model limitations. Scratch inputs
reset across clients/locks/restart and live-plan edits. Historical scenarios,
reports, client agreements, approved facts, live plans and audit events remain
unchanged. See [combined-scenario-comparisons.md](combined-scenario-comparisons.md).

Rust reads a scoped atomic baseline, checks all four expected versions and
canonical model calculations, validates bounded indexed input-only proposals and
recalculates shared budget/debt/savings/cash-flow contexts in order. It preserves
exact cents/rounding, unknowns and unsupported terms. Changed budget/dates are
estimates; cash-flow entries are never automatically synchronized. Existing future
debt changes/extras and savings goal/reserve commitments stay in force. The
interface blocks competing writes/unfinished edits, invalidates old comparisons,
retains failed drafts and rejects late/wrong-client/version/calculator replies.
No migration/dependency/installer is introduced; schema remains 15 and historical
`entered-plans-v1` captures stay unchanged. This preview uses `combined-preview-v1`.

Independent examples include 1,600 biweekly → 3,466.67; income/spending proposals
propagate to the savings allocation check; debt 200.00/primary savings 300.00 plus
retained goal 50.00 yield 1,816.67 buffer in the backend fixture, or 983.34 in
the native fixture with 1,633.33 necessary spending. Proposed snowball interest
23.01/payoff February 2027 compares with baseline avalanche 35.35. A delayed bill
removes the projected shortfall without writing its saved date or changing the
3,300.00 closing. Future debt changes/extras remain reserved and later deficits
are visible. Unknown values and missing optional models remain unavailable.

| Check | Result |
|---|---|
| Locked setup / production frontend build | Passed |
| Interface suite | 102 passed; proposal input-only IPC, canonical outcomes/unknowns, preserved failures, dirty/concurrent guards, reload and client/lock/version isolation |
| Linux backend / worker checks | Full 138-library + 2 worker run passed; subsequent six focused scenario tests passed, including three added retained-commitment/absent-model/higher-reserve checks (141 unique library tests covered) |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile / feature workflows | Standalone build and all four new comparison checks passed: initial/restart/normal restore/damaged-workspace recovery. Complete native run remains incomplete: WebDriver disconnected on a click opening the older counselor-report panel after the fourth comparison |
| Windows source CI | Passed b7fd415: [run 38641952017](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38641952017), 102 interface + 142 library + 2 worker tests (144 backend/process total). All installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

The first native run exposed a locale-dependent date control; proposals now use
explicit YYYY-MM-DD text with Rust validation. The second exposed incorrect
harness expectations: savings reserves the higher budget/model payment, yielding
983.34, and the correct emergency date is November 30, 2027. The third passed all
four new comparisons and unchanged-input checks, including both recovery paths,
then the driver closed an uncertain POST click while opening Casey’s existing
report panel. The final counselor-evidence export/dashboard checks and whole-run
completion were not reached. No click or financial mutation was replayed. Following
the original specification’s three-attempt limit, further full reruns are deferred
and the blocker is preserved. Click errors now include their intended selector for
future diagnosis without changing assertions or retrying mutations.

Shipping application source is `b7fd4151061b86cba7df3d5a6d8b78909008f9ae`; final
checkpoint changes only documentation and click-error diagnostics.

Visual review confirmed both columns, 400.00 reserved debt, 766.67 → 983.34 buffer,
35.35 → 23.01 interest, June 2028 → November 2027 emergency dates and three → zero
cash-flow warning days. The full-run exit is 1; the four feature checks pass, but
this must not be described as a completely passing native regression run.

Logs under `/workspace/shared`: `scenario-comparison-setup.log`, `scenario-comparison-ui.log`,
`scenario-comparison-backend.log`, `scenario-comparison-focused.log`,
`scenario-comparison-fmt.log`, `scenario-comparison-clippy.log`,
`scenario-comparison-native-build.log`, `scenario-comparison-native.log`/`.exit`,
`scenario-comparison-windows-job.log`, `scenario-comparison-windows-result.json`
and `scenario-comparison-windows-verified-validation.log`.
Earlier failures are `scenario-comparison-native-first.log`/`.exit` and
`scenario-comparison-native-second.log`/`.exit`; final artifacts are in
`desktop-smoke-scenario-comparison-verified`. Source/full-history checkpoints are
`checkpoints/eisenbart-combined-scenarios-source.tar.gz` and
`checkpoints/eisenbart-combined-scenarios.bundle` under the same shared directory.

Next priority: **diagnose the native-driver interruption and complete full workflow
validation, then persistent combined scenarios and explicit selection for reports**,
retaining input assumptions, versions, outcomes and uncertainty independently of
approved history. Seven major roadmap work areas remain; this checkpoint advances
the financial-model area without completing it or establishing release readiness.

## Stage 3E: optional budget method comparisons — 0.18.0 source

The saved-budget panel now offers read-only zero-based, 50/30/20 and custom
percentage references. Required minimum debt is classified from existing planned
debt into needs; savings and extra debt share the future bucket. The zero-based
check shows assigned/unassigned income or deficit without assigning funds.
A side-by-side table shows entered amounts, targets, exact signed differences and
reference positions. These optional guidelines never change plans, agreements,
reports, approved sources or audit events. Inputs/results are temporary and reset
across clients/restart; the saved budget remains persistent.

Rust rechecks authenticated scope, saved revision and canonical budget arithmetic.
IPC accepts strings/revision only. Percentages use integer basis points, must total
exactly 100.00%, and targets use largest-remainder cents (ties needs/wants/future)
so the target sum exactly equals entered monthly income. Unknown income/amounts
and minimums withhold affected figures, rather than becoming zero or compliance.
Estimates, incomplete expenses, zero income and separate model assumptions stay
explicit. Failed inputs are retained; edits hide old results, competing writes are
blocked, and stale budgets can be reloaded without a save. No migration/dependency
or installer is added. See [budget-method-comparisons.md](budget-method-comparisons.md).

Independent example: income 3,250.00, necessary 1,633.33, wants 200.00, savings
300.00/debt 400.00; zero-based assigned 2,533.33/remaining 716.67. Minimum debt
100.00 gives needs 1,733.33/future 600.00. Standard targets 1,625.00/975.00/650.00
produce differences +108.33/-775.00/-50.00; custom 60/20/20 needs target 1,950.00
produces -216.67. For 1.01 at 33.33/33.33/33.34, targets 0.34/0.33/0.34 conserve
all cents. Known balanced/deficit/zero, missing income/expenses, unknown minimums,
invalid precision/percent sums, large aggregate amounts, and cent conservation
are independently checked.

| Check | Result |
|---|---|
| Locked setup / production frontend build | Passed |
| Interface suite | 96 passed; lazy/input-only comparison, canonical figures, failed assumptions, unknowns, client/version mismatch/late replies, dirty/concurrent guards and read-only budget reload |
| Linux backend / worker suite | 135 library + 2 worker tests passed (137 total); four pure framework checks plus scoped/stale/locked/no-audit-write storage verification |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile / complete workflow | Passed standalone debug build and full native run, exit 0: exact comparisons/unknowns/failed assumptions with unchanged plans, all report/client workflows, restart and both recovery paths |
| Windows source CI | Passed source 2e07be0: [run 38637078036](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38637078036), 96 interface + 136 library + 2 worker tests (138 backend/process total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

The initial Windows run stopped during frontend checks: the first application
CSV-lock test and an assessment test hit the existing five-second deadline, then
pending application work interfered with later test isolation. Vitest now limits
concurrent DOM workers to two; no timeout or financial/privacy assertion is
removed or relaxed. The source application is unchanged by this runner adjustment.
The corrected Windows run passed all 96 interface tests and the complete
136-library/2-worker backend/process suite.

Native visual review confirmed the side-by-side targets/differences and intact
2,533.33 assignment/716.67 remaining balance. Four visible workflows before and
after restart/normal restore/damaged-workspace recovery verified temporary input
reset, unknown minimums, exact classification and rejected 101% assumptions.
All previous financial, internal/client PDF, source review and recovery checks
passed in the same complete native run. Shipping application source is
`c1f760ac89d51e9d32db6ccf39c7d7b0be30f7aa`; source `2e07be0` adds only the
bounded test-runner configuration and verification notes.

Validation logs are under `/workspace/shared`: `budget-methods-ui.log`,
`budget-methods-backend.log`, `budget-methods-clippy.log`,
`budget-methods-native-build.log`, `budget-methods-native.log`/`.exit`, and
`budget-methods-windows-job.log`. The initial Windows failure log is
`budget-methods-windows-failed.log`. Final native artifacts are in
`desktop-smoke-budget-methods`. Source/full-history checkpoints are
`/workspace/shared/checkpoints/eisenbart-budget-methods-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-budget-methods.bundle`.

Seven major work areas remain in the roadmap: further financial models/scenarios,
document/OCR review, fuller intake/workflow, security/permissions, commercial
branding/support, report refinements and Windows release acceptance. Each requires
smaller checkpoints; this feature does not complete the whole financial-model area.
See [remaining-requirements.md](remaining-requirements.md). Original stages 3–8 are
still partly open; the count is not a time or percentage estimate.

Next priority: **combined non-writing scenario comparisons**, building on entered
budget/debt/savings/cash-flow models while preserving historical records.

## Stage 5G: expanded counselor report evidence — 0.17.0 source

Internal assessments can now optionally capture selected saved session notes and
current merchant/payment groups with the latest counselor decision per reviewed
group. Notes retain their author, original time and ID; decisions retain label,
reason, cadence/subscription, applicability and original evidence digest with
record/approval/source/page/line bindings. Capture retains older approvals needed
by a reviewed payment after later corrections. All imported dates are labeled
separately from the financial assessment period. Narrative money, suggested cadence
and historical subscription decisions never enter approved totals or active plans.

The report viewer and offline internal PDF show these sections separately from
approved activity and hypothetical planning. Evidence times use America/Chicago;
UTC timestamps remain stored. Latest decisions without a current approved group
are explicitly historical. An invalidated decision stays Needs review even when
identical source values return. Client PDFs exclude internal note/payment evidence.
See [counselor-report-evidence.md](counselor-report-evidence.md).

Payload format 3 uses optional supporting context `counselor-evidence-v1`; formats
1/2 remain supported and unchanged. Records without this option continue to save
format 2. Schema stays 15. The existing scoped append-only assessment transaction
and audit check source/profile/plan versions and expected note count, then read up
to ten selected original notes by scoped IDs. Preview shows the latest 100 notes.
Reads/export/recovery recheck note identity against original append-only storage,
canonical text/time and recomputed payment groups/totals/states. New notes flag
supporting report context without replacing selected notes; legacy reports do not
claim notes or gain that new warning. Linked action plans inherit review state.
No new dependency or installer is added.

Independent example: historical payments 10.01 + 12.03 + 10.01 = 32.05 and operating
cash flow -32.05. A selected note mentioning 9999.99 remains narrative only.
Correcting the first payment to 9.99 yields current total 32.03 while retaining the
original reviewed 10.01 approval; withdrawal yields 22.04 without deleting it.

| Check | Result |
|---|---|
| Locked setup / production frontend build | Passed |
| Interface suite | 91 passed; new selection-only IPC, historical context/escaped notes, unchanged figures, failed draft/renewed acknowledgment, note cap, mismatched source/client and late reply tests |
| Linux backend / worker suite | 130 library + 2 worker tests passed (132 total); four new report/PDF/provenance/scope/forgery/restart/recovery checks; relevant PDF checks repeated after adding Chicago evidence timestamps |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile / expanded report workflow | Passed standalone debug build and final full native run, exit 0: selected notes, preserved prior review state after reconfirmation, new-note flags, four counselor PDFs, existing planning/client PDFs, restart and both recovery paths |
| Windows source CI | Passed shipping source 0abe32e: [run 38632476167](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38632476167), 91 interface + 131 library + 2 worker tests (133 backend/process total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

The first native run created the expanded report but expected a Needs review group
after the fixture had already explicitly reconfirmed it. The harness now captures
before reconfirmation, so the original historical status/reason can be checked
against the later current confirmation. A second run lost the driver connection
at a preexisting CSV click; driver logs and screenshot showed a responsive app and
connection reset. The uncertain click was not replayed. Each attempt uses a fresh
disposable fictional workspace; the corrected full run passed with exit 0.

Windows-validated shipping source is
`0abe32e3a6930a727442a73a81321b99eedc36a5`. Later changes concern the native
fixture timing and verification documentation.

Final UI/PDF visual review confirmed the selected note/author/Chicago timestamp,
original payment dates, historical state/reason and source/approval bindings. The
six-page Letter counselor PDF embeds local Unicode Open Sans, with no forms,
JavaScript or original attachments. It preserves 32.05 historical spending and
-32.05 operating cash flow; narrative 9999.99 and later notes remain excluded from
financial amounts and original context. New notes and later reconfirmation flag
context without rewriting captured evidence. Alex's selected note remains scoped,
and the client PDF continues to exclude internal session/payment evidence.

Validation logs are under `/workspace/shared`: `counselor-evidence-ui.log`,
`counselor-evidence-backend.log`, `counselor-evidence-focused.log`,
`counselor-evidence-clippy.log`, `counselor-evidence-native-build.log`,
`counselor-evidence-native-complete.log`/`.exit`, and
`counselor-evidence-windows-job.log`. Final artifacts are in
`desktop-smoke-counselor-evidence-complete`. Source/full-history checkpoints are
`/workspace/shared/checkpoints/eisenbart-counselor-report-evidence-source.tar.gz`
and `/workspace/shared/checkpoints/eisenbart-counselor-report-evidence.bundle`.

Complete coverage/final certification, remaining financial models and combined
scenario comparisons, branding/logo/contact details, direct print commands,
reminders/action completion and commercial/security/release acceptance remain open.
This is a tested development checkpoint, not a commercial release.

Next priority: **optional budget method comparisons** — zero-based, 50/30/20 and
custom percentage frameworks using exact arithmetic and non-writing comparisons.
See [remaining-requirements.md](remaining-requirements.md).

## Stage 5F: immutable client action plans — 0.16.0 source

Client action plans now record three to five ordered agreed steps, responsible
people, optional target dates, budget recommendations, follow-up and reviewed
limitations. A counselor selects a current immutable assessment with a captured
scenario, reviews backend-derived allocations/disclosures and acknowledges the
client, approved decisions, assignments, estimates and missing information.
The separate client PDF includes saved firm/client identity, Chicago report date,
historical position and income/spending breakdown, budget agreement, selected
strategy, emergency/goal targets and conditional milestones, priorities and
follow-up. Local fonts/vector bars remain offline. Existing files are never
replaced. Print the saved PDF through a local viewer.

Schema 15 adds scoped append-only plans, a composite immutable assessment link and
atomic creation audit. IPC accepts notes/selection rather than financial outputs.
Exact cent arithmetic derives selected debt/primary savings allocations; unknown
entered budget amounts and allocations exceeding surplus block recording.
Differences from captured budget allocations are disclosed without editing active
plans. Reads/export/recovery validate hashes, scoped parent identity, canonical
text/dates and recomputed original allocations/disclosures. Chicago date rules
are bundled by locked chrono-tz 0.10.4; no network/system timezone is needed.
Draft and busy guards protect navigation and competing assessment edits. Later
supporting changes mark guidance Needs review without changing original values.
See [client-action-plans.md](client-action-plans.md).

Independent example: income 3,250.00 less necessary 500.00/discretionary 200.00
leaves surplus 2,550.00. Selected debt 100.00 plus primary savings 250.00 leave
2,200.00, separately from captured budget debt 150.00/buffer 2,150.00 and approved
historical cash flow 799.99. Unicode client typography, Chicago winter/summer
midnight dates, failed atomic audit, scoped/locked access, immutable history,
forged rehashed financial outputs, schema 14 rollback/upgrade, restart and
encrypted recovery are covered by new backend checks.

| Check | Result |
|---|---|
| Locked setup / production frontend build | Passed |
| Interface suite | 88 passed, including four client-plan tests and parent draft/concurrency integration |
| Linux backend / worker suite | 126 library + 2 worker tests passed (128 total); five new storage/PDF/recovery tests and Chicago-date/IPC validation |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile / full client-plan workflow | Passed standalone debug build and final full native run, exit 0: reviewed agreement, five client PDFs, immutable figures/review flags after edits, all previous workflows, restart and both recovery paths |
| Windows source CI | Passed source 3c63f44: [run 38618328829](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38618328829), 88 interface + 127 library + 2 worker tests (129 backend/process total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

The first two native runs created the agreement and current client PDF but stopped
on assessment export-field focus during inherited fieldset loading. The runner now
reads the browser's effective `:disabled`/inert state and centers the field before
clear/typing; neither typing, clicks nor financial writes are retried. The next run
passed focus but exposed an older broad export completion selector matching the
nested client-PDF notice. Export helpers now wait for the direct success message
with the exact destination filename. All interrupted runs used disposable fictional
workspaces; no uncertain writes were replayed. The fresh final full run passed
with exit 0, including all exports, restart and both encrypted recovery paths.

Windows-validated shipping source is
`3c63f4499df295d6199fe8ffe213bce017ce6869`; later changes concern the native test
runner, verification documentation and a corrected MIT dependency-license entry.

Final native visual/PDF review confirmed firm/client identity, Chicago report date,
three priorities/responsibility, conditional Avalanche interest 35.35, emergency
target 4,899.99/completion 2028-06-30 and historical operating cash flow 1,820.02.
Agreed monthly debt 100.00/savings 250.00/buffer 1,066.67 remain separately labeled
from captured budget buffer 716.67. The four-page Letter PDF embeds local Unicode
Open Sans with no forms, JavaScript or original attachments. Missing inventories
remain unknown, and historical exports display NEEDS REVIEW.

Verification logs are under `/workspace/shared`: `client-action-plan-ui.log`,
`client-action-plan-backend.log`, `client-action-plan-date.log`,
`client-action-plan-clippy.log`, `client-action-plan-native-build.log`,
`client-action-plan-native-final.log`/`.exit`, and
`client-action-plan-windows-job.log`. Final artifacts are in
`desktop-smoke-client-action-plan-final`. Source/full-history checkpoints are
`/workspace/shared/checkpoints/eisenbart-client-action-plans-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-client-action-plans.bundle`.

This is reviewed guidance with an explicitly incomplete financial basis, not a
commercial release or certification that all accounts/statements are present.
Client signatures, completed-action history/reminders, logo/contact branding,
direct application printing, complete-basis final validation, remaining financial
models and release acceptance remain open. No installer is being created.

Next priority: **expanded counselor reports**, incorporating captured session and
recurring-review evidence with original provenance and historical validation.
See [remaining-requirements.md](remaining-requirements.md).

## Stage 5E: captured saved planning scenarios — 0.15.0 source

Added optional named planning snapshots to immutable internal assessments. A
counselor reviews saved budget/debt/savings/dated cash-flow plans, states assumptions
and explicitly chooses a repayment strategy (or none). Rust captures canonical
inputs, exact contexts/results/schedules, plan versions/timestamps and selected
outcomes. Approved historical activity remains separately calculated and unchanged.
Later plan edits flag context without rewriting the original payload; restoring
prior entered values does not restore the captured plan versions.

Payload format 2 adds optional planning with `entered-plans-v1`; format 1 records
remain readable/exportable. Schema stays 14 and old bytes are never rewritten.
The save transaction checks source/profile and all four saved-plan versions and
requires acknowledgment. IPC accepts selection/notes/versions, not financial
outputs. Reads/export/recovery validate scoped identity, hash, canonical inputs,
derived contexts and exact results from the original captured inputs. Shared pure
context functions reuse existing live model rules without changing calculations.
See [captured-planning-scenarios.md](captured-planning-scenarios.md).

The read-only viewer shows budget assumptions/allocations, entered debt terms and
selected payoff outcome, primary savings/goal milestones and independent dated
cash-flow assumptions, with paginated captured schedules and preserved warnings.
PDFs now include the selected hypothetical scenario, assumptions, terms and summary
milestones; full model schedules remain in the encrypted record/viewer. Draft,
load/client and refresh guards preserve failed drafts and prevent data carryover.

| Check | Result |
|---|---|
| Locked cloud setup / production frontend build | Passed |
| Interface suite | 83 passed; three new tests cover reviewed plan versions, selection-only IPC, separate canonical figures/escaped assumptions, stale draft retention/renewed acknowledgment, refresh/review flags and late/mismatched client replies |
| Linux backend / worker suite | 120 library + 2 executable worker tests passed (122 total); four new tests cover independent models vs approved data, historical preservation/review flags, stale/missing/unsupported selection, scope/lock, forged outcomes/contexts/identity/version/extra fields, corrupted recovery rejection, format 1 compatibility, restart and legitimate historical restore |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile | Passed standalone debug build with no bundle |
| Native full scenario/export/recovery workflow | Passed final shipping source c1a7d8b with corrected readiness/viewport harness: explicit Avalanche capture, separate approved figures, budget edit/reversion without rewriting/reactivating the record, four scenario PDFs, all previous workflows, restart and both encrypted recovery paths; runner exit 0 |
| Windows source CI | Passed final shipping source c1a7d8b: [run 38611157947](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38611157947), 83 interface + 121 library + 2 worker tests (123 backend/process total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

Independent model examples: income 3,250.00 / unallocated 2,150.00, Snowball
interest 43.10 vs Avalanche 35.35, emergency target 1,500.00 / completion
2027-01-31. Captured historical cash flow remains 799.99 when a new approved record
becomes 2,000.01. No frontend output can supply those captured amounts.

Initial Windows run 38610161975 passed 82 interface tests but stopped on an existing
balance-review test before backend checks. The ImportsEditor test helper now waits
for initial render/React effects before exposing review controls to the tests;
financial assertions remain intact. The corrected Windows run passed.

The first native run passed capture, source/planning isolation, plan edit/reversion
and PDF checks, then selected a recovery panel before it had loaded. The recovery
helper now explicitly waits for that panel with a read-only observation. A second
run lost a driver connection while scrolling before a CSV approval; screenshot
and driver log confirmed a responsive app/transport reset. A narrow viewport helper
now retries one connection closure using fixed scroll-only code. It cannot type,
click, invoke app commands or replay approvals/saves; general scripts and financial
mutations still have no automatic retry. A fresh isolated full run completed with
exit 0, without weakening assertions or replaying interrupted writes.

Final shipping source `c1a7d8ba6a8ed2c5a9585aa0b97151e252c73730` is uploaded and
Windows-validated; the final test-harness correction and verification notes are
preserved in the checkpoint. Native UI/PDF visual review confirmed the captured
hypothetical label/assumptions, America/Chicago saved-plan times, original monthly
income 3,250.00 / unallocated 716.67, entered terms and explicit Avalanche outcome
35.35 interest, and disclosures about independent plans/missing information. The
fictional nine-page PDF preserves original approved 1,820.02 activity, savings
milestone 2028-06-30 and independent cash-flow closing 3,300.00 after later changes.
No forms, JavaScript, original attachments or workspace credentials are included.

Source/full-history checkpoints are saved outside the checkout at
`/workspace/shared/checkpoints/eisenbart-planning-scenarios-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-planning-scenarios.bundle`. Validation logs
are under `/workspace/shared`: `planning-snapshot-ui-tests.log`,
`planning-snapshot-backend-tests.log`, `planning-snapshot-focused-tests.log`,
`planning-snapshot-clippy.log`, `planning-snapshot-native-complete.log`,
`planning-snapshot-native-complete.exit` and `planning-snapshot-windows-validation.log`.
Final native artifacts are in `desktop-smoke-planning-snapshot-complete`.

This captures existing saved entered plans; general combined non-writing scenario
variations/side-by-side comparison, a final client action plan, agreed priorities,
remaining advanced debt models, recurring/session evidence and complete original
report/release/commercial requirements remain open. A supported projection can
still show shortfall/growth/horizon warnings; capture never certifies completeness
or feasibility. No new dependency or installer was added.

Next priority: **client action plans** with three to five prioritized actions,
agreed allocations, selected debt/savings milestones, follow-up and final review.
See [remaining-requirements.md](remaining-requirements.md).

## Stage 5D: offline internal assessment PDF export — 0.14.0 source

Implemented local printable PDF export of saved immutable counselor assessments.
Rust rechecks scoped authentication, stored integrity/evidence and original
calculation before rendering; current source/client changes add historical
review notices without replacing captured figures. PDFs include identity, period,
exact approved amounts, local charts, classified/dated facts, counselor notes,
sources/discrepancies and transaction/inventory evidence. Every page discloses
internal/incomplete status. Embedded Open Sans and Unicode maps keep typography
local and text extractable. No schema change (14), new renderer service, source
write or planning inference. See [assessment-pdf-export.md](assessment-pdf-export.md).

Native destination selection and a path input support local saving. Draft and
busy guards prevent conflicting operations and late notices across clients.
Existing files are never replaced; a synchronized private temporary file is
published without clobbering. Unsupported glyphs/shaping characters or export
limits fail without truncation or changing the saved record.

| Check | Result |
|---|---|
| Locked cloud setup / frontend production build | Passed |
| Interface suite | 80 passed; two new tests cover export identity/arguments, cancellation, failed-path retention, concurrent operation/draft blocking and late client replies |
| Linux backend / worker suite | Full suite: 116 library + 2 executable worker tests passed (118 total); all four focused PDF tests also passed after the historical-warning presentation correction; four new tests cover historical 799.99/net worth 4,000.06 vs live 2,000.01, Unicode/font embedding, measured wrapping/page limits, unsupported glyph/no partial file, scoped/locked access and unsafe destinations; corrupt-record export rejection extends existing coverage |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile | Passed debug build with no bundle |
| Native full desktop/recovery/export workflow | Passed final implementation 6f6bf1e: four actual printable PDF exports preserve original 199.98/net worth 525.05 versus current 500.00, captured evidence/review flags and fixed-record wording; byte-for-byte preservation after refused overwrites; restart and both encrypted recovery paths |
| Windows source CI | Passed final implementation 6f6bf1e: [run 38605164502](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38605164502), 80 interface + 117 library + 2 worker tests (119 backend/process total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

Direct in-app printing, complete scenario-backed counselor reports, final client
action plans, organization logos/contact branding, complex-script font fallback,
PDF tags/archival certification and export-specific audit/permissions remain open.
PDFs are readable local files outside workspace encryption. Existing counselor
assessment creation remains audited; exporting does not create another audit event.

Initial Windows run 38604263280 passed 79 interface tests but failed the recurring
review test before backend checks. The test now explicitly settles initial React
loading effects and observes enabled review controls before clicking; its original
verification/withdrawal assertions remain intact. Final Windows checks passed.
Native visual inspection found a captured live-dashboard warning in the initial
PDF; saved views now render historical context wording without changing the
archived warning/financial payload or live dashboard.

The first native implementation run passed. During final validation, a later
focused Cargo test replaced the standalone executable with a development-server
binary; its refused-localhost screenshot identified the cause before data entry.
The standalone build was rebuilt last. A subsequent driver connection dropped
while typing the first report title; screenshot/driver log confirmed a responsive
app and a transport reset. No typing/save was replayed. A fresh isolated run from
the same final committed implementation passed the complete workflow. The build
order and transport limits are recorded in [smoke-test.md](smoke-test.md).

Final implementation `6f6bf1ebb4d7ffd65b7bfcec2cfeb6223f67a220` is uploaded and
Windows-validated. Native PDF visual review confirmed firm/client/counselor,
fixed original period/figures, review notices, readable charts/typography/margins,
page numbers and honest incomplete status. Final fictional PDF: five Letter pages,
embedded font/extractable text, no forms, JavaScript, original attachments or
workspace credentials. Source/full-history checkpoints are saved outside the
checkout at `/workspace/shared/checkpoints/eisenbart-pdf-export-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-pdf-export.bundle`. Validation logs remain
under `/workspace/shared`: `pdf-export-ui-tests.log`, `pdf-export-backend-tests.log`,
`pdf-export-focused-tests.log`, `pdf-export-clippy.log`,
`pdf-export-native-complete.log`, `pdf-export-windows-validation.log`; final native
artifacts are in `desktop-smoke-pdf-export-complete`.

Next priority: **captured planning scenarios and client action plans** with agreed
allocations, debt/savings milestones, prioritized actions and final report review.
The original remaining scope is recorded in [remaining-requirements.md](remaining-requirements.md).
No new installer was created.

## Stage 5C: immutable counselor assessment records — 0.13.0 source

Implemented encrypted, append-only internal assessment records with exact approved
figures, saved profile/firm/counselor identity, captured transaction decisions/raw
evidence, document references/checks and verified balance facts. Counselors can
add findings, recommendations and follow-up. Reused offline charts/text tables
show the captured assessment; paged evidence retains exclusions and original
source locators. Corrections/reversals/profile changes flag prior context without
rewriting historical figures. New records recalculate current approved data.

Schema 14 adds scoped records and atomic audit. Save checks source revision and
profile timestamp, requires acknowledgment and takes only notes/period/context
from IPC. Reads/recovery validate digest, headers, bindings and captured arithmetic
at the original date. Draft/source context and late-reply guards prevent unsafe
updates or cross-client carryover. Saving a report does not alter financial
source decisions/revisions or plans. See [saved-assessments.md](saved-assessments.md).

| Check | Result |
|---|---|
| Locked setup / production frontend | Passed |
| Interface suite | 78 passed, including five saved-assessment tests for scoped lazy load/save, canonical display, failed-draft retention, explicit changed-context review, late-client replies and conflicting-operation guards |
| Linux backend / worker suite | 112 library + 2 executable worker tests passed (114 total); four new tests cover immutable old figures versus new approved results, source/profile changes, scope, stale context, acknowledgment, atomic audit rollback, immutable SQL, malformed figures/dates, schema-13 upgrade/rollback, restart/recovery and corrupted snapshot rejection |
| Formatting / Clippy, warnings denied | Passed |
| Compiled native desktop workflow | Passed full finance/CSV/PDF/recurring/balance workflows, immutable original/new counselor assessments, source-change review flags and preserved original amounts/recommendations, full restart and both encrypted recovery paths |
| Windows source CI | Passed implementation d2b0ab5: [run 38600293809](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38600293809), 78 interface + 113 library + 2 worker tests (115 backend/process total); all installer/package/upload steps skipped |
| New installer / clean Windows offline acceptance | Not run |

Financial examples preserve 799.99 / net worth 4,000.06 before a source withdrawal;
a new record shows 2,000.01 while the original still retains 799.99. Existing
native examples preserve original 199.98 while live/current records change to
500.00. No financial amounts are accepted from the report form.

The first native attempt lost its WebDriver transport while typing an existing
cash-flow input before reaching assessments. A screenshot confirmed the app still
responded. A fresh isolated run completed the full workflow from the same
implementation, with no replayed financial write or weakened assertion. Native
visual review confirmed the saved client/firm/counselor identity, America/Chicago
record time, captured period/approved/incomplete basis, Needs review status and
original independently expected figures after later source withdrawal.

Final implementation `d2b0ab526737bc8ddf7d5a25b4c36c4690717a49` is uploaded and
Windows-validated. Source/full-history checkpoints are saved outside the checkout:
`/workspace/shared/checkpoints/eisenbart-assessments-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-assessments.bundle`. Logs/native artifacts
remain under `/workspace/shared`: `assessment-backend-tests.log`,
`assessment-ui-tests.log`, `assessment-clippy.log`, `assessment-native-complete.log`,
`assessment-windows-validation.log` and `desktop-smoke-assessments-complete`.

Next priority: **offline PDF report export and scenario-backed client action plans**.
The complete original Counselor Report, selected scenarios, saved plan assumptions
and final client plans remain open. See [remaining-requirements.md](remaining-requirements.md)
for the remaining original scope; clean Windows offline acceptance and commercial
readiness are still pending.

## Stage 5B: verified classifications and dated balance facts — 0.12.0 source

Implemented explicit necessary/discretionary and take-home/other income
classifications bound to exact scoped approval decisions. Corrections/reversals
and reapproval leave old classifications inactive until explicit re-verification.
Schema 13 adds append-only dated balance/classification snapshots with evidence,
reason, counselor, authenticated scope, revision checks and atomic audit.

The dashboard shows recorded expense/payroll classifications, dated cash/savings/
debt, sufficient-inventory net worth, a local debt chart and clearly labeled
coverage/payment ratio estimates. Unknowns, stale source decisions, incomplete
inventories, mixed dates and outdated balance facts prevent unsafe conclusions.
Manual planning inputs remain separate; missing transaction coverage remains
explicit even after a complete balance inventory is declared. Gross-income lender
DTI, partial per-account reserve allocations and savings-goal progress remain open.
See [verified-financial-facts.md](verified-financial-facts.md) for formulas/limits.

| Check | Result |
|---|---|
| Locked setup / production frontend | Passed |
| Interface suite | 73 passed, including four facts-editor, two metric-display and one full parent correction/re-verification regression |
| Linux backend / worker suite | 108 library + 2 executable worker tests passed (110 total); new tests cover exact cents/ratios, unknowns, Unicode/bounds, source corrections/reapproval, scoped bindings, audit/revision rollback, append-only history, restart/recovery and schema-12 upgrade/rollback |
| Independent Python Decimal examples | Passed: net worth 4,000.06 / 525.05, coverage 1.99 / 2.00 months, payment ratio 5.00% |
| Formatting / Clippy, warnings denied | Passed |
| Compiled native desktop workflow | Passed full existing finance/import workflows, explicit classifications and dated inventory, exact net worth/ratios, correction invalidation and explicit re-verification, withdrawal exclusion, local debt chart, full restart and both encrypted recovery paths with preserved bindings/history |
| Windows source CI | Passed implementation 96cd863: [run 38593850120](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38593850120), 73 interface + 109 library + 2 worker tests (111 backend/process total); all installer/package/upload steps skipped |
| New installer / clean installed Windows offline acceptance | Not run |

Initial checks caught a JSX delimiter and test helper calls using command names
instead of backend method names; both were corrected. Storage tests caught a
foreign-key reference to a nonexistent client key. The migration now uses the
repository's existing client/organization and counselor/organization keys, with
authoritative counselor ownership enforced in Rust. Focused tests then passed,
followed by the full local suite. A timing-sensitive recurring-detail assertion
failed in a combined interface run while its isolated case passed. It now awaits
the rendered detail before checking the same warning/disabled confirmation; the
complete interface suite subsequently passed. No safeguard was relaxed.

Native testing found that selecting the same prior class in a WebKit dropdown
left the old binding inactive after save. A diagnostic run confirmed that the
saved control remained unclassified, while independent backend and parent
interface tests correctly bound new source decisions. The interface now offers
an explicit Reverify prior classification button that updates the draft from the
current approval. The parent regression checks the correction, explicit action,
exact new approval ID and successful saved state. The complete native run passed
from final implementation 96cd863 after this fix, without replaying any financial
write. The two earlier native attempts preserved inactive classifications when
re-verification had not taken effect. Neither was counted as a passing workflow.

Final implementation `96cd863199d0f4d83b9ee476af10dbc6681942e0` is uploaded and
Windows-validated. Source/full-history checkpoints are saved outside the checkout:
`/workspace/shared/checkpoints/eisenbart-verified-facts-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-verified-facts.bundle`. Validation logs and
native artifacts remain under `/workspace/shared`, including
`facts-backend-tests.log`, `facts-ui-tests.log`, `facts-native-complete.log`,
`facts-windows-validation.log` and `desktop-smoke-facts-complete`.
This source checkpoint does not establish clean Windows installation/offline
acceptance, commercial readiness or independent security review.

Next priority: **immutable saved assessment snapshots and counselor reporting**
using the approved source decisions and verified facts, followed by offline PDF
reports/action plans.

## Stage 5A: approved transaction dashboard — 0.11.0 source

Implemented a client-specific read-only assessment for explicit 1–24 calendar-month
periods. Rust calculates approved-only income receipts, gross spending, refunds,
net spending, operating cash flow and all account movement in exact cents. Locally
bundled charts show approved categories, monthly income/spending and cash-flow
trends. Every view labels period, revision, approved basis, incomplete coverage,
undated exclusions, review decisions and outdated periods. Source corrections and
reversals recalculate results; stale/client-mismatched replies are hidden.

No new schema, financial writes or dependencies. Planning remains separate.
Verified take-home pay, necessary/discretionary classifications, balance facts,
asset/debt metrics, debt/savings charts and immutable final reports remain pending.
See [financial-dashboard.md](financial-dashboard.md) for formulas and limits.

| Check | Result |
|---|---|
| Locked setup / production frontend | Passed |
| Interface suite | 66 passed, including four dashboard interaction tests |
| Linux backend / worker suite | 103 library + 2 executable worker tests passed (105 total); focused dashboard/scoped-storage tests also passed after adding the aggregate-limit example |
| Independent Python Decimal examples | Passed: 918.89 operating flow versus 718.89 account movement across two months |
| Formatting / Clippy, warnings denied | Passed |
| Compiled native application | Passed full financial/import workflows, explicit dashboard periods, approved CSV/PDF totals, pending exclusions, local chart tables, automatic correction/reversal updates, full-process restart and both encrypted recovery paths |
| Windows source CI | Passed implementation b6d5eae: [run 38587770252](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38587770252), 66 interface + 104 library + 2 worker tests (106 backend/process tests); all installer/package/upload steps skipped |
| New installer / clean Windows offline acceptance | Not run |

The first synthetic engine fixture incorrectly used an empty category on an
approved row; approval requires a verified category. The fixture now uses Food,
and the corrected focused tests pass without relaxing approval validation.
The outdated-period example now selects June, which is more than 90 days before
the fixed October reference date. Per-transaction input limits do not cap derived
assessment totals; an independently expected 2,000,000,000.00 aggregate verifies
exact calculation and integer chart scales.

The first native launch lacked the cloud toolchain path; sourcing the existing
cloud environment fixed the launch without application changes. The first full
UI attempt lost its WebDriver connection while selecting an existing CSV mapping
option. A captured screenshot confirmed the application was responsive; no
financial operation was retried. A fresh isolated run completed the entire
workflow from the same committed implementation. Native visual inspection
confirmed the displayed assessment period, approved basis and independently
expected income/spending/refund/cash-flow totals.

Implementation `b6d5eae0b1e120d291a083e6ac4546749293b259` is uploaded and Windows-validated.
Final source and full-history Git checkpoints are saved outside the repository as
`/workspace/shared/checkpoints/eisenbart-dashboard-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-dashboard.bundle`. No installer was created.
Clean installed Windows 11 offline acceptance and commercial release remain open.

Next priority: **approved expense classifications and dated verified balance facts**
for the remaining dashboard metrics, before counselor reports and action plans.

## Stage 4D: statement running-balance reconciliation — 0.10.0 source

Implemented exact integer-cent comparisons for explicitly mapped after-transaction
checking/savings CSV balances and declared oldest/newest-first source order.
Original local steps use reported anchors and original amounts regardless of
review status, only to check source arithmetic. Independent approved cumulative
steps use a known entered opening and approved movements; rejected rows contribute
nothing and never become anchors. An undecided row makes later approved
expectations unknown. Missing dates/amounts/balances remain unknown; contradictory
dates block checks without sorting. Rejected dates are excluded from the approved
sequence. Same-day order is preserved and disclosed.
Final reported-versus-entered-closing checks complement existing whole-source
and approved-total checks. Intermediate differences cannot hide behind a matching
whole-statement total. Matched arithmetic never approves or verifies an asset.

Counselors can correct or clear order declarations with a reason. Schema 12
stores authenticated, client/document-scoped append-only settings and generic
audit events with optimistic revisions. Original source metadata, financial
choices and plans remain unchanged; derived exceptions update immediately after
corrections/reversals and block unsafe bulk approvals. Changing a setting does
not reset Needs Clarification to Pending or automatically approve any row.
Settings/history and comparisons survive encrypted recovery; older JSON defaults
to unknown order. Migrations/write failures preserve existing records. Existing
limits apply, plus 1,000 balance-setting decisions/client. Scanned/text-PDF balance
columns, before-transaction/available/card balances and automatic statement
completeness claims remain outside this checkpoint. See
[statement-balance-reconciliation.md](statement-balance-reconciliation.md).

| Check | Result |
|---|---|
| Locked setup and production frontend | Passed |
| Interface tests | 62 passed, including explicit mapping/order, source-versus-pending display, failed draft/dirty retention, conflicting-operation guards and clearing disabled mapping |
| Linux backend/process suite | 98 library + 2 executable worker tests passed (100 total); seven new engine/storage tests cover exact cents, both source orders, date conflicts, missing/zero/negative figures, pending/rejected exclusion, corrected fields, intermediate discrepancies, scope, stale/audit rollback, Unicode reason limits, immutability, restart/recovery and schema-11 migration rollback |
| Independent Python Decimal examples | Original closing 1,199.98; corrected closing 1,199.99; rejecting rent gives approved movement 500.00, expected 1,500.00 and 300.02 difference |
| Format / Clippy all targets, warnings denied | Passed |
| Compiled native desktop workflow | Passed full existing client/financial/CSV/PDF/recurring workflows, explicit source order and conflict correction, validated bulk approval, one-cent correction, rejection exclusion, immutable settings, client isolation, full-process restart, normal backup restore and missing-metadata recovery |
| Windows source CI | Passed final implementation c052453: run [38525998374](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38525998374), production frontend, 62 interface tests, 99 library + 2 worker tests (101 backend/process tests total). All installer/package/upload steps skipped |
| New installer | Intentionally not built |
| Clean installed Windows 11 offline acceptance | Not run |

Two initial synthetic storage fixtures were corrected: a resolved source-order
setting intentionally leaves imported uncertain records Needs Clarification, and
a second account needs a unique label or existing-account selection. The final
focused storage rerun passed all three tests without weakening either safeguard.
Native UI testing found a selected statement panel closing on a saved revision;
the panel now retains its source selection and displays fresh results. An
interface regression checks results remain visible after a successful setting save.
A second native check found source-record navigation also cleared the source
selection; navigation now retains the statement alongside the transaction editor,
with an interface regression for that context. The selector placeholder closes
the view explicitly.
The final complete native run passed from the final implementation after both
context-preservation fixes. Native visual inspection confirmed the separate
source/approved comparisons and source-record navigation. No financial write was
replayed or required assertion disabled.

Final implementation `c0524535e9b21618dd65f91ed8b775a6f2f94bb7` is uploaded to
main and passed Windows run [38525998374](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38525998374).
Logs: `/workspace/shared/reconciliation-backend-verified.log`,
`reconciliation-frontend-tests.log`, `reconciliation-clippy.log`,
`reconciliation-native-complete.log`, and `reconciliation-windows-validation.log`.
Final native artifacts: `/workspace/shared/desktop-smoke-reconciliation-complete`.
Verified source/Git recovery checkpoints are
`/workspace/shared/checkpoints/eisenbart-reconciliation-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-reconciliation.bundle`; these preserve
development source/history, not client records or a Windows installer.

Next priority: **client financial dashboard** built from approved imported facts,
with explicit assessment dates, missing/outdated-data indicators and locally
bundled charts. Broader statement adapters, scanned OCR, saved cross-module
scenarios, advanced financial models, reports/action-plan PDFs, multiple counselor
roles, firm branding, licensing/signing and full release acceptance remain open.
No new downloadable build was created, as requested.

## Stage 4C: merchant grouping and recurring-payment review — 0.9.0 source

Implemented approved-only historical payment grouping within one statement account
and type. Exact normalized verified merchant names control membership; absent
merchants use the exact description with an explicit fallback warning. Counselors
combine or separate payments by correcting verified Merchant fields in source
review, preserving original provenance and correction history. Expense/fee groups
show exact integer-cent observed totals, amount ranges, dates and source locators;
other types and unapproved records never enter this checkpoint's analysis.

Conservative suggestions require at least three distinct approved payment dates,
no unresolved duplicate conflicts, and every adjacent interval matching weekly,
fortnightly, calendar-monthly, quarterly or annual rules. Month-end/leap-day dates
and variable amounts have independently checked examples. There is no automatic
subscription classification, confirmation, fixed future charge or planning write.
Counselors explicitly choose/correct frequency and label, optionally classify a
verified subscription, or dismiss/withdraw a pattern with a reason.

Schema 11 captures append-only decision evidence, exact approval-history IDs,
reviewer/time/reason, revision/hash checks and atomic generic audit events in the
encrypted workspace. Changed evidence appends a permanent invalidation marker
atomically with the source change. Withdrawing a newly added charge cannot revive
an old decision when the remaining facts match again. Changed/reversed evidence
requires new review; identical
reapproval does not silently reactivate prior confirmation. Absent groups retain
historical decisions. Authenticated client scope, failed-save retention, unsaved
navigation/lock guards, restart, backup and restore validation remain active.
Limits: 500 captured payments/decision, 256 KiB/decision JSON, 1,000 decisions
and 8 MiB captured JSON/client; bounds fail without deleting history. Existing
source limits apply. See [recurring-payment-review.md](recurring-payment-review.md).

| Check | Result |
|---|---|
| Locked setup, production frontend and native debug build | Passed |
| Interface tests | 59 passed; explicit decisions/frequency correction, unchanged exact totals, failed draft/dirty retention, conflicting-operation guards and changed-evidence dismissal |
| Linux Rust suite | 91 library + 2 executable worker tests passed (93 total); seven new tests cover independent patterns, exclusions, duplicates, reversal/reapproval, scope, atomic audit failure, stale writes/hashes, lock/restart, backup restore, permanent invalidation/write rollback, and schema-9/10 upgrades and rollback |
| Independent Python Decimal example | 32.05 observed total; 22.04 after first payment is withdrawn |
| Format / Clippy all targets, warnings denied | Passed |
| Compiled native desktop workflow | Passed final schema-11 full financial/CSV/PDF/recurring workflow, explicit subscription/reconfirmation, permanent invalidation after an added-then-withdrawn charge, exact totals, client isolation, whole-process restart, normal backup restore and missing-metadata recovery, including captured history and invalidation flags |
| Windows source CI | Passed final source 92891dd: run [38517928152](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38517928152), production frontend, 59 interface tests, 92 library + 2 worker tests (94 backend/process tests total). All installer/package/upload steps skipped |
| New Windows installer | Intentionally not built |
| Clean installed Windows 11 offline acceptance | Not run |

Initial backend regression found an older PDF migration fixture retaining the
newer recurring-history table while simulating schema 8; its downgrade fixture
was corrected and the complete suite rerun. A backup test fixture was corrected
to use the existing lock-before-restore rule. An initial native run stopped after
a WebDriver transport closure during the prior cash-flow workflow; no financial
write was replayed or assertion weakened, and the complete run restarted in a
fresh disposable workspace. Native visual review prompted existing compact table
styles and a bordered decision form; the styled build passed the full native
workflow again. Final review found the added-then-withdrawn-charge edge case;
permanent invalidation markers and regressions were added. Schema 11 preserves
the initial schema-10 source checkpoint and flags those earlier decisions for
explicit reconfirmation; upgrade failures preserve the original records.
Intermittent native WebDriver closures also interrupted final checks; financial
writes were never retried in place. The final exact-source native workflow passed after the full backend suite
finished, and passed again after the text-validation patch; no assertion or
financial-write retry policy was weakened. Windows source validation passed for
schema 11 and the final character-length patch.
Source/debug logs and fictional artifacts are under
`/workspace/shared/recurring-*`; final exact-source native artifacts are
`/workspace/shared/desktop-smoke-recurring-complete`. Full backend logs are
`recurring-schema11-backend.log`, interface logs `recurring-frontend-release-final.log`,
and native logs `recurring-native-complete.log`. The final text validation uses
character counts consistently with existing input limits; seven focused tests
passed again, including 120-character accented labels and 500-character accented
reasons surviving lock/restart and encrypted recovery, with over-limit rejection.

Final implementation commit `92891dd0924e420cc528c084c002e1d7d23ee5a6` is uploaded
to main. Windows source CI passed run [38517928152](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38517928152);
full log: `/workspace/shared/recurring-windows-final-validation.log`. Final native
workflow and visual review passed from the same implementation source. Source-only
recovery checkpoints are saved as `/workspace/shared/checkpoints/eisenbart-recurring-source.tar.gz`
and `/workspace/shared/checkpoints/eisenbart-recurring.bundle`; these preserve
tracked development source/Git history, not encrypted client records or an installer.

Next priority: **row-level statement reconciliation** against known reported
running balances, with explicit source/order/sign checks and unresolved exceptions.
Scanned OCR, institution adapters, dashboards/reports, advanced models,
accounts/roles, licensing/signing and full installed/commercial acceptance remain
unfinished. No new download was created, as requested.

## Stage 4B: selectable-text PDF import and review — 0.8.0 source

Implemented bounded local PDF inspection in a supervised child of the desktop
executable before workspace/Tauri initialization. Only source bytes pass through
stdin; no workspace key, password, path, source filename, authentication, or proxy
variables are supplied. No PDF renderer, JavaScript evaluator, attachment
extraction, OCR, or network service is invoked. CPU/data/core limits on Linux,
Windows memory Job Objects, 15-second wall deadlines, and bounded input/output
keep parser failures apart from the active workspace. These are process/resource
boundaries, not a complete operating-system sandbox or security certification.

Counselors explicitly choose page/text lines, actual date style, account, known
period/balances, sign convention, and optional original retention. Supported
starting layout is full date first, description middle, one signed amount last.
Multiple numeric columns or invalid/missing amounts remain unknown; wrapped text
and glyph/ordering uncertainty require human correction. All PDF rows begin as
Needs Clarification, never bulk eligible, and require classification/category,
source acknowledgment and a reason before individual approval. No account number,
transaction type, missing date/amount, merchant, or statement metadata is guessed.

Immutable reported text retains document hash/ID, account, selected page/line,
original date/amount, extraction method and uncertainty. Corrections/reversals
preserve the existing immutable before/after decision history. Approved-only
exact-cent totals, duplicate advisories across CSV/PDF sources, paired transfers,
client scope, optimistic revisions, atomic audit/rollback, lock/cancel/expiry,
and planning separation reuse the CSV foundation. Authorized original PDFs are
SQLCipher BLOBs included in existing authenticated backups. Restore validates
PDF locators and imported decisions/transfer links. Schema 9 preserves previous
records/CSV JSON and adds PDF audit events; older applications cannot reopen it.

See [pdf-import-review.md](pdf-import-review.md) for actual user steps, format,
parser boundaries, completeness limits, and independent examples. Limits: 2 MiB,
100 pages, 1 MiB extracted text, 2,000 nonempty/4,000 total lines, 2,048 bytes/line,
and 500 selected lines/source; existing client/backup limits remain unchanged.
Scanned/image-only or protected PDFs, including owner-protected files with an
empty user password, are rejected. Mixed image/text PDFs require checking every
original page for missing transactions. Selected-line totals cannot prove completeness.

| Check | Result |
|---|---|
| Locked setup/dependency fetch, production frontend, native debug build | Passed |
| Interface tests | 56 passed, including explicit PDF selection/unknown metadata, inert markup, no automatic/bulk approval, failed-draft retention and cancel/no-write |
| Linux Rust full suite | 84 library tests + 2 executable worker integration tests passed (86 total); final focused PDF rerun 7 + worker rerun 2 passed after core-dump/original-encryption safeguards |
| Independent Python Decimal example | Original PDF net 974.90; only approved payroll adds income to 3,000.01 and combined CSV/PDF activity to 1,820.02; pending expense contributes zero |
| Format and Clippy all targets, warnings denied | Passed |
| Compiled native desktop workflow | Passed existing client/planning/CSV workflows, separate PDF parser, damaged rejection, explicit line selection, source locators, individual approval, pending exclusion, authorized retention, whole-process restart/backup interruption, PDF decisions/history restored through normal and missing-metadata recovery |
| Native visual inspection | Passed for reported-text provenance, warnings, and corrected verified fields; final exact-source desktop/recovery rerun passed |
| Windows source CI | Passed: run [38513471590](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38513471590) from 5b6bcbf; production frontend, 56 interface tests, 85 library tests + 2 desktop-worker tests (87 backend/process tests total), including Windows Job Object handling and SQLCipher memory-lock regression |
| New Windows installer | Intentionally not built |
| Clean installed Windows 11 offline PDF workflow/native OS pickers | Not run |

Initial full native artifacts: `/workspace/shared/desktop-smoke-pdf`; exact-source
final artifacts are `/workspace/shared/desktop-smoke-pdf-final`. Logs are under
`/workspace/shared`: `pdf-backend-full.log`, `pdf-focused-final.log`,
`pdf-process-final.log`, `pdf-frontend-tests.log`, `pdf-frontend-build.log`,
`pdf-clippy.log`, `pdf-native-smoke.log`, and `pdf-native-final.log`.
Existing negative SQLCipher fixtures produce expected authentication diagnostics;
no verification or assertion was disabled. During implementation, a form fragment
boundary and a Unix unit-valued binding were corrected before checks passed.
Final review rejects original encryption even after a library decrypts an empty
user password, and disables Linux parser core dumps. Synthetic worker tests
exercise the actual desktop executable/protocol, including deadline termination.
The worker process is not claimed as a complete OS filesystem/network sandbox.

Source commit `5b6bcbf9343c829f074a87ff7ff5f9da1d0f84d3` is uploaded to
main. Windows run [38513471590](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38513471590)
passed; its full log is `/workspace/shared/pdf-windows-validation.log`. The final
compiled exact-source native workflow passed at `/workspace/shared/desktop-smoke-pdf-final`.
Installer creation, installed launch/uninstall, download metadata, installer upload,
and installer-log upload were explicitly skipped. No manual package run occurred.
Windows checks establish source/debug worker compilation/protocol/resource handling;
clean Windows 11 GUI/offline acceptance and release-package verification remain unrun.

Verified tracked-source and full-history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-pdf-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-pdf.bundle`, refreshed after these final
notes. They are development source checkpoints; application-data backups use
`.ebcbackup`. The earlier 0.3.0 installer contains neither this PDF stage nor the
later source-only features. No new download was created.

Next priority: **merchant grouping and recurring-payment review** on imported
records, with advisory suggestions and explicit counselor decisions. Institution
adapters, scanned OCR, automatic table reconstruction, advanced models, dashboard
charts, finalized reports/PDF action plans, accounts/roles, licensing/signing,
clean Windows acceptance, and commercial release review remain unfinished.

## Stage 4A: local CSV import and transaction review — 0.7.0 source

Following the original staged specification, implemented explicit, bounded local
CSV inspection/mapping, client checking/savings account association, entered
statement periods/balances, immutable source provenance, optional authorized
complete-source retention inside SQLCipher, duplicate advisories, a client review
queue, and the exact Pending / Needs Clarification / Approved / Rejected states.
Importing never approves a record. Unknown dates/amounts remain unknown; no
income, account number, merchant classification, or statement balance is guessed.

Rust owns exact-cent approved-only totals and all decisions. Refunds offset
spending; verified equal/opposite transfer legs update separate accounts without
extra income/expense. Bulk approval accepts only 1–100 unchanged validated
Pending rows; conflicts/corrections/uncertain data require individual review.
Approval reversals immediately withdraw their effects and retain immutable
before/after fields, links, reviewer, timestamp, and reason. Reversing a linked
transfer clears both approvals atomically. Imported facts never rewrite planning
inputs. Table filters leave the clearly labeled all-imported-dates totals intact.

Schema 8 preserves all earlier plans and audit history; scoped composite keys,
optimistic revisions, immediate transactions, and atomic audit writes protect
isolation and stale/failing operations. Same-account identical source hashes
cannot import again. Original/corrected row overlaps and reference matches are
advisory; repeated legitimate movements need explicit review. Duplicate diagnostics
are capped at 20 related IDs per row to bound large overlap groups. Captured
source bytes expire after 15 minutes or lock; canceling inspection writes nothing.
Backups include authorized original CSV BLOBs, provenance, choices, and history;
restore also validates imported totals and reciprocal approved transfer links.

See [csv-import-review.md](csv-import-review.md) for user steps, supported formats,
source/accepted statement checks, independently known examples, storage, and limits.
No new Windows installer was built, as requested. The old 0.3.0 download lacks
this stage. Clean Windows 11 installed/offline CSV and recovery acceptance and
independent security review remain unrun. Continue using fictional records.

| Check | Result |
|---|---|
| Locked dependency setup, production frontend, compiled debug native application | Passed |
| Frontend tests | 54 passed, including explicit CSV mapping/retention, pending staging, escaped source text, cancel/no-write, native totals, decision payloads, exact failed-draft retention, transfer pairing, failed-load protection, late responses, and private-path lock/navigation guards |
| Linux Rust tests | Full suite 77 passed; focused import suite 13 passed, including a 2,000-row overlap with bounded diagnostics; schema 7 migration/rollback and existing schema 1–6 fixtures preserve prior records |
| Independent Python Decimal example | Confirmed net spending 1,179.99; checking activity 620.01; savings 200; combined 820.01; source checking closes 1,610 before fee rejection |
| Rust format and Clippy all targets, warnings denied | Passed |
| Full compiled Linux native desktop workflow | Passed: existing plans; explicit CSV mapping/retention; no automatic approval; bulk approval, individual refund, fee reversal/history, transfer pairing, repeated-source rejection; two-client isolation; lock/wrong password; full restart; backup interruption, restored CSV decisions/history/links and missing-metadata recovery |
| Native visual inspection | Passed: review/history readable, checkboxes correctly sized, wide tables scroll inside their own wrapper; final compiled layout/recovery rerun passed |
| Windows source CI | Passed: run [38509546704](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38509546704) from 8054f12; production frontend, 54 interface tests and 78 Rust tests, including the Windows-only SQLCipher memory-lock regression |
| New Windows installer | Intentionally not built |
| Clean installed Windows 11 offline workflows/native pickers | Not run |

Native full-workflow artifacts: `/workspace/shared/desktop-smoke-imports-final-verified`; final exact-source layout/recovery artifacts: `/workspace/shared/desktop-smoke-imports-layout`.
Final logs: `import-backend-complete.log`, `import-bounded-tests.log`,
`import-frontend-final.log`, `import-frontend-build-final.log`,
`import-clippy-final.log`, `import-native-final-verified.log`, and `import-native-layout.log` under
`/workspace/shared`. Negative SQLCipher fixtures reject altered/wrong-key data;
expected authentication diagnostics do not indicate failed checks.

During implementation, a CRLF/multiline source-line locator, fraction grouping,
rejected-row duplicate reapproval, and excessive duplicate diagnostics were
corrected before passing tests. Clippy findings were fixed through clearer parser
control flow and grouped review updates. Native inspection corrected checkbox
sizing. The harness now centers visible controls before clicking to avoid
WebKit's partial-viewport click issue, preserves Unicode in XPath option labels,
and retries one explicit read-only DOM/runtime observation on connection closure.
Early driver interruptions were discarded and rerun in fresh temporary workspaces;
financial writes, clicks, typing, and general scripts are never replayed. No
security verification or assertion was disabled. The final CSV table scrolls
inside its wrapper; the final exact-source layout/recovery rerun passed.

Source commit `8054f12e9a8bb60dcd9f3b2ba8c945e43ee94037` is uploaded to
main. Windows run [38509546704](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38509546704)
passed; the full log is retained at `/workspace/shared/import-windows-validation.log`.
Installer creation, installed launch/uninstall, download metadata, installer upload,
and installer-log upload were explicitly skipped. No manual package run occurred.
These Windows results verify source compilation/tests, not installed GUI/native
pickers or clean Windows 11 offline acceptance.

Recoverable tracked-source and full-history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-imports-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-imports.bundle`, refreshed and verified
after these final validation notes. They are development source checkpoints;
client workspace recovery uses the separate encrypted `.ebcbackup` format.

Next priority: selected **text-based PDF statement extraction**, retaining source
locators and uncertainty and feeding this same review queue. Scanned OCR,
institution/card parsers, inferred recurring groups, row-by-row running-balance
checks, finalized reports, advanced scenarios, licensing/signing, and commercial
release acceptance remain unfinished.

## Recovery hardening: encrypted backup and restore — 0.6.0 source

Following the original staged specification, implemented manual encrypted local
backups and verified whole-workspace restore. The user selects a local/external
folder; unique, exclusively created `.ebcbackup` files never replace existing
backups. SQLCipher transaction snapshots include committed WAL records. The
container authenticates its header and ciphertext with domain-separated
HMAC-SHA256 derived from the workspace key; no password or raw key is persisted.

Restore is available on locked, first-setup, and storage-error screens. It verifies
the password/container, SQLCipher and SQLite integrity, relationships, schema,
and saved plan inputs, then displays the organization/counselor/client count.
An explicit RESTORE confirmation activates a new private generation through
one atomically replaced pointer. Previous files remain intact; records are not
merged. Candidate cancellation/expiry, conflicting operations, source changes,
invalid files, and migration/audit/activation failures preserve the active source.
Existing sessions lock if another instance changes the active generation.

Schema 7 preserves all earlier records and immutable audit history, adding
backup/restore events. Older backups migrate only in the recovery copy. Limits:
256 MiB database, 4 KiB header, and 1,000 clients. Backups require the original
password; there is no password reset. See [backup-recovery.md](backup-recovery.md)
for user steps, exact format, file layout, retained-copy limits, and durability.

| Check | Result |
|---|---|
| Locked setup/dependency fetch and production frontend build | Passed |
| Interface tests | 47 passed: backup folder selection, password clearing, warnings/errors, verified review/confirmation/cancellation, stale responses, and storage-error recovery to a clean locked screen |
| Linux Rust tests | Full suite 63 passed; final recovery-only rerun 10 passed after exclusive-staging/publication safeguards were tightened |
| Rust format and Clippy all targets with warnings denied | Passed |
| Compiled native desktop workflow | Passed: full existing client/financial workflows; encrypted backup; wrong backup password; process stop during verified review leaves original active; explicit restore retains original database bytes and all saved plans; fresh process detects missing metadata and recovers through the error screen |
| Native visual inspection | Passed for backup controls and authenticated restore review; native OS picker dialogs themselves were not exercised by WebDriver |
| Deferred Windows download metadata | Parsed successfully with PowerShell; packaging not invoked |
| Windows source checks | Run [38496639835](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38496639835) passed from 85f0c13: production frontend build, 47 interface tests and 64 Rust tests, including Windows file locks, pointer replacement, backup/restore failures, and SQLCipher memory-lock regression |
| New Windows installer | Intentionally not built, as requested |
| Clean Windows 11 installed/offline recovery and physical removable-drive/power-loss drill | Not run |

Native artifacts: `/workspace/shared/desktop-smoke-recovery`; logs:
`/workspace/shared/recovery-native-smoke.log`, `recovery-backend-final.log`,
`recovery-backend-final-recovery.log`, and `recovery-frontend-final.log`.
Negative fixtures deliberately cause SQLCipher page-authentication diagnostics;
those checks passed by rejecting damaged data. No verification was disabled.
During implementation, compile errors in type inference/audit transaction usage
and fixture error formatting were corrected before tests passed. Final review
also tightened exclusive staging and ensured a failed temporary-pointer claim
cannot delete a pre-existing file. Snapshot sync handles have write access for
Windows FlushFileBuffers compatibility; Windows source validation is recorded separately.

Source commit `85f0c134adcc536bc0d4b2328f1c7cf119a38895` is uploaded to
main. Windows source CI passed, and installer build, install/launch/uninstall,
metadata preparation, installer upload, and validation-log upload were explicitly
skipped. No manual package run was requested. The full Windows source test log is
retained at `/workspace/shared/recovery-windows-validation.log`. This verifies
Windows core compilation and tests, not installed Windows GUI/picker/offline
recovery or real removable-drive durability.

Recoverable tracked-source and full-history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-recovery-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-recovery.bundle`, refreshed and verified
after the final validation notes commit. These are development source archives;
application data backups use the separate encrypted `.ebcbackup` format.

A durable backup survives a subsequent audit-write failure and reports that
warning; its snapshot contains prior audit history. A post-activation directory
sync failure reports an applied restore with a durability warning. Interruption
can leave incomplete backup files or unreferenced encrypted staging folders,
which are never selected automatically. Previous generations are retained;
supported retained-copy selection/pruning is future work. Future document/report
attachments will need inclusion in an extended format; none are stored yet.

Fictional data remains required while independent security review and clean
Windows acceptance are unfinished. The earlier 0.3.0 download does not include
this feature. Installer creation remains manual opt-in. Next: scoped synthetic
CSV import and explicit review/approval, preserving planning inputs separately.

## Stage 3D: paycheck-based cash-flow calendar — 0.5.0 source

Implemented persistent cash-flow plans with up to 20 separate accounts; exact
opening/running/closing balances; entered weekly, biweekly, semimonthly, monthly,
and irregular one-time schedules; dated bills/loan payments/external savings;
paired internal transfers; confirmed/estimated schedules; a calendar with daily
account review; and a chronological ledger. Dated entries are never manufactured
from monthly budgets or forecasts. Current budget/debt/savings revisions and
amounts are references for reviewing completeness, not automatic transactions.

The engine identifies negative opening, intermediate, and daily balances, bills
before the next entered paycheck, missing funding dates, and concentrated payment
days. Same-day debits precede credits conservatively, keeping shortfalls visible
even when daily/monthly totals are positive. Transfers preserve combined cash;
a reserve account never silently covers checking. Unknown amounts remain unknown
and propagate only to affected accounts. Dates respect leap years, original
monthly anchors, inclusive end dates, and semimonthly collision deduplication.

The counselor can edit a date and preview an alternative without saving. Preview
refreshes references, preserves the draft/baseline, and does not write records,
revisions, or audit events. Adjustments require actual payee permission and do
not guarantee bank processing. Encrypted schema 6 preserves all earlier plans
and immutable audit records; atomic saves and optimistic revisions reject stale
windows, failures roll back, and locking clears private state. Older applications
cannot reopen schema 6. See [cash-flow-calculations.md](cash-flow-calculations.md).

| Check | Result |
|---|---|
| Locked setup, production frontend build, compiled debug native desktop | Passed |
| Interface tests | 40 passed, including 7 cash-flow tests and client navigation/lock guards |
| Linux Rust tests | 53 passed, including 8 independent cash-flow calculation tests and 3 persistence/migration/atomicity tests; foreign firm/counselor scope also covers get/preview/save |
| Independent Python Decimal fixture | Checking -700 with combined +300 before payday; transfer conserves 1,800; closes checking 2,100 + reserve 1,200 = 3,300 |
| Rust format and Clippy all targets with warnings denied | Passed |
| Final native GUI workflow | Passed: account/activity entry, calendar/ledger, independently checked balances and next payday, preview without writing, client isolation, lock/wrong password/unlock, full process restart and original saved date |
| Final calendar visual inspection | Passed after widening cells and adding horizontal scrolling to keep amounts readable in narrow client panels |
| Deferred Windows download metadata | Parsed with PowerShell; packaging not invoked |
| New Windows installer | Intentionally not built, as requested |
| Windows source checks | Run [38492199152](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38492199152) succeeded from 75d3934: production frontend build, 40 interface tests and 54 Rust tests passed, including the Windows-only SQLCipher memory-lock regression |
| Clean Windows 11 offline installed cash-flow workflow | Not run |

Final native artifacts: `/workspace/shared/desktop-smoke-cashflow-verified`.
The initial native run stopped on a transient read-only text GET connection
closure after setup. Driver logs confirmed the connection closed before the
response completed. The harness now retries one GET observation, matching its
existing one-retry locator handling; typing, clicks, saves, and JavaScript are
never replayed. The rerun and final layout run passed the full required workflow.
An initially incomplete TypeScript test fixture was corrected before the successful
production build; one ambiguous test locator was narrowed to the account-name field.

Source commit `75d393425d5db79b10913aa14abf70e5dd619048` is uploaded to main.
Windows CI explicitly skipped installer build, install/launch/uninstall checks,
metadata preparation, installer upload, and validation-log artifact upload.
No manual package run was requested. The full Windows test log is preserved at
`/workspace/shared/cashflow-windows-validation.log`. This confirms Windows source
build/tests, not a newly installed Windows GUI or clean-machine offline acceptance.

Recoverable source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-cashflow-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-cashflow.bundle`; the archive contents
and full-history bundle are verified and refreshed after the final notes commit.
These are development source checkpoints, not backups of client workspace data.

Source-only checkpoint; no installer, bank integration, actual transaction
reconciliation, overdraft fees, holds, holiday shifts, or delayed transfer
settlement. Only entered accounts/activity are projected. Fictional records
remain required while supported encrypted recovery and Windows acceptance are
unfinished. The earlier 0.3.0 download contains neither savings nor cash flow.

## Stage 3C: emergency savings and sinking funds — 0.4.0 source

Implemented persistent client savings plans: emergency targets for 1/3/6/custom
whole months of necessary expenses; liquid/restricted/bill/sinking-fund reserve
separation; current emergency coverage; remaining amounts; capped monthly
contributions and estimated completion dates; dated sinking funds with suggested
contributions rounded upward; alternative emergency contributions; and monthly
budget/debt funding review. The counselor can record priorities and adjust
allocations; the application never moves money or silently rewrites other plans.

Reserve buckets partition one liquid pool, preventing double counting. Unknown
inputs remain unknown; missing expense bases or zero contributions do not invent
goal dates. Month-end contributions respect actual deadline days and leap years.
The last transfer is capped and stops at the target. Joint funding checks reserve
the higher of budget/debt payments and include future debt changes/extras, even
after savings finish. Overallocations and conflicting/missing references are
flagged. Scenario affordability is unavailable when it cannot be checked.
See [savings-calculations.md](savings-calculations.md) for assumptions and examples.

Encrypted schema 5 preserves clients, notes, budgets, debts, and immutable audit
history. Writes/audit are atomic, optimistic revisions reject competing windows,
failed loads cannot replace a plan, and locking removes private UI state.
Refreshing budget/debt references keeps unsaved savings drafts and cannot adopt
another window's changed revision. The older versions cannot open an upgraded
schema 5 workspace.

| Check | Result |
|---|---|
| Locked setup and production frontend build | Passed |
| Frontend tests | 32 passed: exact inputs, native forecasts, target choices, goals/scenarios, safe reference refresh, failed loads/saves, missing affordability, stale responses, and lock/leave guards |
| Linux Rust tests | 42 passed: independent reserve/coverage/rounding/deadline/last-cent/scenario examples, zero/missing/funded states, leap days, past dates, caps/horizon, debt events, scope, competing writes, audit rollback, and schema 1/2/3/4 migrations |
| Independent Python Decimal examples | Confirmed remaining 3,899.99; 200/month requires 20 months with final 99.99; 100/month needs 39 months; 300/month needs 13; 200 over 3 eligible months requires 66.67/month; coverage floors to 0.61 |
| Rust format and Clippy, all targets with warnings denied | Passed |
| Final compiled native desktop workflow | Passed: visible savings/reserve/goal/scenario editing, known independent results, cap at final transfer, client isolation, lock/wrong password, and full process restart with saved savings |
| Windows source checks | Run 38489872140 succeeded from e3e06dc; conclusion confirmed during the cash-flow stage after GitHub access returned |
| New Windows installer/package | Intentionally not built, per request to save packaging time |
| Clean Windows 11 installed/offline savings acceptance | Not run |

Native artifacts: `/workspace/shared/desktop-smoke-savings-final`. The initial
run passed financial checks but lost one read-only native-driver locator connection
at relaunch. The harness now retries that read once; typing, clicks, and saves
are never replayed and all workflow assertions remain required. The final run
passed fully. An incorrect expected last-transfer value in one fixture was
corrected to 99.99 after independent Decimal verification; the calculator already
returned that correct value.

The Windows workflow now always runs frontend/backend checks on normal pushes
and pull requests. Installer creation, installer smoke checks, metadata, and
artifact upload require a manual dispatch with `package_installer: true`.
The default is false. This preserves source validation without creating downloads
for each milestone. The 0.3.0 installer remains the earlier download and does
not contain the savings feature. This milestone remains for fictional records,
with no interest/returns, inflation, withdrawals, changing savings expenses,
automatic funding redistribution, recovery, or independent security certification.

### Preserved checkpoint and remaining CI confirmation

Source commit `e3e06dc84a031fdc8e98605ee3d410e323db408e` was successfully pushed
before authentication stopped working. Windows source-check run
[38489872140](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38489872140)
was observed with successful frontend checks and the backend step in progress.
Later authenticated API reads returned `HTTP 401: Bad credentials`; a fresh
uncached request also failed, and normal Git remote access could no longer
authenticate. Runtime status exposed no configured credential refresh capability.
No credential values were inspected or printed, no alternate identity was used,
and no interactive login or replacement token was requested.

At the end of this savings stage, the final Windows result and skipped-package
step records were unverified; this was not a reported build/test failure.
During the cash-flow stage, normal injected Git/CLI access returned and run
38489872140 was confirmed successful. The local notes are included in the next
source push. The workflow definition
makes installer/package upload steps conditional on manual opt-in, and no such
run was requested. Local checks passed and the complete source was already
uploaded. Final validation notes were committed locally for later synchronization.

Recoverable source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-savings-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-savings.bundle`. They are development
source archives, not backups of client workspace data. Recheck CI and synchronize
any local documentation commits when GitHub authentication is available again.

## Stage 3B: monthly debt payoff planning — 0.3.0

Implemented saved client debt accounts and exact fixed-monthly APR payoff
projections, with comparable snowball/avalanche strategies. Each month reserves
entered minimums, allocates extra money by strategy, caps payments at the amount
owed, and rolls excess to the next account. Plans support known monthly fees,
one-time extra payments, and future changes to the total monthly payment budget.
The interface exposes estimated payoff months, interest/fee/payment totals,
remaining balances, and an interactive per-account monthly schedule.

Money uses exact cents; APR uses exact 0.0001-percentage-point units and 128-bit
intermediate multiplication. No money calculation occurs in React. Missing
active terms or unmodeled interest methods block payoff projections while still
allowing incomplete accounts to be saved. Shortfalls stop before unaffordable
payments; growing balances, horizon/safety limits, unverified terms, and saved
budget-capacity/allocation conflicts are flagged. Partial totals cover only
simulated months, with no invented debt-free date. Both strategies share all
payment events and assumptions. See [debt-calculations.md](debt-calculations.md).

Debt inputs are encrypted in schema 4 with firm/counselor/client scope, optimistic
revisions, atomic writes/audit, and rollback on failure. The migration preserves
clients, notes, budgets, and immutable audit history. Calculated projections are
recomputed and do not overwrite budgets or historical transactions. Private
editor state clears on lock; failed loads and delayed client/session responses
cannot become replacement plans. Unsaved debt edits participate in leave prompts.

Executed checkpoint checks:

| Check | Result |
|---|---|
| Locked setup and production frontend build | Passed |
| Frontend tests | 25 passed: input strings/revisions, supported/unknown terms, schedule inspection, events, failed loads/saves, stale responses, unsaved prompts, and lock privacy |
| Linux Rust tests | 31 passed: independent amortization/comparison/final-payment examples, zero interest, fees/events, minimum shortfall, negative amortization, horizon/model limits, rate/date validation, ties, budget context, scope, competing windows, failed writes, and schema 1/2/3 upgrades |
| Independent Python Decimal fixtures | Single loan: 58.98 interest, 1,058.98 paid, 11 months; two debts: snowball 43.10 interest/643.10 paid, avalanche 35.35 interest/635.35 paid, both 7 months |
| Native desktop build and workflow | Passed on the initial complete compiled bundle: visible two-debt editing, independent strategy totals/allocations, budget coexistence, client isolation, lock/wrong password, and full process restart |
| Final Rust format, Clippy (all targets, warnings denied), backend and native checks | Passed on final source after the allocation-warning improvement |
| Windows 0.3.0 build/installer | Passed run 38485910521: 25 frontend/32 Windows backend tests, release build, silent install, responsive x64 native window, uninstall, and isolated cleanup |
| Clean Windows 11 offline/full installed debt workflow | Not run |

Final native artifacts are at `/workspace/shared/desktop-smoke-debt-final`; the
initial run is retained at `/workspace/shared/desktop-smoke-debt`.
This is a development milestone for fictional accounts. It implements fixed
monthly estimates, not daily accrual, variable/promotional APRs, changing lender
minimum formulas, or nonmonthly contractual payments. Source/account-detail
expansion, actual due-date cash flow, savings goals, recovery, and independent
review remain open. The 0.3.0 upgrade prevents older 0.1.0/0.2.0 apps from opening
schema 4; keep using the newer version and preserve the local data folder.

### Verified 0.3.0 Windows download

[Download the debt-planning development installer](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38485910521/artifacts/11859772481).
[Run 38485910521](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38485910521)
succeeded from source commit `631719ba5811c869b4190603c8eddcef53d82b41`.
The package is `Eisenbart Counseling Suite_0.3.0_x64-setup.exe`; its artifact
expires October 27, 2026 and may require GitHub sign-in with repository access.

The actual downloaded ZIP matched GitHub's digest and passed ZIP integrity. The
installer matched `SHA256SUMS.txt`; its README matched the source commit, version,
debt features, and schema-upgrade notice. Downloaded logs confirmed 25 frontend
and 32 backend tests passed, including the real Windows lock-quota regression.
The separate package log confirmed installation, responsive x64 native window,
uninstall, file removal, and removal of only test-created application data.
Archive inspection also verified the bundled WebView2 offline runtime payload.

Installer SHA-256:
`3965d9f9af6a425b1afa0b0ba39cb93ec776da95fd520e194755184bc8745914`.
Installer size: 220,951,701 bytes. ZIP size: 221,001,487 bytes.
Installer artifact: `11859772481`; validation artifact: `11859682545`.
Verified local downloads/logs: `/workspace/shared/windows-downloads/run-38485910521`.
Recoverable source checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-debt-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-debt.bundle`; these are source/history
archives, not backups of client workspaces.

The installer is unsigned and for fictional accounts only. Windows CI launched
the installed native window but entered no client/budget/debt workflow and used a
runner with preinstalled WebView2. Linux native workflow evidence and Windows
backend/package checks do not establish clean Windows 11 offline or full
installed debt-workflow acceptance. Those checks remain open.

## Stage 3A: exact client budgets — 0.2.0

Implemented the first financial-core checkpoint: one persistent USD budget per
client, custom category/source lines, take-home income frequency normalization,
necessary/discretionary spending, proposed savings, planned debt payment
allocations, and signed surplus/deficit results. Amounts and calculations use
integer cents in Rust; the interface receives exact decimal strings. Weekly,
biweekly, semimonthly, monthly, annual, and irregular monthly-estimate inputs are
supported. Missing amounts and estimates are visibly flagged. Original planning
amounts/frequencies remain stored, and no historical transactions are changed.
See [budget-calculations.md](budget-calculations.md) for rounding, formulas,
limits, and independently checked examples.

Budget saving is authenticated and scoped to the current firm/counselor's client.
Encrypted schema 3 adds budgets and preserves earlier clients, notes, and audit
history. Saves and content-free audit events are atomic, with rollback on failure.
Revision checking prevents another desktop window's older plan from overwriting
a newer one. Failed loads cannot become empty writable replacements. Unsaved
edits participate in leave-record prompts; editing hides outdated totals; locking
clears budget state; delayed responses cannot populate another client's editor.

Executed validation for this checkpoint:

| Check | Result |
|---|---|
| Locked setup and production frontend build | Passed |
| Frontend tests | 18 passed; includes exact input payloads, native result display, missing/zero distinction, estimated irregular income, failed loads, private-state clearing, and stale responses |
| Linux Rust tests | 20 passed; includes independent rounding/totals/deficit examples, invalid/large inputs, encrypted persistence, scope, competing connections, atomic rollback, and schema 1/2 upgrades |
| Independent Python Decimal calculation | Matched the documented household example: 3,250.00 income, 1,633.33 necessary expenses, 716.67 unallocated |
| Rust format and Clippy (`--all-targets`, warnings denied) | Passed |
| Native desktop build | Passed with bundled production frontend |
| Native desktop workflow | Passed: visible budget editing, exact six-line example, encrypted saving, client isolation, locking, wrong-password rejection, and complete process restart with persisted budget |
| Windows 0.2.0 installer | Passed run 38482971562: 18 frontend/21 Windows backend tests, release build, silent install, responsive x64 native window, uninstall, and isolated test-data cleanup |
| Clean Windows 11 offline/full installed budgeting workflow | Not run |

Native artifacts: `/workspace/shared/desktop-smoke-budget-final`. An initial
cloud native-driver attempt lost a connection while typing intake; the app
remained responsive for a failure screenshot. The harness now identifies the
failed method/endpoint without printing input payloads. A fresh final build/run
completed all workflow assertions without retrying commands or weakening checks.

Version 0.2.0 upgrades the local database to schema 3. Version 0.1.0 will refuse
that newer schema; switching back is unsupported. The development build remains
for fictional records only, with no supported backup/restore or password recovery.
This is not the full Stage 3 financial engine: optional budget methods, debt
payoff schedules, savings targets, cash-flow dates, and scenarios remain open.

Windows run [38482182798](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38482182798)
finished application validation but failed test-data cleanup when WebView2 still
held `EBWebView/lockfile`. No installer download was uploaded from that run.
Cleanup retains both known-folder ownership checks and the disposable-runner
restriction, waits at most 45 seconds for I/O failures, fails immediately for
other errors, and still requires the test-created directory to be removed.
It does not kill unrelated browser processes or skip the cleanup check.

### Verified 0.2.0 Windows download

[Download the budget development installer](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38482971562/artifacts/11857823874).
[Run 38482971562](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38482971562)
completed successfully from commit `ff5d2e2402f8e7561adfb96f710d5f2ec62fd579`.
The installer is `Eisenbart Counseling Suite_0.2.0_x64-setup.exe`.
GitHub sign-in may be required; artifact retention expires October 27, 2026.

The final downloaded ZIP matched GitHub's SHA-256 digest and passed ZIP
integrity. The installer matched the bundled checksum; its README matched the
exact source commit, version, budget features, and schema-upgrade notice.
Downloaded Windows logs independently confirmed all 18 frontend and 21 backend
tests passed, including the real memory-lock quota regression. The validation
log confirmed native installation, x64 executable/window, successful uninstall,
and removal of only test-created application data. The bounded cleanup
correction passed the complete rerun. NSIS archive inspection passed and found
the 214,090,448-byte WebView2 offline runtime installer payload.

Installer SHA-256:
`663f1c65e8c1919fffa91917590318a703e31f32541c151f29edb86874cb2cc5`.
Installer size: 220,903,103 bytes. ZIP size: 220,952,850 bytes.
Installer artifact: `11857823874`; validation artifact: `11858447750`.
Local verified files/logs: `/workspace/shared/windows-downloads/run-38482971562`.
The native budget implementation checkpoint is `84f474b`; the subsequent
`ff5d2e2` commit changes only Windows test cleanup and documentation.

This package remains unsigned and for fictional records. Windows CI entered
no client/password/budget workflow, and had preinstalled WebView2. Linux native
workflow evidence and Windows backend/package checks do not establish clean
Windows 11 offline or full installed budgeting acceptance. Those checks and
backup/restore remain open.

## Windows development installer milestone — October 13, 2026

**Download available:** [windows-installer ZIP](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38450281912/artifacts/11842130980).
The verified installer is `Eisenbart Counseling Suite_0.1.0_x64-setup.exe`, built
from source commit `370d4414db381edfe3686e1106d15ea6471e1238` on `main`.
[Windows run 38450281912](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38450281912)
completed successfully. The download expires October 27, 2026 and may require
GitHub sign-in with repository access. Extract the ZIP, read `README.txt`, and
run the installer. See [user-guide.md](user-guide.md) for first use.

Executed on the disposable GitHub-hosted `windows-2025` runner:

- Locked frontend production build and **11 frontend tests passed**.
- Locked MSVC Rust build and **12 backend tests passed**, including the real
  Windows memory-lock quota regression, encrypted persistence, integrity,
  incorrect-password rejection, and scope protections.
- Release NSIS installer built successfully with bundled WebView2 offline runtime.
- Silent installation returned exit code 0; the installed program's PE machine
  was AMD64 (`0x8664`). It displayed a responsive native window with the expected
  title. The isolated test stopped its own process, silently uninstalled with
  exit code 0, and verified removal of the installed files.
- Download instructions/checksums and separate validation logs uploaded.

The installer is **unsigned** and remains a development build for fictional
records. The Windows package test entered no password, counselor account, or
client workflow; it tested installation and native-window launch. The runner
already had a browser runtime. A clean Windows 11 offline install without
preinstalled WebView2, full installed client workflows, and upgrade/data-retention
acceptance remain unrun. No security certification or commercial readiness is
claimed.

Independent checks after downloading the actual artifacts:

| Artifact check | Result |
|---|---|
| GitHub installer ZIP SHA-256 | Matched GitHub's digest; ZIP integrity passed |
| Installer SHA-256 | Matched bundled `SHA256SUMS.txt` |
| Download README source commit | Matched the successful run's full source commit |
| Windows validation log | Confirmed installation, responsive x64 app, uninstall, and isolated cleanup passed |
| NSIS archive integrity | Passed 7-Zip archive test |
| Offline runtime payload | Found 214,090,448-byte `MicrosoftEdgeWebView2RuntimeInstaller.exe` inside the installer |

Installer SHA-256:
`d6faff55f9b5d9db6c8f6783ed64eae1ea5e17e587d50e15e30446f11f831408`.
The installer is 220,875,970 bytes; the download ZIP is 220,925,577 bytes.
Artifact ID: `11842130980`. Validation-log artifact ID: `11841143510`.
Local verified downloads and run logs are retained under
`/workspace/shared/windows-downloads/run-38450281912`.

### Windows crash correction preserved in this checkpoint

The initial Windows run
[38445920011](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38445920011)
built the frontend and Rust code but aborted its tests with a stack overflow;
that run produced no installer. Locked SQLCipher 4.14.0 source and its error log
identified recursive native logging: failed `VirtualLock` allocations log a
memory warning through a formatter that itself allocates through SQLite.

The Windows-only correction configures native log sources once before memory
security activates, excluding MEMORY while retaining CORE, PROVIDER, and MUTEX
diagnostics. Encryption, Argon2id, memory wiping, memory-lock attempts, integrity
checks, and application errors are preserved. Memory locking remains best effort;
native memory-lock failure warnings are omitted on Windows. The Windows-only
subprocess regression forces an actual lock-quota failure and now passes on
Windows. See [security.md](security.md) for this limitation and review needs.

After the correction, the existing 11 Linux backend tests passed again and the
compiled Linux desktop passed the complete synthetic setup/client/note/lock/
restart workflow again. Those artifacts are retained at
`/workspace/shared/desktop-smoke-windows-fix`. The Windows packaging scripts also
passed PowerShell parsing and synthetic guard/header/checksum checks before CI.

GitHub API, logs, and artifact downloads worked through the existing injected
authentication after the environment network settings were updated. No new
credentials were requested. Runtime configuration metadata still reports unknown
network enforcement; successful commands establish access to these endpoints,
not a general network-readiness claim.

## Foundation milestone — October 13, 2026

This earlier checkpoint combined specification stages 1 and 2: an established
repository and a functioning desktop foundation. Advanced financial modules had
not been started at that checkpoint. It was a development build for fictional
records only; the later budget milestone is recorded above.

Implemented:

- Tauri 2 desktop host, React/TypeScript interface, original application icons,
  bundled local assets, and Windows NSIS packaging configuration.
- First-run organization and counselor setup with editable Eisenbart & Co.
  default branding; local password unlock; manual and 15-minute inactivity lock.
- SQLCipher-encrypted SQLite storage, Argon2id key derivation, key/password/KDF
  memory cleanup, schema migrations, integrity checks, and guarded setup.
- Persistent client intake, contact details, household size, goals, lifecycle
  status, searching, filtering, and editing.
- Append-only counselor notes with authorship/timestamps, operational audit
  records without note/contact contents, and server-enforced firm/counselor scope.
- Real client counts and lifecycle summaries; no invented financial results.
- Session and client-switch guards that discard stale responses and clear private
  interface data when locked.
- Reusable cloud setup/activation, backend/frontend tests, native desktop smoke
  harness, Windows build workflow, security notes, and staged roadmap.

The original spreadsheet was unavailable and has not been verified or changed.
No real client information was imported.

## Executed checks

Machine: managed Debian 13 Linux x64, Node 24.19.0, Rust 1.99.0, GTK 3.24.49,
WebKitGTK 2.54.0. Native packages were verified against signed Debian metadata.
The writable native sysroot avoids privileged installation.

| Check | Result |
|---|---|
| `bash scripts/cloud-setup.sh` | Passed; locked npm installation and Cargo fetch, tool activation and native dependency checks |
| `npm run build` | Passed; TypeScript and bundled production frontend |
| `npm test` | Passed: 11 tests, including setup, editing, client-note isolation, lock clearing, and stale-session failures |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline -- --test-threads=2` | Passed: 11 backend tests; persistence, encryption, wrong keys, validation, scope, append-only history, migrations, corruption and existing-file preservation |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed |
| `npm run tauri -- build --debug --no-bundle` | Passed; standalone Linux desktop binary with compiled frontend assets |
| Native GUI workflow | Passed with exit 0 on the final compiled bundle, including a complete app/driver process restart and record persistence |
| Windows executable/NSIS installer build | Passed on run 38450281912; automated install/native-window launch/uninstall passed |
| Clean Windows 11 offline install/use | Not run |
| Independent security review and commercial release review | Not performed |

Binary: `src-tauri/target/debug/eisenbart-counseling-suite`. This Linux development
binary is not a Windows executable or installer. `dist/` is generated frontend
output; opening it alone is not the desktop application.

The native test uses real Tauri IPC and compiled assets, not a browser mock. It
creates a fictional firm/counselor and two clients, saves an authored note, checks
the second client has no such note, edits status/goals, verifies locking clears
private DOM values, rejects a wrong password, unlocks, and checks records after
relaunch. The final test terminates all its private native app/driver processes
and starts a fresh driver against the same temporary data. Temporary application
data is removed afterward. Early test-harness issues with CSS-clipped identity
text and native WebKit session teardown were corrected using full identity values
and a full process restart. All final workflow assertions passed. No live test
processes or driver listeners remained.

Final native command, from the checkout after `source scripts/cloud-env.sh`:

```bash
WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 \
WEBKIT_DISABLE_DMABUF_RENDERER=1 LIBGL_ALWAYS_SOFTWARE=1 \
dbus-run-session -- xvfb-run -a -s '-screen 0 1280x900x24 -nolisten tcp' \
  bash scripts/cloud-native-run.sh python3 scripts/desktop-smoke.py \
  --artifacts /workspace/shared/desktop-smoke-foundation-final-process-restart
```

The native screenshot and driver diagnostics are retained at that artifacts
location. No development server was running or required for the tested app.

SQLCipher prints expected HMAC/decryption diagnostics in intentionally wrong-key
and corrupted-file checks. They contain no test passwords or client contents.
The tests exercise actual encrypted-page authentication; these diagnostics are
not failed test outcomes.

## Reusable environment configuration

The environment draft has saved `install_script` and `start_skill`. Installation
calls the repository's tested `scripts/cloud-setup.sh`; startup instructions cover
tool activation, builds, checks, and new-process native GUI validation. No new
secret bindings were required. Later Windows artifact validation used the
user-reviewed GitHub API, Actions log receiver, and Azure Blob download domains.

The trusted synthetic Linux GUI test needs an explicit WebKit sandbox exception
because this cloud container denies WebKit's nested sandbox. It remains confined
to test processes; product configuration and default helpers preserve the sandbox.
This does not validate Windows WebView2 isolation or certify security.

Draft saving does not publish the environment or demonstrate restoration in a
new task. Review and save the draft in environment settings, then publish the
environment through the product. The foundation source checkpoint is now
uploaded to GitHub. Retained cloud files, uploaded Git history, and restoration
in a newly created cloud task are separate outcomes; a fresh-task restore has
not been verified.

Recoverable source checkpoints are retained outside the checkout at
`/workspace/shared/checkpoints/eisenbart-foundation-source.tar.gz` (tracked
source) and `eisenbart-foundation.bundle` (Git history). These are development
source checkpoints, not backups of application client data. A fresh-task restore
has not been verified; retain or upload the source before relying on a new remote
clone.

## Current priority reference

The current stage and next priority are recorded at the top of this document.
Earlier milestone sections retain their historical plans; they are not a list of
features still missing today. Use the prototype development map for current
working page areas and future implementation entry points.

## Distance to the complete product

Stages 1–2 provide the implemented architecture and desktop foundation; the
unavailable original workbook remains unverified. A tested Windows development
package also completes part of Stage 7. Stages 3–8 are still open:

| Specification stage | Remaining work |
|---|---|
| 3: Financial core | Exact budgets, optional budget methods, fixed-monthly debt, savings, dated cash flow and bounded combined scenarios implemented; advanced debt terms, transfer settlement and broader goal/funding scenarios remain |
| 4: Document processing | CSV, selected text-PDF review, recurring/grouped payment review and running-balance reconciliation implemented; scanned OCR and broader institution adapters remain |
| 5: Dashboard and reports | Approved dashboards, immutable scenario-backed counselor reports, client plans, offline PDFs and bounded final counselor approval implemented; savings progress, native printing and broader report policies remain |
| 6: Security/commercial | Encrypted backup/restore and firm/report branding implemented; clean Windows recovery, password lifecycle, accounts/roles, export/deletion, licensing and independent review remain |
| 7: Production packaging | Signed distribution, upgrade behavior, remaining administrator/recovery/support guides |
| 8: Final verification | Complete installed workflows, clean Windows 11 offline tests, usability and release acceptance |

These stages differ greatly in size and will be split into smaller tested
checkpoints. A stage count is not a percentage or a reliable calendar estimate.
The complete specified application remains substantial work; the development
installer is suitable for evaluating the implemented foundation.
