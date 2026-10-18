# Eisenbart & Co. — Household Management & Shared Financial Planning System

Implement a complete **Household Management and Shared Financial Planning system** within the existing Eisenbart & Co. desktop financial counseling application.

The purpose is to allow a counselor to link two or more individuals into a household, analyze their shared finances, establish customized expense-sharing arrangements, track payments and reimbursements, and integrate each person's household obligations into their individual financial planning tools.

The system must support married couples, unmarried partners, roommates, multigenerational families, and other household arrangements without assuming that household members share all their income, debts, assets, or financial responsibilities.

**Before implementation:** Inspect the existing application architecture, database, client management system, budget engine, debt planning, savings tools, cash-flow calendar, statement imports, transaction classification, and financial dashboards. Reuse and extend existing components, calculations, and data structures wherever practical. Implement the functionality directly in the working application rather than creating a disconnected prototype.

Preserve all existing client data, financial calculations, security protections, and functionality.

## 1. Household Creation and Client Linking

Create a dedicated household management system.

Allow the counselor to:

- Create, name, rename, and archive household workspaces.
- Link two or more existing client records to a household using stable identifiers.
- Add household members who are not full counseling clients, using limited information when appropriate.
- Assign descriptive relationships or roles, such as spouse, partner, roommate, dependent, or other.
- Add or remove members without destroying historical financial records.
- Specify household membership start and end dates.
- Support any practical number of household members.
- Maintain independent individual client records.
- Link household members without automatically combining their private financial information.

Allow a person to belong to more than one household when appropriate, while keeping financial obligations and records clearly separated.

Maintain appropriate permissions and documented authorization for accessing and sharing financial information.

## 2. Household Workspace and Navigation

Integrate household workspaces into the application's existing tabbed client navigation.

The top tab bar should allow simultaneous access to individual clients and households, for example:

**All Clients | John Doe | Sarah Smith | Doe Household | +**

Users must be able to switch between open workspaces without returning to the client directory.

Each household workspace should contain a collapsible left navigation menu with the following sections:

1. Household Overview
2. Members & Permissions
3. Shared Expenses
4. Expense Allocation Rules
5. Payments & Settlements
6. Household Budget
7. Household Cash Flow
8. Spending Analysis
9. Goals & Scenarios
10. Reports & History

Remember the last-opened section, applicable filters, and working state when switching tabs.

Follow the application's existing professional navy, white, slate, and muted gold design. Use compact summary cards, readable tables, well-designed charts, and expandable detail panels. Avoid the excessively long scrolling layouts previously used in individual client records.

## 3. Flexible Expense Allocation Engine

Build a reusable financial allocation engine capable of distributing shared expenses among any number of household members.

Support these allocation methods:

**A. Equal Split**

Divide the expense evenly among participating members. Examples include 50/50 for two people or approximately 33.33% each for three people.

**B. Custom Percentage Split**

Allow the counselor to specify different percentages for each participating member.

Example: Three roommates share $2,400 rent. John pays 25% ($600), while Sarah and Michael each pay 37.5% ($900).

**C. Fixed Dollar Contributions**

Assign specific monthly dollar obligations rather than percentages.

Example: One member contributes $500 toward rent while the remaining amount is distributed among other members.

**D. Income-Proportional Contributions**

Optionally calculate each person's share according to their authorized, known income. Allow the counselor to choose the appropriate income basis, such as monthly take-home income. Do not expose private income figures unnecessarily.

**E. Individual Responsibility**

Assign an entire expense to one household member, even when others benefit from it.

**F. Hybrid Allocation**

Support combinations of allocation methods. For example, one person pays a fixed $500 and the remaining expense is divided equally or proportionally among specified members.

Allow different allocation methods for different spending categories and individual bills. A household should not be restricted to one universal split.

Provide category-level default rules, individual expense overrides, and an option to exclude specific members from particular expenses.

Support recurring expenses, irregular expenses, and one-time household purchases.

## 4. Expense Allocation Editor

Create an intuitive interface for configuring household expenses and their allocation rules.

Each expense should support:

- Expense name and category.
- Expected or actual amount.
- Frequency and due date.
- Participating household members.
- Allocation method.
- Individual percentages or dollar contributions.
- Responsible payer or payment account.
- Reimbursement expectations.
- Effective start and end dates.
- Whether the expense is confirmed, estimated, or pending review.
- Counselor notes.

Display a live breakdown showing exactly how much each member owes.

Automatically recalculate member obligations whenever an expense or allocation rule changes.

Validate that allocated percentages total 100% and allocated dollar amounts equal the total expense. Use precise decimal or integer-cent calculations, with consistent rounding and deterministic distribution of leftover cents.

