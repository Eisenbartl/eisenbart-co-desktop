# Remaining original requirements after 0.19.0

The application has an encrypted offline desktop foundation, isolated clients and
notes, exact entered budgets/debt/savings/cash-flow plans, encrypted recovery,
CSV/selectable-text PDF review, recurring-payment review, balance reconciliation,
approved-data dashboards, explicit classifications/dated inventories, and saved
immutable internal counselor assessments with optional captured planning scenarios, entered terms/milestones and offline printable PDF export. Immutable client action plans now record reviewed allocations, three to five priorities and follow-up, with client PDF export. Internal reports now optionally capture selected session notes and latest recurring-review evidence with original provenance. Optional read-only zero-based/50-30-20/custom budget references now add exact targets, minimum-debt classification and unknown-data safeguards. Temporary combined previews now compare entered budget, debt, savings and explicit dated changes without writing plans. The full master specification in
[product-specification.md](product-specification.md) remains the governing scope.
This is a development checkpoint, not a commercial release. The four new desktop
comparison workflows passed, but the complete native regression stopped on an
older report-panel click when WebDriver disconnected. Finish that verification
after diagnosing the transport interruption before the next feature checkpoint;
see [development-status.md](development-status.md).

| Work area | Main remaining work |
|---|---|
| Reports and client plans | Remaining financial models/scenario comparisons in counselor reports; client-plan logo/contact branding and complete-basis final validation; direct application printing; savings-goal progress and final report validation |
| Financial models — next | Persistent combined scenario selection/report capture, further proposal controls (extra payments/new expenses/goals/temporary shocks); advanced daily/variable/promotional debt terms, delayed transfer settlement, finer account reserve allocation, and meaningful gross-income underwriting metrics where verified data exists |
| Documents/review | Scanned PDF/OCR, institution/card/loan/paystub adapters, broader supported input formats, category suggestions and document/account workflow refinements with provenance/review safeguards |
| Client management/workflow | Full specified household/employment/financial intake fields, reminders/follow-up tracking, and remaining account/document/action-history views |
| Security/privacy | Configurable lock/password safeguards, multiple counselor accounts/roles and organization permissions, export/deletion, broader audit/recovery options, independent security/dependency/license review and privacy acceptance |
| Commercial customization/support | Organization branding/logo/contact details, licensing architecture, commercial documentation/disclosures/legal review and update/support procedures |
| Windows release/final verification | Signed installer, clean Windows 11 offline install and complete workflows, upgrades/uninstall, recovery drills, realistic data-volume tests, usability/accessibility and final release acceptance |

The original workbook/legacy parity remains unverified without the authorized
reference. Stages 3–8 of the original specification are partly open and need
several smaller checkpoints; a stage count is not a reliable percentage or time
estimate. No new installer is requested while feature development continues.
