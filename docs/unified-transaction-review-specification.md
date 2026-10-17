Forespan — Unified Transaction Review and Financial Document Inspection
Primary Objective
Enhance Forespan's existing financial document intake and transaction review capabilities to create a clean, reliable, professional interface where financial counselors can examine original client documents, review extracted financial data, analyze transactions across multiple accounts, and identify meaningful financial patterns.
IMPORTANT: Codex is currently working on financial document recognition, secure PDF validation, transaction extraction, account classification, and statement period identification.
Before beginning implementation, inspect the existing codebase and identify which features are already implemented, partially implemented, or currently being developed.
Do not duplicate existing systems, create competing components, or unnecessarily rewrite functioning code.
The objective is to extend and integrate the existing application rather than create a second document-processing or transaction-management system.
This work should be completed before declaring the next full test version ready.

Phase 1 — Inspect Existing Implementation and Prevent Redundancy
Before changing any application code:
Inspect the application's existing architecture, database schema, transaction models, document intake pipeline, and financial review interfaces.
Identify existing document viewers, transaction tables, categorization systems, review workflows, analytics, and reconciliation features.
Inspect automated tests and available documentation.
Check for uncommitted changes or development work that may overlap with the current assignment.
Determine which requested features can reuse existing functions and components.
Identify incomplete integrations, missing functionality, and genuine defects.
Create an implementation checklist categorizing each requirement as:
Already implemented and functioning.
Implemented but needing improvement.
Partially implemented.
Not implemented.
Blocked by other unfinished work.
Do not rebuild features that already work correctly.
If the earlier document extraction work is still in progress, avoid modifying overlapping files or introducing incompatible schemas. Use a separate worktree or branch where appropriate, or document the dependency and proceed with independent work.
Preserve compatibility with the existing client, debt, savings, household, and financial analysis systems.
Proceed through the following phases in order.

Phase 2 — Unified Transaction History Across All Accounts
Create or enhance a centralized transaction history that combines financial activity from every account belonging to the selected client.
A. Combined chronological transaction history
The counselor must be able to see transactions from all of a client's financial accounts in one unified table.
Supported account types should include:
Checking accounts.
Savings accounts.
Credit cards.
Money market accounts.
Personal lines of credit.
Loans and mortgages where transaction-level records exist.
Other supported financial accounts.
All transactions must appear in chronological order regardless of the financial institution, account type, or uploaded source document.
For example:
Date
Description
Account
Type
Amount
10/15/26
Grocery Store
Chase Checking
Debit purchase
-$85.40
10/15/26
Gas Station
Discover Credit
Credit card purchase
-$42.00
10/15/26
Payroll
Wells Fargo Checking
Income
+$2,100.00
10/14/26
Online Shopping
Chase Credit
Credit card purchase
-$69.99
10/14/26
Savings Transfer
Wells Fargo Savings
Transfer
+$200.00

This table is an illustrative example, not real client data.
Transactions occurring on the same date must appear together, even when they originate from different accounts or institutions.
Do not separate them into account-specific sections when the combined view is selected.
Default to most recent transactions first, with an option for oldest first.
Maintain deterministic ordering for transactions sharing the same date. Use transaction times where reliably available, otherwise use stable secondary ordering without inventing times.
Display transaction date and posting date separately when both are available. Clearly indicate which date controls sorting.
B. Required transaction information
Where available, preserve and display:
Transaction date.
Posting date.
Original description.
Cleaned merchant or transaction description.
Financial institution.
Account name.
Account type.
Masked account identifier.
Transaction amount and currency.
Debit or credit direction.
Transaction classification.
Spending category.
Source document.
Review and reconciliation status.
Extraction confidence or uncertainty indicators.
Unique internal transaction identifier.
Clearly distinguish between transaction direction, payment method, and account type.
For example, a credit card purchase is different from a credit deposited into checking.
Maintain consistent amount conventions across the application without losing the direction and meaning of the original transaction.
C. Supported transaction classifications
Distinguish, where possible, among:
Debit card purchases.
Credit card purchases.
Checks.
ACH debits and credits.
Direct deposits.
Cash withdrawals.
Cash deposits.
Internal transfers.
External transfers.
Credit card payments.
Loan payments.
Interest earned.
Interest charged.
Financial institution fees.
Refunds.
Reversals.
Other deposits and withdrawals.
Do not force ambiguous transactions into incorrect classifications. Use an unclassified or needs-review state when necessary.
Allow counselors to correct classifications without modifying the original source record.

