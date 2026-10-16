# Eisenbart & Co. Financial Counseling Suite

Version **0.48.0 source** adds [transaction-focused statement extraction](docs/pdf-import-review.md) for checking/savings and credit-card PDFs: fine-print exclusion, editable confident preselection, exact draft totals, and source-bound Necessary/Discretionary dispositions in Review & reconciliation. Current spending analysis updates automatically; original sources and decision history remain intact. **No new installer will be created or provided unless requested.**


Version **0.47.0 source** adds [client Spending Analysis](docs/spending-analysis.md): approved/categorized spending cards, category donut and discretionary bars, filters, supported opportunities and discussed Necessary/Discretionary Type changes. Saved changes refresh all affected results while preserving original imports and classification history. Import/review remains separate in the thirteen-section workspace. The [verified 0.47.0 Windows development installer](docs/windows.md) passed automated installation, launch and uninstall; its downloaded checksum and source/version instructions were verified.

Version **0.46.0 source** introduced the [client workspace redesign](docs/client-workspace-redesign.md): a directory-only Clients page, stable-ID client tabs, twelve focused sections, separate review subtabs and saved-plan summaries. Existing financial engines and security were retained. Discard choices use an explicit in-app dialog, including asynchronous native-confirm safeguards. These tools remain available in 0.47.0.

Version **0.45.0 source** adds [configurable inactivity locking](docs/inactivity-locking.md): a backend-enforced 1–60 minute workspace policy, encrypted history, stale-save protection and policy-preserving recovery. Existing workspaces default to 15 minutes. No installer was created.

Version **0.44.0 source** adds [workspace password changes and recovery safeguards](docs/workspace-password-change.md): current-password verification, a verified new encrypted generation, atomic activation and locking. Older backups and retained copies keep their original passwords. No installer was created.

Version **0.43.0 source** adds [client history and reviewed note corrections](docs/client-history.md): searchable dated records, explicit corrections with original versions retained, and current-note selection for new reports while old reports keep their exact evidence. No installer was created.

Version **0.42.0 source** adds [reusable local category rules](docs/local-category-rules.md): create/edit/archive/reactivate with immutable history, conflict checks and explicit transaction review. Saved applications retain the exact original rule version. No installer was created.

Version **0.41.0 source** adds [local category suggestions](docs/classification-assistance.md): current approved merchant evidence, unresolved conflicts, bounded search/pages and category-only unsaved review drafts. Save an explicit decision through the existing review controls; no automatic approval. No installer was created.

Version **0.40.0 source** adds [Open saved PDF to print](docs/windows-pdf-print-workflow.md): an explicit, consented request to open an unchanged counselor report export in the Windows PDF viewer. Choose Print in that viewer; the app does not submit jobs or confirm printing. Export lifetime and failures are explicit. No installer was created.

Version **0.39.0 source** adds [summary-first counselor PDFs](docs/report-presentation.md) with a complete evidence appendix and the original detailed option. Version **0.38.0 source** adds [searchable bank/card transaction history](docs/transaction-activity-history.md).

Version **0.37.0 source** adds [card-inclusive final approval](docs/card-inclusive-final-approval.md): complete supported bank/card evidence, explicit counselor review, exact combined spending versus bank cash movement, and immutable approvals/PDFs. Existing debt is counted once and older approvals retain their original policy. No installer was created.

Version **0.33.0 source** adds [final counselor report approval](docs/final-report-approval.md): required evidence gates, explicit professional decisions, immutable audited approvals and offline final PDFs. Later context changes preserve the original approval and flag it for review. No installer was created.

Version **0.32.0 source** adds [saved completeness reviews](docs/saved-completeness-reviews.md) with expected accounts, applicable dates, missing information, exceptions and encrypted version history. Reports can capture the exact reviewed version; later changes preserve the original report and flag it for review. Professional approval follows in 0.33.0 above. No installer was created.

