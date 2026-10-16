# Prototype structure and development map — 0.44.0 source

The user has prioritized a coherent CRUD application structure before further
advanced financial features. This checkpoint exposes all twelve main sections
from specification section 5, using existing persistent workflows. The original
[product specification](product-specification.md) remains governing scope.
A page being present does not mean every capability on it is implemented.

## Navigation and implementation entry points

`src/workspaceNavigation.ts` defines main sections and client subsections.
`src/App.tsx` owns unlocked navigation, selected-client context and draft/busy
checks. `ClientEditor` selects existing components for focused pages; **Complete
record** retains the integrated workflow. `src/ImportsEditor.tsx` has explicit
all/documents/transactions/review/reports presentations over the same scoped
Rust-backed data. Do not add another persistence layer or copy the calculators.
`src/PlannedWorkspaceAreas.tsx` distinguishes working subpages from named future areas, and
`src/AdministrationPages.tsx` hosts working/read-only/planned settings pages.

| Main section | Working parts / current entry point |
|---|---|
| Dashboard | Scoped practice counts, recent clients, appointments and follow-ups (`FollowUpOverview.tsx`) |
| Clients | Directory/search/filter, create/edit/status/archive; Profile, Intake & follow-ups, Session notes, History & records, Complete record (`App.tsx`, `ClientCare.tsx`) |
| Financial Review | Approved-data dashboard, explicit financial facts, running-balance checks, recurring-payment review and live completeness diagnostics and versioned reviews (`ImportsEditor.tsx`, `CompletenessReview.tsx`, `SavedCoverageReviews.tsx`) |
| Transactions | Filterable review queue, source correction/approval/rejection/reversal/history, matched transfers; client-wide searchable bank/card history and local category suggestions, reusable rule CRUD/history and unsaved review drafts (`ImportsEditor.tsx`, `TransactionActivity.tsx`, `ClassificationAssistance.tsx`) |
| Documents | Bank CSV/selectable-text PDF import, separate signed-card CSV review, statement account management, source documents/checks and versioned collection checklist (`ImportsEditor.tsx`, `DocumentCollection.tsx`, `StatementAccounts.tsx`) |
| Budget Planner | Saved income/expenses/allocations, remove/edit rows, optional budget method comparisons (`BudgetEditor.tsx`) |
| Debt Planner | Saved debts, payment changes/extras and monthly snowball/avalanche projections (`DebtEditor.tsx`) |
| Savings Planner | Saved reserves, emergency targets, sinking-fund goals and contribution comparisons (`SavingsEditor.tsx`) |
| Cash-Flow Calendar | Saved account balances, dated schedules/transfers, calendar/ledger and non-writing previews (`CashflowEditor.tsx`) |
| Financial Scenarios | Temporary comparisons, explicitly saved scenarios and historical review (`ScenarioComparisons.tsx`) |
| Reports | Immutable internal assessments, selected intake/evidence/scenarios, client action plans, summary-first/detailed PDF exports, captured-source final-report checklists and audited counselor approval (`AssessmentRecords.tsx`, `ClientActionPlans.tsx`, `CompletenessReview.tsx`, `ReportPresentation.tsx`) |
| Settings and Administration | Workspace identity, security/lock, reviewed workspace password change, encrypted backup, lock-to-restore route, help; working firm/report branding and named future roles/data/licensing pages (`AdministrationPages.tsx`) |

Client pages require a selected client and display that context. Switching a
page/client ignores late responses and checks unsaved persistent-record drafts
or ongoing writes. Explicitly accepting discard remounts the workflow. Temporary
scenario previews are intentionally unsaved and reset on leaving; save a reviewed
scenario to retain it. Lock/unlock clears selected clients and page state.
Main navigation is keyboard reachable, identifies the current page and focuses
the heading on page/area changes. Sidebar navigation scrolls in short windows.
No workspace or confidential data is stored in browser local storage or a URL.

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

## Future feature areas: return here before implementing

Future areas below have a named UI home with a visible **Planned** state. The
collection checklist is now working, and its row records the implementation.
Planned areas do not save records, run unfinished calculations or claim readiness. Reuse existing
client/organization authorization, exact-money arithmetic, optimistic revisions,
atomic audit and encrypted recovery. Add migrations only when the data needs them.