Phase 3 — Transaction Filtering, Sorting, and Search
The unified transaction history must offer flexible filtering without becoming visually cluttered.
Required filters
Provide the ability to filter by:
All accounts.
Checking accounts only.
Savings accounts only.
Credit card accounts only.
Loan and credit line accounts.
Specific financial institution.
Individual account.
Date range.
Transaction type.
Spending category.
Debit or credit direction.
Amount range.
Review status.
Reconciliation status.
Include transaction search by merchant, description, and other useful identifiers.
Allow multiple filters to be combined.
Examples:
All credit card purchases during September.
Grocery expenses across all accounts.
Checking account withdrawals exceeding $200.
All interest charges across credit cards.
Transactions from one particular financial institution.
All transactions requiring manual review.
Display the active filters clearly.
Provide an easy way to reset all filters.
Changing filters must not alter or delete underlying financial information.
Maintain a consistent, reusable filtering implementation rather than creating separate transaction tables or database systems for different account types.
Use efficient queries and pagination or virtualization where needed to handle large transaction histories.

Phase 4 — Original Financial Document Viewer
I want counselors to be able to inspect the financial documents uploaded by a client while reviewing the information extracted from them.
However, I do not want the interface to become overcrowded or difficult to navigate.
Implement a side-by-side document viewer only if it can be integrated cleanly into the existing interface.
Preferred interface
Provide an optional document inspection mode.
In this mode:
Left panel: Original uploaded financial document
Embedded PDF viewer.
Page navigation.
Zoom controls.
Document name.
Financial institution.
Statement period.
Account identification using masked numbers.
Right panel: Extracted financial information
Account and statement information.
Extracted transactions.
Transaction classifications.
Amounts and dates.
Review status.
Editable fields.
Validation warnings.
Approval controls where supported by the existing review workflow.
Allow the counselor to resize the panels when practical.
Provide a clear control to close the document panel and return to the full-width transaction history.
The normal transaction history should remain the primary working interface.
Do not permanently allocate half the screen to a document viewer.
Document-to-transaction linking
When reviewing a transaction, allow the counselor to open its source document.
Ideally, selecting a transaction should navigate to the corresponding page and highlight the relevant source information.
Only implement precise highlighting where the extraction pipeline provides reliable source coordinates.
If coordinates are unavailable, navigate to the relevant page where known. Otherwise, open the document without falsely suggesting an exact source location.
Never fabricate document page numbers or source coordinates.
Document viewing security
Do not weaken the existing secure upload protections.
Render PDFs using a secure, maintained viewer that does not execute embedded PDF scripts or attachments.
Preserve local-first functionality, enforce client-level access boundaries, and avoid sending confidential documents to external services without an explicitly configured and authorized integration.
If the existing application architecture cannot support a safe, clean embedded viewer, provide a straightforward alternative such as an internal modal document inspection window.
Prioritize clarity, security, and usability over visual complexity.