Version **0.31.0 source** adds [financial completeness diagnostics and saved-report validation](docs/financial-completeness.md): declared account/date coverage, gaps/overlaps, unresolved source checks and historical PDF checklists. Finalization remains blocked pending recorded complete inventory and professional review. Versions 0.29–0.30 added [statement account management](docs/statement-account-management.md) and [firm/report branding](docs/firm-report-branding.md). These source stages are included in the current **0.47.0** preview.

Version **0.28.0 source** adds a [persistent document collection checklist](docs/document-collection.md), with request editing, explicit receipt, source links, archive/reactivation and version history. Version 0.27.0 added all 12 main application sections, focused client pages, settings/administration areas and labeled future-feature pages. The [prototype development map](docs/prototype-development-map.md) records CRUD coverage and the remaining implementation work. Version 0.26.0 added [selected intake context in immutable internal reports](docs/report-intake-context.md), with explicit section review, original-version validation and permanent review flags after selected details change. Client action-plan PDFs omit this internal context. Version 0.25.0 added a [dashboard appointment/follow-up overview](docs/follow-up-overview.md), with scoped counts, filters, pagination and direct navigation to the selected client item. Version 0.24.0 added optional client intake, dated appointments and follow-up tasks, overdue reminders, retained completion/cancellation outcomes, and encrypted version history. See [intake and follow-ups](docs/client-intake-followups.md). The **0.47.0** preview includes these intake and follow-up tools.

A Windows desktop application for offline financial counseling, built in tested stages. The current source also includes an approved-transaction dashboard with explicit periods and offline charts; complete reporting and release requirements remain in development. The first milestone establishes the desktop shell, local counselor sign-in, encrypted local storage, and client records. Version 0.23.0 adds bounded temporary income reductions, monthly funding deficits/recovery, explicitly selected dated paycheck adjustments and saved report capture. A tested Windows preview installer is available; see [download and installation instructions](docs/windows.md). Version 0.22.0 added new recurring-expense proposals with explicit dated bill selection, shared affordability/emergency-target recalculation, saved history and report PDF support. Version 0.21.0 added hypothetical extra debt payments and changes to existing savings-goal contributions, with exact combined comparisons, saved history and report PDF support. Version 0.20.0 added immutable saved combined scenarios, history/review flags and explicit counselor report selection with offline PDF export. Temporary combined comparisons remain available: budget amount, debt-payment, emergency-contribution and explicit dated-schedule proposals are recalculated side by side with saved baselines, without changing plans or approved facts. Optional budget methods include read-only zero-based, 50/30/20 and custom-percentage budget comparisons with exact targets, minimum-debt classification and unknown-data safeguards. Internal counselor reports include selected session notes and captured recurring-review evidence, preserving original source references and exact totals. Immutable client action plans include reviewed monthly allocations, three to five prioritized steps, follow-up and offline client PDF export. Named captured planning scenarios preserve saved assumptions, selected repayment outcomes and milestones independently of approved historical figures. Later plan changes flag historical context without rewriting it. Existing features include verified expense/payroll classifications, dated balance inventories, conditional net worth and clearly labeled coverage/payment estimates, plus exact statement running-balance comparisons, declared source order, separate original/approved checks, clear exceptions, and encrypted setting history. Existing features include CSV review, duplicate/statement checks, refunds/transfers, encrypted backup/restore, cash flow, savings, budgets, and debt plans. Scanned OCR, broader institution adapters, savings progress and commercial release acceptance remain in later checkpoints.

**Development build: use fictional data only.** Backup/recovery is now implemented and locally tested. Clean Windows 11 offline recovery acceptance and independent security review are required before real client use or commercial distribution. Losing the counselor password can make the current local database permanently inaccessible.

## Project status

The source specification is preserved in [docs/product-specification.md](docs/product-specification.md). [Development status](docs/development-status.md) records the checks actually executed and the milestone checkpoint. The 0.47.0 Windows development installer passed automated installation, native-window launch, and uninstall checks on Windows Server 2025. Full Windows 11 offline acceptance remains pending.

See the [Windows download guide](docs/windows.md) and [evaluation guide](docs/user-guide.md)
for the exact version, verified build and installer limitations. Older installer
links do not validate the current redesign. GitHub sign-in with repository access
may be required; extract the ZIP and read its README and checksum file before
running the unsigned development installer.

