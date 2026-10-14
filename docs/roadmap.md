# Staged development and acceptance

The complete product specification is [product-specification.md](product-specification.md). [Development status](development-status.md) records the verified checkpoint. Each stage needs working behavior, meaningful tests, updated documentation, and a Git commit before the next feature group.

## Current foundation milestone

Stage 1 establishes the repository, architecture, and source specification. The prior Google Sheets workbook was unavailable; its existing features and formulas remain unverified. Stage 2 builds initial organization/counselor setup, local authentication, an encrypted database, client profiles and lifecycle, notes, and lock/unlock in the native desktop application.

Foundation acceptance requires:

- Build and launch the native Tauri application, not only the React page.
- Create a fictional organization/counselor and reject incorrect unlock credentials.
- Create and edit multiple fictional clients without carrying data between selections.
- Persist client profiles and append-only notes across application restarts.
- Enforce organization ownership in Rust and record material actions in an audit log.
- Exercise encrypted persistence, validation, migration initialization, and client/organization isolation tests.
- Produce the Windows installer on Windows and test installation and the local workflow while disconnected before calling Windows delivery complete.

Items above are acceptance checks, not blanket claims of completion. Check the recorded evidence for passed, failed, and unrun outcomes.

## Next priorities

| Stage | Deliverable | Required validation |
| --- | --- | --- |
| Foundation hardening | Complete Windows installation verification, encrypted backup/restore, password policy, corruption and migration handling | Clean-machine/offline installation, simulated failure and restore, wrong-password and isolation tests |
| 3: Financial core | Decimal-safe budgeting, income normalization, debt schedules, savings targets, cash-flow forecasting, scenarios | Independent expected amounts, rounding rules, edge cases, identical snowball/avalanche assumptions, no changes to historical facts |
| 4: Document processing | Local CSV and selected text-PDF import, provenance, reconciliation, review states and approval controls | Synthetic statements, malformed-input limits, duplicates, transfers, interrupted imports, approval reversals and rejected-data exclusion |
| 5: Dashboard and reports | Client-specific summaries, locally bundled charts, counselor report, PDF action plan, immutable finalized snapshots | Approved-data totals, incomplete-data labels, client isolation, reproducible report snapshots, offline PDF export |
| 6: Security and commercial architecture | Multiple accounts/roles, organization branding, export/deletion, completed audit and recovery safeguards, dependency review | Explicit permission tests, restore drills, independent security review, license inventory and unresolved legal matters |
| 7: Production packaging | Signed Windows installer, offline runtime, user/admin/recovery guides, demonstration dataset | Reproducible Windows artifact, clean installation, upgrade/uninstall behavior, offline checks, accurate third-party notices |
| 8: Final verification | Stable offline release candidate | Full installed workflows, usability/accessibility, all release-blocking tests, documented limitations |

Security prerequisites are pulled forward whenever real financial data would otherwise be at risk; the stage numbering is not permission to defer encryption or recovery until after production use. The Windows development package now passes CI installation, native-window launch, and uninstall checks. Clean Windows 11 offline acceptance and recoverability remain open. Stage 3's first slice now provides exact money values, documented income-frequency normalization, and persistent client budgets with independently checked totals. The debt checkpoint adds exact fixed-monthly APR schedules and comparable snowball/avalanche plans with extra payments and payment changes. Daily/variable/promotional-rate models remain unimplemented. The savings checkpoint now implements emergency coverage and sinking-fund goals with contribution scenarios and budget/debt allocation review. The dated cash-flow checkpoint implements per-account balances, entered paycheck/bill schedules, paired transfers, calendar/ledger views, and non-writing date previews. The recovery checkpoint adds encrypted authenticated backups, verified staged restore, retained previous files, and tested failure behavior. Clean Windows offline recovery and independent security review remain open. The first Stage 4 checkpoint implements local CSV statement import, provenance, duplicate/statement checks, explicit review, approved-only totals, refunds, and paired transfers. These historical facts remain separate from planning estimates. The next checkpoint implements selected selectable-text PDF extraction in a supervised child process, page/line provenance, and mandatory individual review. The next tested checkpoint adds approved-only merchant grouping, conservative recurring-payment suggestions, explicit counselor decisions, stale-evidence review and encrypted captured history. Statement running-balance reconciliation now adds declared order, independent source/approved comparisons and scoped immutable setting history. The first dashboard slice now shows approved-data periods, category/monthly/cash-flow charts and completeness indicators. Next: approved expense classifications and dated verified balance facts for remaining dashboard metrics; scanned OCR and broader institution adapters remain later checkpoints. Windows installer packaging is now a manual opt-in; normal pushes run checks without creating downloads. Use fictional fixtures while recovery safeguards and Windows acceptance remain pending.

## Full-product release criteria

The eventual first stable release must install on Windows 11 x64 and work offline; securely persist multiple clients; calculate budgets, debt, savings, and cash flows accurately; import supported documents; enforce review and approval; generate a professional PDF action plan; back up and restore securely; preserve client and firm isolation; avoid transmitting sensitive information; and provide tested recovery and appropriate financial-planning disclosures.

These full-product requirements are not satisfied by the foundation milestone. Independent security review, usability testing, commercial dependency/license review, installer signing, support/update procedures, and legal review are additional requirements before sale. No certification, legal compliance conclusion, or production-readiness claim is made.
