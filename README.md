# Eisenbart & Co. Financial Counseling Suite

A Windows desktop application for offline financial counseling, built in tested stages. The current source also includes an approved-transaction dashboard with explicit periods and offline charts; complete reports and client PDF plans remain in development. The first milestone establishes the desktop shell, local counselor sign-in, encrypted local storage, and client records. Version 0.15.0 source adds named captured planning scenarios to internal assessments and PDFs, preserving saved assumptions, selected repayment outcomes and milestones independently of approved historical figures. Later plan changes flag historical context without rewriting it. Existing features include verified expense/payroll classifications, dated balance inventories, conditional net worth and clearly labeled coverage/payment estimates, plus exact statement running-balance comparisons, declared source order, separate original/approved checks, clear exceptions, and encrypted setting history. Existing features include CSV review, duplicate/statement checks, refunds/transfers, encrypted backup/restore, cash flow, savings, budgets, and debt plans. Scanned OCR, institution adapters, savings progress, and complete scenario-backed reports follow in later checkpoints.

**Development build: use fictional data only.** Backup/recovery is now implemented and locally tested. Clean Windows 11 offline recovery acceptance and independent security review are required before real client use or commercial distribution. Losing the counselor password can make the current local database permanently inaccessible.

## Project status

The source specification is preserved in [docs/product-specification.md](docs/product-specification.md). [Development status](docs/development-status.md) records the checks actually executed and the milestone checkpoint. The Windows development installer was built and passed automated installation, native-window launch, and uninstall checks on Windows Server 2025. Full Windows 11 offline acceptance remains pending.

[Earlier 0.3.0 Windows installer (without savings, cash flow, backup/recovery, or CSV review)](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38485910521/artifacts/11859772481). The current source dashboard milestone has no new downloadable installer, as requested. GitHub sign-in with repository access may be required. Extract the ZIP, read `README.txt`, and run the installer. This build passed 25 frontend and 32 Windows backend tests, installation, native-window launch, uninstall, and isolated cleanup. The download expires October 27, 2026. Follow the [Windows guide](docs/windows.md) and [evaluation guide](docs/user-guide.md). Version 0.3.0 upgrades the database to schema 4; older versions cannot reopen it afterward.

The original Google Sheets workbook and supporting documents were not available for inspection. No spreadsheet behavior or financial formula is treated as verified. No real client records are included.

## First desktop milestone

The foundation is scoped to:

- Initial organization and counselor setup, then local password sign-in.
- A React interface with working dashboard, client, and settings workflows.
- Client profiles, lifecycle status, and dated counselor notes.
- Rust commands that enforce organization ownership and store records in an encrypted SQLCipher database.
- Manual session locking and locking after 15 minutes of inactivity.
- Persistence and isolation checks using fictional data.

One organization and one counselor are supported per local installation initially. The schema retains organization and counselor identifiers to support future accounts and permissions. Shared access to a SQLite file over a network is unsupported.

Client budgets now calculate income, expenses, savings/debt allocations, and signed surplus/deficit using exact cents. Debt plans now compare fixed monthly APR snowball/avalanche schedules, extra payments, and payment changes, with missing-term and capacity warnings. Savings now separates liquid/restricted/earmarked funds, calculates emergency targets and sinking-fund deadlines, and compares contributions against entered cash/debt commitments. The application does not yet provide daily/variable/promotional interest models, Scanned-document OCR, broader institution adapters, final client action plans, forgotten-password reset, or commercial licensing. These are tracked in [the roadmap](docs/roadmap.md); the first screen does not present unfinished financial tools as usable features.

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
- [Offline assessment PDF export](docs/assessment-pdf-export.md)
- [Captured planning scenarios](docs/captured-planning-scenarios.md)
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
