# Prototype structure and development map — 0.52.0 development source (schema 33)

The user requested the [client workspace redesign](client-workspace-redesign.md)
using a supplied layout reference. The global sidebar now contains Dashboard,
Clients and Settings. A directory-only Clients page opens stable-ID in-app client
tabs. Each client workspace exposes thirteen focused sections in a local rail:
Overview, Profile & intake, Session notes, Monthly budget, Debt payoff, Savings &
sinking funds, Cash-flow calendar, Scenario comparisons, Import statements,
Review & reconciliation, Spending Analysis, Financial dashboard and Assessments & reports.

`App.tsx` owns scoped active-tab navigation and draft/busy guards;
`ClientDirectory.tsx`, `ClientWorkspaceLayout.tsx`, `ClientOverview.tsx` and
`client-workspace.css` compose the redesign. Existing client editors and financial
components remain the implementation entry points. `ImportsEditor.tsx` now also
exposes separately mounted review subtabs. `client_workspace.rs` supplies a
read-only summary from the existing saved-plan engines. Schema 30 added the versioned PDF evidence compatibility gate; schema 31 stores
household planning history, and schema 32 stores scoped transaction-inspection
details. Version 0.51.0 adds no migration. Original report formats remain unchanged.

Only the selected section is mounted. Tabs retain per-client section metadata,
require explicit draft discard and close to the most recently used remaining tab.
Locking clears all client tabs and private UI. Directory counts/statuses/profile
update dates come from real saved records. Future sections below retain their
existing working/planned boundaries; their former global-page names now refer to
local sections and subtabs. See the [complete mapping and limits](client-workspace-redesign.md).

## Statement extraction and review dispositions — working in 0.48.0

Checking/savings and card PDF inspection uses `statement_pdf.rs` and shared
`PdfTransactionInspection.tsx`: transaction candidates, editable confident
preselection, exact draft totals, minimal replayable evidence and immutable
source locators. Card imports remain in Import statements; **Card transactions**
in Review & reconciliation owns decisions. Bank/card source approvals can save an
explicit necessary/discretionary purpose atomically into existing facts histories.
Current analysis recalculates; saved plans/reports keep their documented basis.
[Usage, supported layouts and limits](pdf-import-review.md). No installer requested.

## CRUD coverage and retained history

| Entity | Create/read/update | Remove/archive behavior and remaining gaps |
|---|---|---|
| Clients | Create, directory/search/open, edit profile and lifecycle status | Archive via status; reactivate via another status. Permanent deletion not implemented |
| Intake and follow-ups | Save/edit optional intake and task/appointment versions; complete/cancel outcomes | Remove unsaved tasks only; saved tasks stay in history and can be cancelled |
| Session notes | Append authored dated notes; explicitly correct current versions with a reason and read original/correction history | Immutable original and correction versions; no silent edit/delete. Existing reports retain original evidence; new selections use current heads |
| Budget/debt/savings/cash-flow inputs | Save/reload/edit and remove current plan rows or goals | Removal changes the current saved plan; previously recorded report/scenario snapshots remain historical |
| Statement accounts | Standalone creation, current-name edits, archive/reactivate and change history; immutable original source identity | Other account types/institution metadata and planning-account mappings remain future work |
| Documents / collection | Inspect/import sources and read provenance; create/edit requests, mark receipt and view versions | Cancel unsaved previews/requests; archive/reactivate saved requests. Saved source replacement/deletion is not implemented |
| Transactions | Import, inspect, correct fields and change review decisions | Reject/reverse with retained decision history; no silent destruction of source evidence |
| Category rules | Create/read/edit explicit client/account/type/text rules, with immutable versions and original-decision application history | Archive/reactivate; no automatic approval or financial writes. Cross-account/card/bulk rules remain future work |
| Scenarios/reports/client plans | Explicitly record, reopen and export immutable snapshots | Retained history; amend by recording a new reviewed version, not editing old financial claims |
| Organization/admin | Versioned firm name/contact/PNG/disclosure editing and report capture; password access/change, lock, backup/verified recovery | Counselor/role CRUD, categories/templates, portable export/deletion and licensing remain planned |

A CRUD-oriented interface does not justify deleting audit evidence or rewriting
an assessment already given to a client. Destructive lifecycle work needs explicit
retention policy, source dependency checks, audit and recovery acceptance.