| UI home | Next implementation files / dependencies | Acceptance before marking working |
|---|---|---|
| Documents → Collection checklist — working in 0.28.0 | `DocumentCollection.tsx`, `document_collection.rs`, `document_collection_store.rs`, schema 18; see [usage and limits](document-collection.md) | Implemented create/reopen/edit/archive/reactivate, explicit receipt and scoped source links, history, stale-save/audit rollback, restart/recovery checks; expand limits and usability only with evidence |
| Documents → Credit-card statements — signed CSV review working | `cards.rs`, `cards_store.rs`, `CardStatements.tsx`, schema 23 | Scoped encrypted sources, explicit signed mapping, individual decisions/history, exact liability/spending/payment checks, bank-report guard and restart/recovery. [Limits and integration work](credit-card-review.md) |
| Documents → Additional document types | `pdf.rs`, `imports.rs`, supervised parser adapters and import UI | Synthetic card/loan/paystub/institution fixtures, bounded parse, original locators, individual review, duplicate/sign/balance controls |
| Documents → Scanned documents / OCR | Extend supervised `pdf_process.rs`; vetted offline OCR assets/dependencies | Scanned/mixed/rotated/low-quality fixtures, time/memory/page limits, no guessed facts, explicit review, offline/license checks |
| Transactions → Classification assistance — suggestions and rule CRUD working through 0.42.0 | `classification.rs`, `classification_store.rs`, `ClassificationAssistance.tsx`, existing `ImportsEditor.tsx` review | Scoped 25-record pages, exact current approvals, unresolved conflicts/unknowns, fingerprint-bound unsaved category drafts, explicit audited decisions, reversal, restart/recovery and client isolation. Scoped rule create/edit/archive/reactivate, exact original-version application, conflict/stale/rollback/recovery checks and no automatic approval. [Rules](local-category-rules.md) and [suggestions](classification-assistance.md) |
| Transactions → Activity history — working in 0.38.0 | `activity_store.rs`, `TransactionActivity.tsx`; existing schema 25 | Scoped 25-event pages, literal/date/account/source/status filters, original/corrected decision evidence, both transfer legs, exact current/obsolete repayment locators, stale-token rejection, ownership, read-only failure and recovery. [Limits](transaction-activity-history.md) |
| Financial Review → Completeness & final validation — working review/history | `completeness.rs`, `coverage_review.rs`, `coverage_review_store.rs`, `SavedCoverageReviews.tsx` | Declared coverage plus scoped schema-21 expected inventory, dates, source choices, missing information and exceptions; stale/audit/recovery controls and immutable format-7 capture. [Review history and constraints](saved-completeness-reviews.md) |
| Financial Review → Savings progress | Dated verified balance facts and `FactMetricsView.tsx` | Real approved/verified sources, explicit period, unknown data, independent expected points and no estimates masquerading as facts |
| Budget Planner → Paycheck allocation | Reuse `budget.rs` and `cashflow.rs`; define period allocations first | Exact frequency/rounding examples, unknown dates, insufficient funding and no automatic double-counted schedules |
| Debt Planner → Advanced debt terms | Extend Rust debt models/versioned capture, not React math | Independent daily/variable/promotional/fee schedules, payment-order assumptions, unknown terms, comparable strategies and legacy payload compatibility |
| Savings Planner → Funding & drawdown | Extend `savings.rs` and shared `planning.rs` contexts | No duplicate reserves or invented funds, bounded drawdown dates, independently checked deficits/recovery and transfers excluded from expenses |
| Cash-Flow Calendar → Transfer settlement | Extend per-account `cashflow.rs` events and persisted assumptions | Outgoing/incoming dates, both account shortfalls, reserves, same-day order, no premature combined liquidity or invented settlement dates |
| Financial Scenarios → New goals & funding sources | `scenarios.rs` / `scenarios_store` and control components | Explicit new goal/funding/account IDs, baseline remains unchanged, exact affordability/milestones, immutable linked report/recovery validation |
| Reports → Final report checklist — approval working | `final_reports.rs`, `final_reports_store.rs`, `FinalReportApproval.tsx`, report PDF | Schema-22 scoped immutable counselor approval, current evidence gates, account/scenario decisions, exact parent binding, atomic audit, final PDF and restart/recovery. [Policy limits and future work](final-report-approval.md) |
| Reports → Print & presentation — PDF layouts / Windows viewer handoff working in 0.40.0 | `ReportPresentation.tsx`, `report_presentation.rs`, `report_presentation_store.rs`, `assessment_pdf.rs`; schema 25 unchanged | Summary-first page, complete evidence appendix, original detailed option, distinct internal/v1/v2 bases, exact figures/unknowns, stale/scoped/no-overwrite export, restart/recovery. Explicit unchanged-file Windows viewer handoff with consent/expiry/session checks; choose Print in that viewer. In-app job submission and installed Windows viewer/printer acceptance remain open. [Usage](windows-pdf-print-workflow.md) |
| Clients → History & records — working in 0.43.0 | `ClientHistory.tsx`, `NoteCorrection.tsx`, `client_history_store.rs`, schema 27; original note/report formats unchanged | Scoped 25-event pages, literal/type/Chicago-date filters, source-section navigation, explicit correction reasons/history, stale/audit/recovery controls, original report bytes retained and current-head report selection. [Usage and limits](client-history.md) |
| Documents → Statement accounts (working) | `StatementAccounts.tsx`, `statement_accounts_store`, schema 19 | Scoped standalone create/rename/archive/reactivate, revision/audit/recovery checks; original source/report associations preserved; no destructive deletion |
| Settings → Branding — working in 0.30.0 | `FirmBranding.tsx`, `branding.rs`, `branding_store.rs`, schema 20, format-6 assessments | Implemented bounded normalized local PNG/contact/disclosure, guarded revision/audit saves, exact immutable capture, old payload/digest preservation and recovery; see [limits and future work](firm-report-branding.md) |
| Settings → Counselors & roles | Auth/session/storage ownership and role model before UI CRUD | Explicit account lifecycle and permissions for reads/writes/approvals/reports/admin, fail-closed access, isolation and recovery drills |
| Settings → Security — password change working in 0.44.0 | `PasswordChange.tsx`, `password_change.rs`, generation activation and command operation lock; schema 28 | Current-password authentication, new confirmation, retained-copy disclosure, verified copy-before-switch, failure rollback, interrupted processes, stale instances, old/new backup passwords and private UI clearing. Configurable inactivity and independent/installed Windows acceptance remain open. [Usage](workspace-password-change.md) |
| Settings → Backup & recovery | `backup.rs`; current verified recovery flow already works | Scheduled backups/retained-copy selection need bounded local scheduling, failures, durable restore and clean Windows offline drills |
| Settings → Data management | Authorized client export/retention/deletion services | Reviewable scope/dependencies, encryption/disclosure, audit, no cross-client data loss, cancellation and recovery tests |
| Settings → Licensing | Offline license/update architecture and distribution metadata | Offline grace/expiry rules, signed updates/installers, third-party license inventory, support/legal review; no certification claims |

