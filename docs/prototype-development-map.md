# Prototype structure and development map — 0.28.0 source

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
`src/PlannedWorkspaceAreas.tsx` defines named future areas, and
`src/AdministrationPages.tsx` hosts working/read-only/planned settings pages.

| Main section | Working parts / current entry point |
|---|---|
| Dashboard | Scoped practice counts, recent clients, appointments and follow-ups (`FollowUpOverview.tsx`) |
| Clients | Directory/search/filter, create/edit/status/archive; Profile, Intake & follow-ups, Session notes, History & records, Complete record (`App.tsx`, `ClientCare.tsx`) |
| Financial Review | Approved-data dashboard, explicit financial facts, running-balance checks and recurring-payment review (`ImportsEditor.tsx` review page) |
| Transactions | Filterable review queue, source correction/approval/rejection/reversal/history, matched transfers (`ImportsEditor.tsx` transactions page) |
| Documents | CSV/selectable-text PDF import, statement account management, source documents/checks and versioned collection checklist (`ImportsEditor.tsx`, `DocumentCollection.tsx`, `StatementAccounts.tsx`) |
| Budget Planner | Saved income/expenses/allocations, remove/edit rows, optional budget method comparisons (`BudgetEditor.tsx`) |
| Debt Planner | Saved debts, payment changes/extras and monthly snowball/avalanche projections (`DebtEditor.tsx`) |
| Savings Planner | Saved reserves, emergency targets, sinking-fund goals and contribution comparisons (`SavingsEditor.tsx`) |
| Cash-Flow Calendar | Saved account balances, dated schedules/transfers, calendar/ledger and non-writing previews (`CashflowEditor.tsx`) |
| Financial Scenarios | Temporary comparisons, explicitly saved scenarios and historical review (`ScenarioComparisons.tsx`) |
| Reports | Immutable internal assessments, selected intake/evidence/scenarios, client action plans and PDF exports (`AssessmentRecords.tsx`, `ClientActionPlans.tsx`) |
| Settings and Administration | Workspace identity, security/lock, encrypted backup, lock-to-restore route, help; named future branding/roles/data/licensing pages (`AdministrationPages.tsx`) |

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
| Session notes | Append authored dated notes and read history | Silent edit/delete not offered; reviewed amendments and a combined activity view remain future work |
| Budget/debt/savings/cash-flow inputs | Save/reload/edit and remove current plan rows or goals | Removal changes the current saved plan; previously recorded report/scenario snapshots remain historical |
| Statement accounts | Standalone creation, current-name edits, archive/reactivate and change history; immutable original source identity | Other account types/institution metadata and planning-account mappings remain future work |
| Documents / collection | Inspect/import sources and read provenance; create/edit requests, mark receipt and view versions | Cancel unsaved previews/requests; archive/reactivate saved requests. Saved source replacement/deletion is not implemented |
| Transactions | Import, inspect, correct fields and change review decisions | Reject/reverse with retained decision history; no silent destruction of source evidence |
| Scenarios/reports/client plans | Explicitly record, reopen and export immutable snapshots | Retained history; amend by recording a new reviewed version, not editing old financial claims |
| Organization/admin | Read setup identity; password access, lock, backup/verified recovery | Identity editing, counselor/role CRUD, password changes, portable export/deletion and licensing remain planned |

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
| Documents → Additional document types | `pdf.rs`, `imports.rs`, supervised parser adapters and import UI | Synthetic card/loan/paystub/institution fixtures, bounded parse, original locators, individual review, duplicate/sign/balance controls |
| Documents → Scanned documents / OCR | Extend supervised `pdf_process.rs`; vetted offline OCR assets/dependencies | Scanned/mixed/rotated/low-quality fixtures, time/memory/page limits, no guessed facts, explicit review, offline/license checks |
| Transactions → Classification assistance | Pure local suggestions plus scoped decision/rule persistence; `RecurringReview.tsx` conventions | Suggestions never auto-approve; immutable provenance, false-positive correction, undo/reversal and client isolation |
| Transactions → Activity history | Bounded scoped history query across existing review records | Stable pagination/filtering, corrections and both transfer legs, authorized scope, unreadable history failure and no writes |
| Financial Review → Completeness & final validation | `dashboard.rs`, source/account coverage and final report eligibility | Missing data never becomes zero, rejected/pending/out-of-period exclusion, coverage gaps, stale contexts and explicit blocking reasons |
| Financial Review → Savings progress | Dated verified balance facts and `FactMetricsView.tsx` | Real approved/verified sources, explicit period, unknown data, independent expected points and no estimates masquerading as facts |
| Budget Planner → Paycheck allocation | Reuse `budget.rs` and `cashflow.rs`; define period allocations first | Exact frequency/rounding examples, unknown dates, insufficient funding and no automatic double-counted schedules |
| Debt Planner → Advanced debt terms | Extend Rust debt models/versioned capture, not React math | Independent daily/variable/promotional/fee schedules, payment-order assumptions, unknown terms, comparable strategies and legacy payload compatibility |
| Savings Planner → Funding & drawdown | Extend `savings.rs` and shared `planning.rs` contexts | No duplicate reserves or invented funds, bounded drawdown dates, independently checked deficits/recovery and transfers excluded from expenses |
| Cash-Flow Calendar → Transfer settlement | Extend per-account `cashflow.rs` events and persisted assumptions | Outgoing/incoming dates, both account shortfalls, reserves, same-day order, no premature combined liquidity or invented settlement dates |
| Financial Scenarios → New goals & funding sources | `scenarios.rs` / `scenarios_store` and control components | Explicit new goal/funding/account IDs, baseline remains unchanged, exact affordability/milestones, immutable linked report/recovery validation |
| Reports → Final report checklist | `assessments.rs`, `assessments_store.rs`, report UI | Client/period/calculator/source decisions/account assignments/completeness/scenario gates; original snapshot and review flags retained |
| Reports → Print & presentation | `assessment_pdf.rs`, branding snapshots and native printing | Embedded/offline assets, exact totals, private internal evidence excluded from client plans, page margins and Windows print acceptance |
| Clients → History & records | Add bounded cross-module timeline and reviewed note amendments beside note storage | Authorship, original notes retained, reason/audit, scope/pagination, stale edits, restart and encrypted recovery |
| Documents → Statement accounts (working) | `StatementAccounts.tsx`, `statement_accounts_store`, schema 19 | Scoped standalone create/rename/archive/reactivate, revision/audit/recovery checks; original source/report associations preserved; no destructive deletion |
| Settings → Branding | Versioned organization contact/logo store and immutable report captures | Validated bounded local logo/contact fields, atomic scoped save, source identity captured on record, old report bytes and restore preserved |
| Settings → Counselors & roles | Auth/session/storage ownership and role model before UI CRUD | Explicit account lifecycle and permissions for reads/writes/approvals/reports/admin, fail-closed access, isolation and recovery drills |
| Settings → Security | Existing password/key/session model | Password rekey/rollback/recovery, inactivity settings, no password disclosure and independent security acceptance |
| Settings → Backup & recovery | `backup.rs`; current verified recovery flow already works | Scheduled backups/retained-copy selection need bounded local scheduling, failures, durable restore and clean Windows offline drills |
| Settings → Data management | Authorized client export/retention/deletion services | Reviewable scope/dependencies, encryption/disclosure, audit, no cross-client data loss, cancellation and recovery tests |
| Settings → Licensing | Offline license/update architecture and distribution metadata | Offline grace/expiry rules, signed updates/installers, third-party license inventory, support/legal review; no certification claims |

The unprovided original workbook remains an unverified reference. Remaining
release work includes clean Windows 11 offline install/upgrade/uninstall and full
installed workflows, realistic volume tests, accessibility/usability, signing,
security/privacy review, documentation and commercial/legal acceptance. Page
structure does not complete these requirements.

## Next implementation priority

Implement **firm/report branding** next: authenticated organization display-name
editing and explicit saved report branding, preserving original report formats,
bytes and provenance. Keep commercial licensing and multi-counselor permissions
planned until their own implementation and verification. Account institution
metadata, additional account/document types and planning mappings are recorded in
[statement-account-management.md](statement-account-management.md).
