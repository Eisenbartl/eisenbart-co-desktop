# Eisenbart & Co. Financial Counseling Suite

A Windows desktop application for offline financial counseling, built in tested stages. The first milestone establishes the desktop shell, local counselor sign-in, encrypted local storage, and client records. Financial planning, document processing, and reports follow after the foundation is verified.

**Development build: use fictional data only.** Recovery, encrypted backups, Windows installation testing, and independent security review are required before real client use or commercial distribution. Losing the counselor password can make the current local database permanently inaccessible.

## Project status

The source specification is preserved in [docs/product-specification.md](docs/product-specification.md). [Development status](docs/development-status.md) records the checks actually executed and the milestone checkpoint. Configuration for a Windows installer is not proof that an installer has been built or tested.

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

The application does not yet provide financial calculations, statement imports, transaction approvals, charts of financial results, PDF reports, backup/restore, password recovery, or commercial licensing. These are tracked in [the roadmap](docs/roadmap.md); the first screen does not present unfinished financial tools as usable features.

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

- [Architecture and development](docs/architecture.md)
- [Security, data storage, and recovery limitations](docs/security.md)
- [Windows installation and build instructions](docs/windows.md)
- [Managed cloud setup and desktop validation](docs/cloud-development.md)
- [Staged roadmap and release acceptance criteria](docs/roadmap.md)
- [Initial dependency license inventory](docs/licenses.md)
- [Executed checks and current limitations](docs/development-status.md)
- [Complete product specification](docs/product-specification.md)

Each stage should end with passing relevant checks, updated status documentation, and a Git commit. Preserve the last working checkpoint before starting the next feature group.