Phase 5 — Editable Transaction Review and Approval
Extend the existing transaction review system rather than implementing a competing workflow.
Where already implemented, reuse the existing approval controls and review statuses.
Counselors should be able to:
Review extracted financial information.
Edit transaction descriptions.
Correct dates and amounts.
Change account classification where appropriate.
Assign or change spending categories.
Correct transaction classifications.
Approve valid extracted transactions.
Reject invalid extracted records.
Flag transactions requiring clarification.
Review the original source document before approval.
Reverse an approval when a subsequent correction becomes necessary.
Maintain an audit trail distinguishing:
Original extracted value.
Corrected value.
Counselor's action.
Time of correction.
Source document.
Current review status.
Avoid destructive edits to source evidence.
Use existing approval and reconciliation systems wherever possible.
Only approved and appropriately reconciled financial information should influence finalized financial assessments, unless another existing workflow deliberately allows clearly labeled provisional data.

Phase 6 — Payment Method and Financial Behavior Analysis
Using accurately classified, eligible transactions, add a focused analysis showing how clients use their different financial accounts.
A. Credit versus checking purchases
For a selected month or date range, calculate:
Total purchase value made through credit cards.
Total purchase value made through checking-linked debit cards.
Percentage of purchase spending made through credit cards.
Percentage of purchase spending made through checking-linked debit cards.
Number of purchases made through each method.
Average purchase amount for each payment method.
For example:
Payment method
Purchase amount
Share
Credit cards
$1,850
61.7%
Checking/debit
$1,150
38.3%
Total
$3,000
100%

These figures should represent actual purchase spending, not credit card repayments, transfers, or duplicate entries.
The analysis should identify whether a client tends to favor credit cards, debit cards, or a relatively balanced combination.
B. Additional observations
Where reliable data exists, identify:
Frequent credit card usage.
Large spending concentrations by category.
Recurring subscriptions.
Repeated overdraft or insufficient-funds fees.
Recurring interest charges.
High-frequency discretionary purchases.
Patterns of cash withdrawals.
Shifts in payment method usage between months.
Changes in purchase frequency or average purchase size.
Present these as financial observations rather than definitive judgments about a client's character or financial responsibility.
Credit card usage alone must not be classified as financially unhealthy.
Do not assume that a purchase was unnecessary simply because it occurred at a restaurant, entertainment business, or retailer.
C. Simple financial insights interface
Display a small, clearly organized overview, preferably integrated into the existing review or analysis interface.
Potential sections:
Spending by payment method.
Spending by account type.
Recurring spending.
Noteworthy transaction patterns.
Use existing charting libraries and design components wherever practical.
Avoid large collections of unnecessary charts or decorative dashboards.
Prioritize financial usefulness.
D. Measurement limitations
If classification coverage is incomplete, display the amount or percentage of spending that could not be classified.
Clearly distinguish observed transaction behavior from estimated conclusions.
Do not display misleading percentages based on incomplete data without an appropriate warning.

Phase 7 — Financial Accuracy and Reconciliation
Accurate financial accounting is essential.
Prevent double-counting
Handle the following scenarios correctly:
Credit card purchase and subsequent payment
A $100 credit card purchase followed by a $100 payment from checking must not be counted as $200 of consumer spending.
The purchase represents spending.
The checking payment represents a debt payment and corresponding movement of funds, not another purchase.
Transfers between owned accounts
Moving $500 from checking to savings must not increase earned income or create $500 of new expenses.
Overlapping statements
Importing two statements covering overlapping transaction dates must not automatically duplicate previously imported transactions.
Refunds
Refunds should be linked to the correct financial treatment and should not automatically be counted as new earned income.
Loan payments
Where enough detail is available, distinguish principal reduction, interest, and fees. Do not manufacture missing payment allocations.
Reconciliation requirements
Preserve statement opening and closing dates.
Preserve beginning and ending balances where available.
Check statement arithmetic using reliable transaction and balance conventions.
Identify gaps and overlaps in statement coverage.
Flag unresolved discrepancies.
Distinguish suspected duplicates from confirmed duplicates.
Preserve transaction provenance.
Avoid mixing provisional and finalized financial totals.
Transactions from multiple imported statements must remain connected to their actual source documents.
Use decimal-safe monetary calculations rather than binary floating-point arithmetic for financial totals.