The unprovided original workbook remains an unverified reference. Remaining
release work includes clean Windows 11 offline install/upgrade/uninstall and full
installed workflows, realistic volume tests, accessibility/usability, signing,
security/privacy review, documentation and commercial/legal acceptance. Page
structure does not complete these requirements.

## Next implementation priority

The 0.44.0 source implements **local workspace password change and recovery safeguards** in Settings → Security. Complete independent security and installed Windows interruption/recovery acceptance before real use. Next bounded implementation priority: configurable inactivity locking, with backend-owned bounds and timeout enforcement, preserved lock privacy and existing fixed-timeout compatibility. Scheduled backups and retained-copy selection/pruning remain separate work. See [password-change usage and design](workspace-password-change.md).

Client history and reviewed note corrections are implemented in 0.43.0; see [usage and limits](client-history.md).

Reusable local category-rule CRUD is implemented in 0.42.0, including archive/reactivation and exact original-version application history; do not rebuild it.

The bounded Windows saved-PDF handoff is implemented in 0.40.0. Printing is chosen
in the external viewer; direct job submission and actual installed Windows
viewer/printer acceptance remain separate. Do not redo report layouts or approvals.

Card account CRUD, assignments, repayments, completeness/roles/debt mappings and
bounded versioned final approval are implemented through 0.37.0 below. Build on
them. Other loan obligations, unsupported refund roles, credits, early closure
and split/cross-period repayments stay blocked until modeled and tested. Broader
adapters/OCR/printing/advanced models, security/commercial review and installed
Windows acceptance remain separate original requirements.

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
