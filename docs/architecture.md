# Architecture and development

## Supported target and first milestone

Windows 11 x64 is the first distribution target. Tauri 2 provides the native desktop host, React and TypeScript provide the interface, Rust owns authentication and persistence, and SQLCipher encrypts the local SQLite database. Development builds on Linux help validate code and local workflows; Windows installation and offline runtime behavior require a Windows machine.

The first milestone is intentionally limited to a working local counselor-and-client workflow. The complete specification is in [product-specification.md](product-specification.md). The unavailable workbook is a future reference to inspect only with authorized access; it is not a runtime dependency.

## Boundaries

| Layer | Responsibility in the first milestone |
| --- | --- |
| React interface | Local setup, sign-in, navigation, client forms, notes, clear validation and error states |
| Tauri command boundary | Narrow native commands, serializable request/response types, access to managed application state |
| Rust application core | Input validation, authenticated-session checks, organization ownership, lifecycle transitions |
| Persistence | Versioned schema, SQLCipher access, transactional writes, parameterized queries, audit records |
| Authentication | Password-based key derivation, opening the encrypted database, session lock/unlock |

The first financial module, `budget.rs`, owns exact cent arithmetic, normalized monthly budget totals, and missing/estimated planning flags. Budgets are scoped to the authenticated client in encrypted schema 3, saved atomically with audit events, and use optimistic revisions to prevent stale overwrites. `debt.rs` adds fixed-monthly APR repayment schedules, snowball/avalanche comparison, payment events, and budget-capacity review. Debt inputs are stored in encrypted schema 4 with the same scope/revision/audit safeguards; projections are recomputed and do not alter historical transactions. `savings.rs` adds liquid-pool reserve accounting, necessary-expense emergency coverage, dated sinking-fund goals, capped contribution forecasts, and budget/debt funding checks. Inputs are stored in schema 5 with the same authenticated revision/audit protections. Remaining financial calculations, document processing, and reporting belong in separate Rust modules with independent tests. React components must not become the authoritative calculation engine. Money uses exact integer cents and documented rounding; future approval workflows must separate confirmed facts from estimates and rejected data.

```mermaid
flowchart LR
    UI[Locally bundled React interface] --> IPC[Tauri commands]
    IPC --> Core[Rust application core]
    Core --> Auth[Local authentication and session]
    Core --> DB[Encrypted SQLCipher database]
    Core -. Future milestones .-> Calc[Decimal financial engine]
    Core -. Future milestones .-> Docs[Local document processing]
    Core -. Future milestones .-> Reports[Reports and immutable snapshots]
```

There is no required cloud service, spreadsheet integration, or shared network database. Each installation owns its local data directory. Development dependency downloads and Windows installer build downloads are separate from the installed application's essential offline workflow.

## Data model and ownership

Organization, counselor, client, note, and audit records use internal identifiers. The first installation creates one organization and counselor; this is not a completed multi-user permission system. Queries must derive the organization from the authenticated session and constrain records by that organization in Rust. Interface filtering alone is insufficient.

Client lifecycle states follow the specification: New Intake, Information Collection, Financial Review, Planning, Action Plan, Follow-Up, and Archived. Notes retain their author and creation time and are appended rather than silently overwritten. Audit entries record material operations without storing passwords, keys, or complete sensitive record contents.

The encrypted database stores the schema version. Future migrations must be tested against older databases and avoid leaving partially migrated state. Future transaction, financial-account, document, scenario, and report tables must retain client and organization ownership. Approved historical facts and hypothetical planning scenarios must remain distinct.

## Development checks

Run commands from the repository directory:

```sh
npm ci
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

The build type-checks and bundles the interface. Frontend tests exercise meaningful interactions. Rust tests exercise encryption, local authentication, persistence, validation, and record isolation. [Development status](development-status.md) distinguishes checks executed in this environment from tests that remain pending.

For native development on Ubuntu/Debian, Tauri 2 ordinarily requires a C/C++ build toolchain, `pkg-config`, `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`, and OpenSSL development support. SQLCipher/OpenSSL source builds may need additional compiler prerequisites. Follow [cloud-development.md](cloud-development.md) for the scripts that install and activate these dependencies in the actual managed cloud environment.

Cloud tasks already use an isolated environment: reuse the checkout under `/workspace/eisenbart-co-desktop` and do not create a Git worktree unless requested. Use frozen dependency installation (`npm ci`) and locked Rust builds. Keep dependency caches and generated build outputs separate from committed source. Startup processes must be relaunched in a new cloud task; retained build files do not mean the desktop process is still running.

## Windows packaging

The Windows workflow builds an NSIS installer with Tauri's `offlineInstaller` WebView2 mode. This is intended to include the WebView2 runtime installer so the resulting package can install without fetching that component during installation. Building it requires network access to obtain dependencies and the runtime redistributable.

The packaging configuration must be exercised on Windows. Merely committing the workflow does not produce an executable, verify the NSIS installer, demonstrate offline installation, or provide a signing certificate. Follow [windows.md](windows.md) for the build and release checks.