The original Google Sheets workbook and supporting documents were not available for inspection. No spreadsheet behavior or financial formula is treated as verified. No real client records are included.

## First desktop milestone

The foundation is scoped to:

- Initial organization and counselor setup, then local password sign-in.
- A React interface with working dashboard, client, and settings workflows.
- Client profiles, lifecycle status, and dated counselor notes.
- Rust commands that enforce organization ownership and store records in an encrypted SQLCipher database.
- Manual session locking and a configurable 1–60 minute inactivity limit (15-minute default).
- Persistence and isolation checks using fictional data.

One organization and one counselor are supported per local installation initially. The schema retains organization and counselor identifiers to support future accounts and permissions. Shared access to a SQLite file over a network is unsupported.

Client budgets now calculate income, expenses, savings/debt allocations, and signed surplus/deficit using exact cents. Debt plans now compare fixed monthly APR snowball/avalanche schedules, extra payments, and payment changes, with missing-term and capacity warnings. Savings now separates liquid/restricted/earmarked funds, calculates emergency targets and sinking-fund deadlines, and compares contributions against entered cash/debt commitments. The application does not yet provide daily/variable/promotional interest models, Scanned-document OCR, broader institution adapters, forgotten-password reset, or commercial licensing. These are tracked in [the roadmap](docs/roadmap.md); the first screen does not present unfinished financial tools as usable features.

## Run from source

For a Windows computer, follow [the Windows guide](docs/windows.md). For this managed Linux cloud environment, follow [the cloud development guide](docs/cloud-development.md) to install and activate the retained native tools first. Development needs Node.js 24, a current stable Rust toolchain, and Tauri's platform build prerequisites.

From the repository directory:

```sh
npm ci
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

`npm run tauri dev` launches the native desktop application. Opening the React development page in a browser alone does not exercise encrypted storage or native authentication.

On Ubuntu/Debian development hosts, Tauri also requires the platform packages listed in [the architecture guide](docs/architecture.md). The project targets Windows 11 x64 first; a Linux build is a development check, not a Windows release test.

## Documentation

- [Selectable-text PDF import and review](docs/pdf-import-review.md)
- [Final counselor report approval](docs/final-report-approval.md)
- [Offline assessment PDF export](docs/assessment-pdf-export.md)
- [Captured planning scenarios](docs/captured-planning-scenarios.md)
- [Client action plans](docs/client-action-plans.md)
- [Counselor report evidence](docs/counselor-report-evidence.md)
- [Budget method comparisons](docs/budget-method-comparisons.md)
- [CSV import and transaction review](docs/csv-import-review.md)
- [Merchant grouping and recurring payments](docs/recurring-payment-review.md)
- [Statement running-balance checks](docs/statement-balance-reconciliation.md)
- [Encrypted backup and recovery](docs/backup-recovery.md)
- [Savings calculation rules](docs/savings-calculations.md)
- [Debt repayment calculation rules](docs/debt-calculations.md)
- [Client budget calculation rules](docs/budget-calculations.md)
- [Architecture and development](docs/architecture.md)
- [Security, data storage, and recovery limitations](docs/security.md)
- [Windows installation and build instructions](docs/windows.md)
- [Windows evaluation and first-use guide](docs/user-guide.md)
- [Managed cloud setup and desktop validation](docs/cloud-development.md)
- [Staged roadmap and release acceptance criteria](docs/roadmap.md)
- [Initial dependency license inventory](docs/licenses.md)
- [Executed checks and current limitations](docs/development-status.md)
- [Complete product specification](docs/product-specification.md)

Each stage should end with passing relevant checks, updated status documentation, and a Git commit. Preserve the last working checkpoint before starting the next feature group.

Exact dated cash-flow assumptions and independently checked examples: [cash-flow calculations](docs/cash-flow-calculations.md).

Backup creation, restore, password consequences, and failure behavior: [backup and recovery guide](docs/backup-recovery.md).
