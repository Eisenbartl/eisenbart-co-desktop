# Remaining original requirements after 0.35.0 source

The application has an encrypted offline desktop foundation, isolated clients and
notes, exact entered budgets/debt/savings/cash-flow plans, encrypted recovery,
CSV/selectable-text PDF review, recurring-payment review, balance reconciliation,
approved-data dashboards, explicit classifications/dated inventories, and saved
immutable internal counselor assessments with optional captured planning scenarios, entered terms/milestones and offline printable PDF export. Immutable client action plans now record reviewed allocations, three to five priorities and follow-up, with client PDF export. Internal reports now optionally capture selected session notes and latest recurring-review evidence with original provenance. Optional read-only zero-based/50-30-20/custom budget references now add exact targets, minimum-debt classification and unknown-data safeguards. Temporary combined previews now compare entered budget, debt, savings and explicit dated changes without writing plans. The full master specification in
[product-specification.md](product-specification.md) remains the governing scope.
This is a development checkpoint, not a commercial release. Saved combined scenarios now retain immutable baselines/proposals and can be
explicitly selected for counselor reports. Proposals now also add extra debt payments and change existing savings-goal contributions, preserving original events, targets and reserves. New recurring-expense proposals recalculate shared affordability and can explicitly add estimated bills to a selected dated account. Temporary income reductions now retain bounded monthly funding/recovery checks and optional dated paycheck changes, with conditional continuing forecasts. The previously interrupted full native
workflow passed through direct WebKit transport before this feature stage; see
[development-status.md](development-status.md) for new-stage verification.


All twelve main sections and named future/admin areas are present. Statement-account
CRUD, document collection, firm/logo/contact branding, saved bank completeness
reviews and bounded bank-only final approvals have since been implemented.
Signed card CSV review and provisional bank/card supplements now work, including
durable card account/source links, explicit repayment matching and historical
report/PDF capture. Do not repeat those completed foundations. See
[development-status.md](development-status.md) for current verification and
[prototype-development-map.md](prototype-development-map.md) for entry points.
Planned pages still do not implement their future features.

Next implementation priority: card-inclusive household inventory and verified
balance/fact mappings, with a separately versioned completeness/final policy.
The current provisional supplement cannot authorize final card-inclusive reports.

| Work area | Main remaining work |
|---|---|
| Reports and client plans | Card-inclusive completeness/final approval and reviewed balance mappings; remaining comparative measures/models; direct application printing; savings-goal progress. Firm presentation and bank-only final validation already exist. |
| Financial models — next | Savings drawdown/funding-source simulation and period-specific client-plan allocations; new-goal and account-targeted extra payment controls; advanced daily/variable/promotional debt terms, delayed transfer settlement, finer account reserve allocation, and meaningful gross-income underwriting metrics where verified data exists |
| Documents/review | Scanned PDF/OCR, card PDF, institution/loan/paystub adapters, broader supported input formats, category suggestions and document/account workflow refinements with provenance/review safeguards |
| Client management/workflow | Configurable time zones and remaining account/document/action-history views; broader review/usability acceptance. Explicitly selected intake sections in immutable internal reports are implemented in 0.26.0 source. The practice-wide filtered appointment/follow-up overview is implemented in 0.25.0 source. Optional household/employment context and per-client dated reminders/completion/cancellation are implemented in 0.24.0 source. |
| Security/privacy | Configurable lock/password safeguards, multiple counselor accounts/roles and organization permissions, export/deletion, broader audit/recovery options, independent security/dependency/license review and privacy acceptance |
| Commercial customization/support | Further branding/usability refinements, licensing architecture, commercial documentation/disclosures/legal review and update/support procedures |
| Windows release/final verification | Signed installer, clean Windows 11 offline install and complete workflows, upgrades/uninstall, recovery drills, realistic data-volume tests, usability/accessibility and final release acceptance |

The original workbook/legacy parity remains unverified without the authorized
reference. Stages 3–8 of the original specification are partly open and need
several smaller checkpoints; a stage count is not a reliable percentage or time
estimate. A tested 0.23.0 Windows preview installer is available in the [Windows guide](windows.md); package checks and clean Windows 11 acceptance remain separate evidence.