## Spending Analysis — working, focused and complete native acceptance passed

The client workspace has a thirteenth section, **Spending Analysis**. It reuses
approved bank/card sources and existing purpose-review histories, with exact
summary cards, donut/bars, table filtering and discussion-bound Type edits.
Import and approval stay in their own sections. [Calculation rules, opportunities
and safeguards](spending-analysis.md). Current validation is recorded in
[development status](development-status.md).

## Unified review and household workspaces — existing supported workflows

**Review & reconciliation → All transactions** is the default combined chronological
bank/card history, with existing financial decision forms and optional retained PDF
inspection. Dollar shares, purchase-count shares, averages and unknown methods use
the same eligible purchase rows. See [workflow and limits](unified-transaction-review.md).

Existing household tabs expose explicit members, shared expenses, allocations,
settlements and reviewed planning handoffs. They do not infer client relationships
or alter individual records on opening. See [household workflow](household-management.md).
No new linking or household/business functionality is part of consolidation.

## Financial health assessment — completed and verified in 0.51.0

The existing **Financial dashboard** now hosts the explainable health score,
essential-evidence gates, approved income-relative measures, behavioral observations,
source/balance provenance and counselor-selected reduction previews. Existing
Spending Analysis owns purpose edits; the existing scenario/debt/savings/calendar
engines own hypothetical projections. No new workspace section or financial store
was added. [Policy, reuse and acceptance](financial-health-assessment.md); executed
checks, final focused native acceptance and remaining platform/adapter limits are
in development status.

## Future feature areas: return here before implementing

Future areas below have a named UI home with a visible **Planned** state. The
collection checklist is now working, and its row records the implementation.
Planned areas do not save records, run unfinished calculations or claim readiness. Reuse existing
client/organization authorization, exact-money arithmetic, optimistic revisions,
atomic audit and encrypted recovery. Add migrations only when the data needs them.

