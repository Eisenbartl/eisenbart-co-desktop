# Development guidance

Read `docs/development-status.md`, `docs/architecture.md`, and the relevant part
of `docs/product-specification.md` before changing the application. Work in the
existing checkout; cloud tasks are already isolated and do not need a worktree.

- Build one manageable milestone at a time. Preserve the functioning foundation
  and commit tested milestones. Update the development status with what actually
  ran, remaining limitations, and the next priority.
- Use fictional clients and synthetic statements. Do not import actual client
  information without explicit authorization.
- Keep permissions, persistence, authentication, and financial calculations in
  Rust. React presents results and collects input. Never trust client/firm IDs
  supplied by the interface as authorization.
- Future money calculations must use exact decimal or integer minor units,
  documented rounding, and independently checked examples; do not use binary
  floating-point money calculations.
- Keep essential operation offline. Bundle UI assets locally and never add a
  cloud dependency, telemetry, or external fonts silently.
- Preserve password/key secrecy and encryption/signature/TLS verification.
  Linux cloud smoke-test sandbox exceptions must stay scoped to isolated,
  synthetic test processes and must not enter the shipped application.
- Use locked installs. In this managed cloud machine run
  `bash scripts/cloud-setup.sh`, then `source scripts/cloud-env.sh` in shells
  that need Cargo or native libraries.
- Run relevant meaningful checks: `npm run build`, `npm test`, and
  `cargo test --manifest-path src-tauri/Cargo.toml --locked`. For desktop changes,
  also build with `npm run tauri -- build --debug --no-bundle` and run the native
  smoke test documented in `docs/smoke-test.md`.
- Windows packaging/installation and commercial readiness need their own
  verification. A successful Linux build cannot establish those results.
- Do not weaken tests or call an untested installer or calculation complete.
  Keep development-build status until its release criteria are met.