Prevent invalid allocations from being saved as finalized agreements. Clearly identify any unallocated or overallocated amounts.

Allow the counselor to preview proposed allocation changes before saving them.

## 5. Payment Responsibility vs. Actual Payments

**This distinction is essential to financial accuracy.**

Track separately:

1. What each household member is responsible for paying.
2. Who actually makes a payment.
3. Which account or payment source is used.
4. Who must reimburse another member.
5. Whether reimbursement has occurred.
6. What remains outstanding.

Example:

John pays the entire $2,400 rent to the landlord, but his agreed share is $600. Sarah and Michael each owe $900.

The system should record the landlord payment as $2,400 of household housing expenses, not $4,200 after reimbursements.

Sarah's and Michael's payments to John are internal settlements, not additional household spending.

Support partial payments, multiple payers, joint accounts, overpayments, underpayments, and reimbursements occurring on different dates.

Provide a household settlement ledger that calculates outstanding balances and suggests appropriate reimbursements.

Distinguish planned obligations, recorded payments, imported transactions, and verified completed settlements. Never treat an expected payment as completed without supporting confirmation.

## 6. Shared Bank Accounts and Transaction Reconciliation

Support households that pay expenses through individual bank accounts, joint accounts, or a combination.

Allow authorized imported transactions to be associated with household expenses without duplicating their original financial records.

A transaction should retain its original client/account ownership and source-document reference.

Provide matching and reconciliation workflows for:

- Rent and mortgage payments.
- Utility payments.
- Shared grocery purchases.
- Joint account contributions.
- Reimbursements between household members.
- Transfers between linked accounts.
- Refunds and reversals.
- Shared purchases made using one person's credit card.

Prevent transfers, reimbursements, and duplicate imported transactions from inflating household spending or income.

Distinguish original source data from counselor-entered planning assumptions and approved adjustments.

Do not modify or delete original imported statement transactions when household classifications or allocation rules change.

Use the application's existing transaction approval, classification, duplicate-detection, and reconciliation systems wherever possible.

## 7. Household Overview Dashboard

Create a visually polished household financial dashboard displaying:

- Total known monthly household income, when authorized.
- Total shared household expenses.
- Necessary vs. discretionary shared spending.
- Total obligations assigned to each member.
- Actual contributions and payments.
- Outstanding reimbursements.
- Upcoming household bills.
- Expected cash-flow shortfalls.
- Household savings goals.
- Missing information and unresolved financial issues.

Include charts showing how household expenses are divided among members and categories.

Provide a clear member contribution comparison showing each person's monthly responsibility, actual payments, reimbursements, and remaining obligations.

All displayed metrics must update when expense amounts, classifications, membership, or allocation rules change.

Label estimates, incomplete records, and unverified information appropriately. Do not present an incomplete household assessment as a complete financial picture.

## 8. Individual Budget Integration

Connect approved household expense allocations to the existing individual client budgeting system.

For example, if a household's rent is $2,400 but John's agreed share is $600, John's individual budget should recognize his $600 obligation rather than automatically assigning him the entire household rent.

Maintain a clear distinction between:

- Individual personal expenses.
- Individual shares of household expenses.
- Full payments temporarily made on behalf of the household.
- Expected reimbursements.
- Contributions to shared savings or joint accounts.

Do not double-count household obligations already represented in individual budgets.

If an existing budget conflicts with a newly calculated household allocation, flag the discrepancy and offer a controlled reconciliation process.

Never silently overwrite an approved or previously saved individual budget.

Record changes through the existing budget revision system and preserve the historical values.

## 9. Household Cash-Flow Planning

Extend the existing cash-flow calendar to support shared household obligations.

Include bill due dates, anticipated household contributions, paydays when authorized, joint account balances, reimbursement dates, and actual payment activity.

Identify situations where household spending is affordable overall but an individual member or payment account may experience a temporary shortfall.

For example, John may have to pay the full rent on the first of the month even though other members will reimburse him on the fifth.

Model that timing difference correctly.

Support irregular income, delayed reimbursements, changing payment schedules, insufficient cash balances, and missing payment information.

Clearly distinguish projected balances from verified balances.

## 10. Household Spending Analysis

Extend the existing Spending Analysis functionality to shared household finances.

Provide charts showing:

- Spending by category.
- Necessary versus discretionary spending.
- Shared expenses by household member.
- Monthly spending trends.
- Major recurring household costs.
- Potential opportunities for spending reductions.

Use appropriately reviewed and approved transaction information.

Allow counselors to adjust spending classifications after discussing transactions with clients, maintaining a history of all changes.

When a classification changes, recalculate the affected household and individual metrics, charts, totals, and percentages.