| UI home | Next implementation files / dependencies | Acceptance before marking working |
|---|---|---|
| Documents → Collection checklist — working in 0.28.0 | `DocumentCollection.tsx`, `document_collection.rs`, `document_collection_store.rs`, schema 18; see [usage and limits](document-collection.md) | Implemented create/reopen/edit/archive/reactivate, explicit receipt and scoped source links, history, stale-save/audit rollback, restart/recovery checks; expand limits and usability only with evidence |
| Import statements → Credit-card statements; Review & reconciliation → Card transactions — CSV/PDF working in 0.48.0 | `cards.rs`, `cards_store.rs`, `statement_pdf.rs`, `CardStatements.tsx`, `PdfTransactionInspection.tsx`, schema 30 | Scoped encrypted sources, explicit CSV mappings, editable PDF preselection and exact selected totals, separate individual review, atomic necessary/discretionary dispositions, immutable evidence/history, exact liability/spending/payment checks and restart/recovery. [Usage and limits](credit-card-review.md) |
| Documents → Additional document types | `pdf.rs`, `imports.rs`, supervised parser adapters and import UI | Broader card/loan/paystub/institution fixtures, bounded parse, original locators, individual review, duplicate/sign/balance controls |
| Documents → Scanned documents / OCR | Extend supervised `pdf_process.rs`; vetted offline OCR assets/dependencies | Scanned/mixed/rotated/low-quality fixtures, time/memory/page limits, no guessed facts, explicit review, offline/license checks |
| Transactions → Classification assistance — suggestions and rule CRUD working through 0.42.0 | `classification.rs`, `classification_store.rs`, `ClassificationAssistance.tsx`, existing `ImportsEditor.tsx` review | Scoped 25-record pages, exact current approvals, unresolved conflicts/unknowns, fingerprint-bound unsaved category drafts, explicit audited decisions, reversal, restart/recovery and client isolation. Scoped rule create/edit/archive/reactivate, exact original-version application, conflict/stale/rollback/recovery checks and no automatic approval. [Rules](local-category-rules.md) and [suggestions](classification-assistance.md) |
| Transactions → Activity history — working in 0.38.0 | `activity_store.rs`, `TransactionActivity.tsx`; existing schema 25 | Scoped 25-event pages, literal/date/account/source/status filters, original/corrected decision evidence, both transfer legs, exact current/obsolete repayment locators, stale-token rejection, ownership, read-only failure and recovery. [Limits](transaction-activity-history.md) |
| Financial Review → Completeness & final validation — working review/history | `completeness.rs`, `coverage_review.rs`, `coverage_review_store.rs`, `SavedCoverageReviews.tsx` | Declared coverage plus scoped schema-21 expected inventory, dates, source choices, missing information and exceptions; stale/audit/recovery controls and immutable format-7 capture. [Review history and constraints](saved-completeness-reviews.md) |
| Financial Review → Savings progress — implemented in 0.52.0 | `savings_progress.rs`, existing verified fact history and `SavingsProgress.tsx` | Real approved/verified sources, explicit period, unknown data, independent expected points and no estimates masquerading as facts |
| Budget Planner → Paycheck allocation — implemented in 0.52.0 | `paycheck.rs` derives saved budget earmarks from existing calendar occurrences | Exact frequency/rounding examples, unknown dates, insufficient funding and no automatic double-counted schedules |
| Debt Planner → Advanced debt terms — implemented in 0.52.0 | Extended `debt.rs` and versioned v2 planning capture; shared debt editor | Independent daily/variable/promotional/fee schedules, payment-order assumptions, unknown terms, comparable strategies and legacy payload compatibility |
| Savings Planner → Funding & drawdown — implemented in 0.52.0 | Existing `savings.rs` schedule with bounded `savings_funding.rs` state and shared contexts | No duplicate reserves or invented funds, bounded drawdown dates, independently checked deficits/recovery and transfers excluded from expenses |
| Cash-Flow Calendar → Transfer settlement — implemented in 0.52.0 | Existing per-account `cashflow.rs` schedules with departure/receipt events and reserve floors | Outgoing/incoming dates, both account shortfalls, reserves, same-day order, no premature combined liquidity or invented settlement dates |
| Financial Scenarios → New goals & funding sources — implemented in 0.52.0 | Existing `scenarios.rs` / `scenarios_store`, shared funding/targeted payment controls | Explicit new goal/funding/account IDs, baseline remains unchanged, exact affordability/milestones, immutable linked report/recovery validation |
| Reports → Final report checklist — approval working | `final_reports.rs`, `final_reports_store.rs`, `FinalReportApproval.tsx`, report PDF | Schema-22 scoped immutable counselor approval, current evidence gates, account/scenario decisions, exact parent binding, atomic audit, final PDF and restart/recovery. [Policy limits and future work](final-report-approval.md) |
| Reports → Print & presentation — PDF layouts / Windows viewer handoff working in 0.40.0 | `ReportPresentation.tsx`, `report_presentation.rs`, `report_presentation_store.rs`, `assessment_pdf.rs`; schema 25 unchanged | Summary-first page, complete evidence appendix, original detailed option, distinct internal/v1/v2 bases, exact figures/unknowns, stale/scoped/no-overwrite export, restart/recovery. Explicit unchanged-file Windows viewer handoff with consent/expiry/session checks; choose Print in that viewer. In-app job submission and installed Windows viewer/printer acceptance remain open. [Usage](windows-pdf-print-workflow.md) |
| Clients → History & records — working in 0.43.0 | `ClientHistory.tsx`, `NoteCorrection.tsx`, `client_history_store.rs`, schema 27; original note/report formats unchanged | Scoped 25-event pages, literal/type/Chicago-date filters, source-section navigation, explicit correction reasons/history, stale/audit/recovery controls, original report bytes retained and current-head report selection. [Usage and limits](client-history.md) |
| Documents → Statement accounts (working) | `StatementAccounts.tsx`, `statement_accounts_store`, schema 19 | Scoped standalone create/rename/archive/reactivate, revision/audit/recovery checks; original source/report associations preserved; no destructive deletion |
| Settings → Branding — working in 0.30.0 | `FirmBranding.tsx`, `branding.rs`, `branding_store.rs`, schema 20, format-6 assessments | Implemented bounded normalized local PNG/contact/disclosure, guarded revision/audit saves, exact immutable capture, old payload/digest preservation and recovery; see [limits and future work](firm-report-branding.md) |
| Settings → Counselors & roles | Auth/session/storage ownership and role model before UI CRUD | Explicit account lifecycle and permissions for reads/writes/approvals/reports/admin, fail-closed access, isolation and recovery drills |
| Settings → Security — password change and configurable inactivity working through 0.45.0 | `PasswordChange.tsx`, `password_change.rs`, `InactivitySettings.tsx`, `security_policy.rs`, generation activation and command operation lock; schema 29 | Current-password authentication, new confirmation, retained-copy disclosure, verified copy-before-switch, failure rollback, interrupted processes, stale instances, old/new backup passwords and private UI clearing. Versioned 1–60 minute inactivity settings now preserve the legacy 15-minute default, use no-refresh background reads, enforce other-instance shortening, and survive restart/recovery; independent/installed Windows acceptance remains open. [Usage](workspace-password-change.md) |
| Settings → Backup & recovery | `backup.rs`; current verified recovery flow already works | Scheduled backups/retained-copy selection need bounded local scheduling, failures, durable restore and clean Windows offline drills |
| Settings → Data management | Authorized client export/retention/deletion services | Reviewable scope/dependencies, encryption/disclosure, audit, no cross-client data loss, cancellation and recovery tests |
| Settings → Licensing | Offline license/update architecture and distribution metadata | Offline grace/expiry rules, signed updates/installers, third-party license inventory, support/legal review; no certification claims |

