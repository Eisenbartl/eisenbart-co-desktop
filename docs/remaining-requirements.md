# Remaining original requirements after 0.38.0 source

The [original product specification](product-specification.md) governs the full
application. This is a tested development checkpoint, not a commercial release.
The twelve main sections and their future areas are present, but planned pages
still identify functionality that is unavailable.

## Working foundations to build on

- Encrypted offline storage, local setup/authentication, isolated clients and
  append-only notes, client intake, reminders and the practice follow-up overview.
- Exact budget, debt, savings and dated cash-flow planning; temporary comparisons
  and immutable saved scenarios with explicit assumptions and version checks.
- Bank CSV/selectable-text PDF review, provenance, recurring-payment decisions,
  statement reconciliation, approved-data dashboards and verified financial facts.
- Document collection and bank statement-account CRUD/archive; firm name, logo,
  contact details and disclosures captured in historical reports and client plans.
- Saved internal assessments and client action plans with offline PDF export;
  recorded bank completeness reviews and bounded bank-only final approvals.
- Signed card CSV review, durable card accounts, reviewed statement assignments,
  explicit repayment links, provisional bank/card calculations and format-8
  internal report/PDF supplements. Payments do not become another expense.
- Recorded card completeness/applicability, source-bound necessary/discretionary
  roles and distinct existing-debt mappings, captured in internal reports/PDFs.
- Versioned card-inclusive final counselor approval, exact combined spending/cash
  figures and preserved original approvals/PDFs within its supported scope.
- Searchable client-wide bank/card import/decision history with stable pages,
  original/corrected provenance and exact transfer/repayment decision navigation.
- Encrypted backup/recovery and bounded integrity validation of historical records.

Do not repeat these foundations. See [development-status.md](development-status.md)
for the checks actually completed, and
[prototype-development-map.md](prototype-development-map.md) for implementation
entry points and acceptance criteria.

## Next development priority

Improve Reports → Print & presentation: a client/period financial summary first,
with approval audit details in an appendix and explicit Windows printing
acceptance. Preserve original financial capture and historical exports.

Card-inclusive final approval now works for the [bounded v2 policy](card-inclusive-final-approval.md).
Unsupported other loan obligations, bank-refund role treatments, credits, early
opening/closure and split/cross-period repayments remain blocked until explicitly
modeled and independently checked. Preserve all historical report bytes and policy
receipts when extending these rules.

## Further original requirements

| Work area | Main remaining work |
|---|---|
| Financial review and reports | Broader obligation/card-policy support; further comparative measures; dated savings-progress charts; direct desktop printing; final full-product report acceptance. Existing branding and bank-only final validation already work. |
| Financial models | Savings funding/drawdown, paycheck allocation, proposed new goals and account-targeted extra payments, advanced daily/variable/promotional debt terms and fees, delayed transfer settlement and finer account reserves. Verified gross-income metrics only where their basis exists. |
| Documents and transactions | Card PDF, loan/paystub and broader institution adapters, scanned PDF/OCR, local category suggestions/reusable rules, all with provenance and explicit review. Transaction activity history is implemented. No universal parser compatibility is claimed. |
| Client workflow and administration | Configurable time zones, remaining workflow refinements and realistic counselor usability/accessibility acceptance. Client intake, reminders, follow-up overview and account CRUD already exist. |
| Security/privacy | Configurable password/lock policies, multiple counselor accounts and roles, export/deletion controls and broader audit/recovery options; independent security, privacy and dependency/license reviews. |
| Commercial product | Offline licensing, commercial documentation/disclosures/legal review, signing and update/support procedures. Basic organization presentation is already implemented. |
| Windows release and final verification | New installer only when requested; clean Windows 11 offline installation and complete workflows, upgrades/uninstall, recovery drills, realistic data volumes and release acceptance. Source checks and Linux native tests do not establish installed-Windows acceptance. |

The original workbook and legacy parity remain unverified without the authorized
reference. Original stages 3–8 are partly open and require smaller checkpoints;
a stage count is not a reliable percentage or time estimate. The previous Windows
preview is 0.23.0; see the [Windows guide](windows.md). No newer installer is being
created without the user's request.