Avoid automatically labeling expenses as unnecessary. Use neutral terminology such as discretionary spending and opportunity areas.

## 11. Changing Household Arrangements Over Time

Household arrangements must support effective-dated revisions.

For example, one member may temporarily pay a smaller share of rent after losing a job, or roommates may renegotiate responsibilities when someone moves out.

Allow allocation changes to take effect on a specified date without rewriting historical obligations.

Support temporary arrangements, future scheduled changes, new household members, departing members, and expenses that begin or end during a month.

Preserve previous allocation agreements and calculate historical financial reports using the rules applicable during the relevant periods.

Clearly distinguish proposed future arrangements from approved current arrangements.

## 12. Household Goals and Scenario Planning

Integrate the household workspace with the existing savings, debt, and scenario-planning tools.

Support shared financial goals such as emergency savings, moving expenses, home repairs, holiday expenses, and other sinking funds.

Allow custom member contribution arrangements for shared goals.

Support hypothetical scenarios involving changes to rent, utilities, income, household membership, or expense allocations.

Show how a proposed change would affect each participating member's budget, available cash, and savings capacity.

Do not alter active financial plans until a proposed scenario is explicitly accepted and saved.

Keep personally owned debts and assets separate from shared obligations unless they have been explicitly designated as shared.

## 13. Privacy, Security, and Audit History

Preserve the application's offline-first architecture and existing local data protection mechanisms.

Do not introduce mandatory internet access, cloud synchronization, or third-party services.

Household membership must never automatically grant access to another person's private financial records.

Allow the counselor to specify which information may be included in household calculations and reports.

Where a member has not authorized disclosure of income, debt, account balances, or individual transactions, display only the permitted household contribution or aggregated information.

Maintain an audit history of:

- Household creation and membership changes.
- Expense allocation agreements and revisions.
- Counselor adjustments.
- Payment and reimbursement records.
- Spending classification changes.
- Financial reconciliations.
- Applicable authorization and consent records.

Preserve original source-document data and provide traceability from calculated household totals to underlying approved transactions and allocation rules.

## 14. Reliability and Financial Validation

Implement comprehensive automated tests for the allocation engine, payment ledger, household calculations, persistence, and integrations.

Include cases covering:

- Two-member households.
- Three or more members.
- Equal and unequal percentage splits.
- Fixed contributions and hybrid allocations.
- Income-based splits, including zero or unavailable income.
- Rounding to exact cents.
- Partial and missed payments.
- Outstanding reimbursements.
- Joint bank accounts.
- Duplicate transactions.
- Transfers between members.
- Refunds and reversals.
- Changing household membership.
- Historical allocation revisions.
- Midmonth arrangement changes.
- Conflicting budget obligations.
- Missing or incomplete financial information.
- Multiple households linked to one person.
- Reopening saved households after application restart.

Verify that household totals reconcile correctly to member allocations, recorded payments, and outstanding balances.

Ensure imported transactions, personal budget calculations, and household aggregates are never inadvertently counted twice.

Use database transactions and safe schema migrations where appropriate. Preserve compatibility with existing local workspace data.

## 15. Implementation Standards and Final Verification

Build this as production-oriented functionality within the existing application, following its established code structure, UI components, naming conventions, and testing framework.

Prioritize financial accuracy, data integrity, clarity, privacy, and usability over unnecessary visual complexity.

Avoid duplicate financial engines, redundant data entry, and independent systems that cannot reconcile with existing client tools.

Reuse common calculation services and maintain a clear separation between financial source records, obligations, payments, and projections.

Implement the interface, persistence, calculations, and related workflows rather than creating visual placeholders.

After implementation:

1. Run relevant automated tests and resolve failures.
2. Test the complete workflow using fictional clients and household arrangements.
3. Verify the example where three roommates split $2,400 rent at 25%, 37.5%, and 37.5%.
4. Confirm that changes to allocation rules update the relevant financial metrics without altering historical records.
5. Confirm that one person paying an entire shared bill does not cause duplicate household expenses.
6. Test individual and household tab switching.
7. Verify compatibility with existing budgeting, savings, debt, cash-flow, and statement-review features.
8. Verify that local data persists correctly after restarting the application.
9. Document any incomplete functionality or limitations instead of presenting it as finished.

**Final objective:** Create a flexible, reliable Household Management system that allows Eisenbart & Co. counselors to manage any reasonable household arrangement, customize responsibility for every shared expense, accurately track actual payments and reimbursements, analyze combined household finances, and understand how those arrangements affect each individual's financial position.

The completed system should feel like a seamless extension of the existing Eisenbart & Co. application—not a separate product—and should preserve the reliability and simplicity expected of professional financial counseling software.