The unprovided original workbook remains an unverified reference. Remaining
release work includes clean Windows 11 offline install/upgrade/uninstall and full
installed workflows, realistic volume tests, accessibility/usability, signing,
security/privacy review, documentation and commercial/legal acceptance. Page
structure does not complete these requirements.

## Current priority and retained implementation history

Complete and validate the user-requested [financial planning subpages](financial-planning-subpages.md).
All six subpages are now connected to their existing homes and engines, including
new goals, named sources, targeted extras and explicit scenario settlement changes.
Final full/native/CI acceptance is in progress. The previous audit
review remains historical context; no merge or installer is authorized.


The [end-to-end workflow integration audit](workflow-integration-audit.md) now
verifies the consolidated 0.51.0 source. Its review remains open; the newly requested 0.52.0 planning completion is now the
active task, followed by human review and counselor evaluation with fictional records.
Unified review (0.50.0) and financial
health (0.51.0) are implemented; their current validation belongs in development
status. Do not restart them as future projects. Broader adapters/OCR, retained-copy
selection, lender-specific financial terms and installed Windows acceptance remain separate
priorities requiring explicit scope. No new installer without a request.

Existing password changes/inactivity policy, client history, category-rule CRUD,
report presentation and final approval must be reused. The 0.47.0 download is a
historical package rather than a current source delivery. The milestone sections
below retain original sequencing and acceptance context; their former “next” steps
are superseded by later implemented milestones and the current priority above.

### 0.35.0 — integrated provisional bank/card review

Financial Review → Bank & card activity now implements durable card account
creation/rename/archive/reopen, explicit statement assignment/unassignment,
one-to-one repayment matching/removal, preserved version history and exact calendar
review. `card_review.rs`/`card_review_store.rs` own calculations, scope, provenance,
revision/audit transactions and recovery. `CardFinancialReview` owns guarded forms;
`AssessmentRecords` captures reviewed format-8 supplements and `assessment_pdf`
prints them. Existing bank figures, manual balance facts and planning are not
silently changed. Gaps/overlaps, mismatches, adjustments and stale approvals block
combined observed spending. Independent example: 749.96 bank cash, 1287.10 spending,
50.03 repayment excluded. Final card-inclusive approval remains blocked.

Next priority: card-inclusive household inventory and balance/fact mapping with a
versioned completeness policy. Account identity and repayment links now exist;
do not reimplement them or relax bank/card final gates without complete evidence.
Preserve legacy report bytes and v1 readiness receipts. Cross-period/split payments
need explicit semantics, not automatic adjustments. Broader document adapters,
OCR, direct printing, installed Windows acceptance and commercial/multiple-counselor
work remain separate. See [bank-card-review.md](bank-card-review.md).

Acceptance for the next policy includes dated card applicability, explicit
necessary/discretionary classifications bound to card approval IDs, verified
closing-debt mappings to the existing inventory without duplicate debt, unknown
and discrepancy gates, and fresh professional review. Categories alone cannot
supply verified spending roles. New fields/calculators/policies require versioned
capture and unchanged historical report bytes. Unsupported repayment timing and
split cases remain blocked until separately defined and tested.

### 0.36.0 — reviewed card completeness, spending roles and debt mappings

Financial Review → Card completeness now records known/unprovided cards,
applicability/dates, explicitly selected sources, role classifications tied to
approvals, distinct existing-debt mappings and preserved read-only history.
`card_facts.rs`/`card_facts_store.rs` own validation, exact calculations, scope,
immutable source reconstruction, audit and recovery. `CardCompleteness` owns
private draft/busy guards. Format-9 `AssessmentRecords` and internal PDFs capture
the exact reviewed evidence with the original bank/card supplement. Zero net
unclassified spending still has an unresolved-row gate. Mappings compare debt;
they never add household debt a second time. See
[card-completeness-evidence.md](card-completeness-evidence.md).

Next priority: a separately versioned card-inclusive final policy using existing
bank completeness, recorded card evidence and current repayment approvals. Add
fresh professional confirmation and parent/digest-bound approval/export without
changing old policy receipts. Keep unsupported credits, early closure balance
semantics, split/cross-period repayments and unprovided obligations blocked.
Preserve format-9 evidence/calculator behavior and legacy report bytes. Existing
card identity/assignment/role/debt-mapping CRUD now works; build on it. Other
original document/OCR/printing/commercial and installed Windows work remains open.

### 0.37.0 — versioned card-inclusive final approval

`card_final_policy.rs` combines existing bank gates with saved format-9 card
inventory/roles/debt evidence and exact current repayment links. All required gates
precede a format-2 `counselor-final-report-v2` approval. The optional original
summary distinguishes bank cash, combined spending and existing debt, while
`FinalReportApproval` collects separate card notes/confirmation. Scoped storage,
audit, receipt/date rechecks, PDF preflight and recovery reuse the existing final
report service. Original v1 receipts and calculators stay frozen. See
[policy and limitations](card-inclusive-final-approval.md).

### 0.38.0 — client-wide transaction activity history

`activity_store.rs` derives read-only bank/card import and decision events from
existing scoped readers in one transaction. `TransactionActivity.tsx` provides
literal text/date/account/source/status filters, stable fingerprint-bound pages,
original/corrected evidence and exact counterpart decision searches. Current
transfer/repayment context is explicitly separate from historical event state.
No new storage, review mutations or financial calculator is introduced. See
[usage and limits](transaction-activity-history.md). Build on this completed page;
report presentation/printing follows in 0.39.0 below.

### 0.39.0 — summary-first counselor PDF presentation

`ReportPresentation.tsx` selects existing scoped records and either the new
`counselor-summary-v1` layout or original detailed export. Rust constructs the
summary from original validated money strings and checks a context receipt before
writing. Internal and v1 bank-only summaries stay distinct from v2 combined
approval figures. The new first page preserves full identity, period and review
status; complete original evidence and firm disclosures follow in an appendix.
No schema, calculator, policy or immutable payload changes are introduced. Direct
in-app printing is still planned. See [usage and limits](report-presentation.md).

### 0.40.0 — explicit Windows saved-PDF print workflow

A successful presentation export can be opened in the Windows PDF viewer only by
explicit user action and confidentiality acknowledgement. Rust checks a scoped,
expiring session request, unchanged source receipt and exact file bytes; lock,
restart and recovery invalidate it. The export remains user-managed/readable.
No shell interpreter, temporary PDF or financial write is introduced. Windows
request acceptance is not print success. Unsupported hosts show manual guidance.
Source/native verification and installed Windows acceptance are recorded separately
in [development status](development-status.md). No installer is created.

### 0.41.0 — local bank category suggestions

Working Classification assistance reuses current approved merchant evidence for
same-account/type missing-category expenses/fees. Bounded pages, conflicts and
unknowns are explicit; a suggestion only fills an unsaved existing review field.
All saves retain existing audit/revision/history/financial boundaries. Schema 25
is unchanged. Saved reusable rules and larger evidence sets remain future work.
See [usage](classification-assistance.md) and [executed checks](development-status.md).

### 0.42.0 — reusable local category rules

`CategoryRules.tsx`, `category_rules.rs` and `category_rules_store.rs` implement
scoped immutable rule CRUD and original-decision provenance in schema 26.
Classification reuses exact merchant identity and current evidence; final rule
application rechecks both source and rule context atomically through the existing
transaction review. No automatic approval or financial writes occur on rule save.
See [usage and limits](local-category-rules.md) and [executed checks](development-status.md).

### 0.43.0 — client history and reviewed note corrections

The Clients history page now combines bounded source metadata and note/correction
content, with literal/type/date filters, stable pages and source-section links.
Session notes support explicit append-only corrections, optimistic fingerprints
and retained original/reason history. New reports select current note heads; old
report bytes and IDs remain original and changed context requires renewed review.
See [usage](client-history.md) and [executed checks](development-status.md).

### 0.44.0 — workspace password change and recovery safeguards

Settings → Security creates a verified new encrypted generation, atomically
activates it and locks the workspace. Whole-command serialization prevents
cooperating stale instances from writing to a retired generation. Older backups
and retained copies keep their original passwords; no forgotten-password reset
is introduced. Schema 28 only extends immutable audit events. See
[design and limits](workspace-password-change.md) and executed development status.

### 0.45.0 — encrypted configurable inactivity policy

Settings → Security saves immutable revisioned 1–60 minute policy changes with
atomic audit. Rust uses the latest policy for every authenticated access and
background expiry; observation does not refresh activity and corruption locks
access. Existing workspaces/older backups retain the 15-minute default. Recovery
validates history, password changes preserve it and UI locking clears private
state. Pending review/preview expiry remains fixed. See
[design and validation scope](inactivity-locking.md).

### 0.47.0 — approved spending analysis and reviewed Type changes

`SpendingAnalysis.tsx` and `spending_analysis.rs` supply exact live bank/card
spending, category charts, table filters, comparison and supported opportunities.
Discussion-bound Necessary/Discretionary changes reuse existing immutable purpose
stores, preserve source evidence and captured reports, recalculate current results
and require renewed card completeness review. Schema 29 remains unchanged.
See [scope and limits](spending-analysis.md) and [executed validation](development-status.md).

## Household workspaces — working in 0.49.0

Implemented the supplied household-management stage as an extension of Clients:
Household directory and top tabs, ten-section collapsible household rail, exact
shared allocation previews and effective revisions, member linking/consent,
separate payment/settlement ledger, authorized source matching and shared purpose
history, controlled individual budget/cash-flow reconciliation, shared goals,
proposals and internal historical PDF reports. The navy/white/slate/gold reference
informs the compact overview cards, donut, contribution bars and tables.

See [workflow, calculation basis and bounded limitations](household-management.md)
and the [original household brief](household-management-specification.md).
Household state survives tab changes; saved records survive restart and encrypted
recovery. Monthly responsibilities use applicable due-date agreements; no daily
proration, private income/account import or personal debt merging is inferred.

Acceptance requires exact $600/$900/$900 shares for $2,400 rent, no spending
inflation from internal reimbursements, explicit individual reconciliation,
source/consent revocation handling, preserved history, tab/draft privacy on lock,
native desktop/narrow layout, restart and recovery. Execution results belong in
`development-status.md`, separately from this functionality map. No installer.

## Unified transaction review — working in 0.50.0, native acceptance passed

**Review & reconciliation → All transactions** combines one current row per bank
or card record, sorted by transaction date with stable same-day IDs. Combined
filters/search, posting-date basis, bounded pages, original/history access and
optional PDF inspection extend existing intake/review rather than duplicate it.
Focused bank/card forms retain their original audited approval/purpose APIs.
Source-bound method/posting details and reviewed institution/masked account
metadata use an encrypted append-only schema-32 journal. Suspected duplicates and
explicitly confirmed rejected duplicates remain distinct; reapproval invalidates
old detail bindings. Reconciled approved purchase-method figures include unknown
coverage and exclude payments/transfers/refunds/fees/interest/duplicates.

[Baseline reuse checklist](unified-transaction-review-checklist.md), [brief](unified-transaction-review-specification.md)
and [workflow/limits](unified-transaction-review.md) define the feature. Loan,
mortgage and credit-line transaction adapters remain unavailable; plans/balances
are not invented transaction sources. Native original rendering, focused financial
decisions, exact totals, narrow layout, lock/restart and recovery passed. The full
regression and final localized channel refinement are recorded separately in
`development-status.md`. No installer without an explicit request.
