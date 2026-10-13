# MASTER DEVELOPMENT INSTRUCTIONS
## Eisenbart & Co. Financial Counseling Suite
### Build a Complete, Offline-First, Commercially Distributable Desktop Application

## 1. YOUR ROLE AND PRIMARY OBJECTIVE

Act as an integrated software development team consisting of:

- Principal software architect
- Senior desktop application engineer
- Full-stack developer
- Database architect
- Financial software specialist
- Financial counseling workflow designer
- Security and privacy engineer
- UI/UX designer
- Quality assurance engineer
- Software packaging and release engineer
- Technical documentation specialist

Your assignment is to **design, develop, test, and package a functioning Windows desktop application** called Eisenbart & Co. Financial Counseling Suite.

This must be an actual application with working functionality, persistent data storage, and a usable graphical interface.

Do not stop after producing a plan, requirements document, mockup, pseudocode, or code snippets.

Execute the development work to the extent supported by the available environment, and clearly distinguish implemented functionality from incomplete or untested functionality.

The intended result is a professional-grade financial counseling application that:

1. Works entirely offline for all essential operations.
2. Does not depend on Google Sheets, Google Drive, cloud servers, or external APIs.
3. Stores client information securely on the local computer.
4. Allows counselors to manage clients and analyze financial records.
5. Imports and processes financial documents locally.
6. Provides budgeting, debt management, savings planning, and financial forecasting tools.
7. Generates professional financial counseling reports.
8. Supports customization by different counseling firms.
9. Can eventually be commercially licensed and sold.
10. Can be expanded to support multiple counselors and organizations without rebuilding the core architecture.

Prioritize correctness, reliability, security, usability, and maintainability over flashy features or rapid but fragile implementation.

This application is initially for Eisenbart & Co., a financial counseling business focused on budgeting, debt, savings, cash-flow management, and general personal financial education.

It is not initially intended to provide regulated securities advice, trade execution, investment portfolio management, or insurance sales.

Design its features and disclosures accordingly.

---

## 2. EXISTING PROJECT — IMPORTANT CONTEXT

An extensive financial counseling system has already been developed in Google Sheets for Eisenbart & Co.

Before development, inspect the existing development workbook and related documentation if they are accessible with authorization.

Treat this workbook as a **reference implementation and source of requirements**, not as a mandatory long-term component.

Existing functionality and planned improvements include:

- Client intake and financial profiles
- Financial document collection
- Bank and credit card statement processing
- Transaction extraction and classification
- Transaction and extracted-figure approval workflows
- Financial review dashboards
- Income and expense analysis
- Spending categorization
- Debt management and repayment simulations
- Snowball and avalanche comparisons
- Emergency savings estimators
- Sinking funds
- Paycheck-based cash-flow forecasting
- Planning scenarios
- Internal counselor reports
- Client financial action plans
- PDF report generation

Identify which features actually exist and work, which are incomplete, and which are only planned.

Do not assume an earlier feature works without verifying it.

Preserve useful calculation rules, validation logic, report concepts, and established workflows.

However, **do not make the application dependent on the original spreadsheet**.

Reimplement approved logic in the application and create automated regression tests using verified spreadsheet examples where available.

Do not alter or delete the original workbook.

Do not import real client records into development without explicit authorization. Use fictional test clients and synthetic financial statements by default.

If the workbook cannot be accessed, proceed using these specifications and document any assumptions.

---

## 3. DEVELOPMENT TECHNOLOGY

Use a modern, professional, maintainable desktop architecture.

Preferred technology stack:

| Component | Preferred technology |
|---|---|
| Desktop application | Tauri |
| Frontend | React with TypeScript |
| Core business logic | Rust |
| Database | SQLite |
| Database encryption | SQLCipher or a suitably reviewed equivalent |
| Financial calculations | Deterministic, decimal-safe calculation engine |
| Charts | Offline-capable, locally bundled charting library |
| PDF processing | Local PDF parsing libraries |
| OCR | Local OCR engine with an offline language model |
| PDF report creation | Local PDF generation |
| Automated testing | Rust, TypeScript, and desktop integration tests |
| Packaging | Native Windows installer |

Use current stable, mutually compatible releases rather than blindly pinning outdated versions.

You may choose an alternative technology only if there is a significant technical justification. Document the trade-offs.

Prefer permissively licensed dependencies suitable for commercial distribution.

Audit dependency licenses, particularly PDF libraries, OCR engines, encryption components, fonts, and third-party UI assets.

The application should not require ongoing payments for its basic operation.

The internet may be used during development to acquire dependencies, but the resulting application must work offline, including on a fresh supported Windows installation.

Bundle required runtime components in an offline installation package where practical and legally permitted.

Target Windows 11 x64 first. Structure the application to permit future macOS and Linux releases.

---

## 4. NONNEGOTIABLE ARCHITECTURAL REQUIREMENTS

### Offline-first operation

All essential application features must function without an internet connection.

These include:

- Logging in locally
- Creating and editing client records
- Importing documents
- Extracting document information
- Categorizing transactions
- Reviewing and approving financial data
- Performing calculations
- Running debt and savings simulations
- Producing charts
- Generating reports
- Exporting data
- Backing up and restoring the database

Do not quietly substitute a cloud API for a requested offline function.

If a feature cannot be implemented offline, state that limitation instead of disguising a network dependency.

### Separate application layers

Create clear separation between:

1. User interface
2. Business logic
3. Financial calculation engine
4. Document-processing engine
5. Database and persistence
6. Reporting engine
7. Authentication and security
8. Licensing and product configuration

Avoid putting critical calculations directly inside React components.

Avoid creating a monolithic codebase that is difficult to test or expand.

### Multi-organization support

Even though the first release will be used by a single counselor, design the underlying data structures to support multiple firms and counselors.

Use appropriate organization, counselor, client, and record identifiers.

Ensure organization ownership and access permissions are consistently enforced, not merely hidden through interface filters.

Each independently installed firm's database must remain separate.

Do not implement shared access to one SQLite file over a network.

Future collaboration may use a properly designed local-server or network service, but must not be required for the first release.

---

## 5. DESIGN AND USER EXPERIENCE

Create a modern, polished, professional interface suitable for a financial services practice.

Use Eisenbart & Co. branding as the initial configuration.

### Visual direction

- Professional and sophisticated
- Clean, minimal interface
- Strong typography
- Restrained color palette
- Slate, charcoal, white, and subtle gold accents
- Generous whitespace
- Consistent spacing
- Simple navigation
- Clear financial charts
- Appropriate accessibility and contrast
- No unnecessary animation
- No cluttered dashboards

Avoid a flashy fintech aesthetic.

The software should feel like a well-established professional financial counseling tool.

Support common desktop window sizes, scaling, keyboard navigation, and accessibility.

### Main application navigation

Create the following functional sections:

1. Dashboard
2. Clients
3. Financial Review
4. Transactions
5. Documents
6. Budget Planner
7. Debt Planner
8. Savings Planner
9. Cash-Flow Calendar
10. Financial Scenarios
11. Reports
12. Settings and Administration

Navigation must be functional, not decorative.

Each section must have coherent empty states, loading indicators where relevant, validation messages, and error handling.

---

## 6. CLIENT MANAGEMENT SYSTEM

Build a complete local client management module.

### Client profiles

Support:

- Client identification
- Contact information
- Household composition
- Financial goals
- Employment and income details
- Counseling history
- Notes
- Appointments and follow-up reminders
- Document inventory
- Financial accounts
- Financial review status
- Action-plan history

Sensitive fields should be optional unless essential to the relevant workflow.

Use internal identifiers instead of relying on client names as unique keys.

### Client lifecycle

Support a structured workflow:

New Intake → Information Collection → Financial Review → Planning → Action Plan → Follow-Up → Archived

Allow counselors to change statuses with appropriate safeguards.

### Counselor notes

Provide a local notes system with timestamps and clear authorship.

Record material financial counseling decisions and approved planning assumptions.

Avoid silently overwriting previous counselor notes.

### Multiple client support

Client records must remain completely isolated.

Changing the selected client must never carry data, calculations, or reports from another client.

Include automated tests for cross-client data contamination.

---

## 7. DOCUMENT MANAGEMENT AND EXTRACTION

Build a local document-processing system.

### Supported input

Prioritize:

- CSV transaction files
- Text-based bank statement PDFs
- Credit card statement PDFs
- Loan statements
- Savings account statements
- Pay stubs
- Relevant financial documents

Add scanned-document OCR where technically feasible.

Design a modular parser architecture so future releases can support more institutions and document formats.

Do not claim universal statement compatibility.

### Document workflow

Import → Identify → Extract → Validate → Review → Approve → Apply

Retain the original source document when authorized by the counselor.

Every extracted financial record must retain provenance such as:

- Source document
- File identifier
- Account association
- Statement period
- Original reported amount
- Extraction method
- Page or other source locator, when available
- Confidence or uncertainty indicator
- Review status

Never invent missing amounts, account numbers, transaction dates, or balances.

### Extraction quality

Support detection of:

- Dates
- Merchants
- Transaction descriptions
- Debits and credits
- Account balances
- Interest
- Fees
- Transfers
- Payments
- Refunds
- Deposits
- Statement totals

Validate statement-level consistency where possible.

Flag ambiguities and differences between extracted figures and transaction totals.

Use human review for uncertain financial records.

### File security

Validate input files, enforce resource limits, and protect against malformed or malicious documents.

Do not execute active content embedded in imported documents.

Keep document parsing isolated from sensitive application capabilities where practical.

---

## 8. FINANCIAL REVIEW CENTER

Implement a centralized financial review system based on the previously established workflow.

Use these exact review states:

- Pending
- Needs Clarification
- Approved
- Rejected

Allow counselors to:

1. Review all extracted transactions for a selected client.
2. Filter by account, document, date, merchant, category, type, and status.
3. Assign or correct spending categories.
4. Group transactions from the same merchant.
5. Identify recurring payments and subscriptions.
6. Distinguish income, expenses, transfers, refunds, fees, and interest.
7. Detect possible duplicates.
8. Review extracted statement balances and totals.
9. Bulk-approve records only when validation criteria are satisfied.
10. Require individual review for uncertain or conflicting information.
11. Reverse prior approvals with an audit record.
12. Review unresolved exceptions.
13. See approval progress and data-completeness indicators.

### Approved data

Approved transactions and figures must form the basis of official financial assessments and finalized reports.

Pending information may appear in draft analysis only when clearly labeled.

Rejected records must not contaminate calculations.

A reversal must update dependent results and preserve the prior decision history.

### Financial data controls

Prevent double-counting of:

- Transfers between accounts
- Credit card purchases and subsequent card payments
- Loan principal movements
- Refunds and reversed transactions
- Savings transfers

Clearly distinguish expenses from balance-sheet movements.

Maintain reconciliation checks to catch discrepancies.

---

## 9. FINANCIAL DASHBOARD

Create a client-specific financial overview.

Display:

- Monthly take-home income
- Monthly spending
- Necessary expenses
- Discretionary expenses
- Net monthly cash flow
- Cash and accessible savings
- Outstanding debt
- Estimated net worth, where enough data is available
- Debt-to-income metrics where meaningful
- Emergency savings coverage
- Review completeness

Include:

- Spending category chart
- Monthly income and expense comparison
- Debt balance chart
- Savings progress
- Cash-flow trends

Charts must use real application data or visibly identified fictional demo data.

Do not display invented figures in production records.

Clearly indicate the assessment period and whether the figures are approved, estimated, incomplete, or outdated.

---

## 10. BUDGET PLANNING ENGINE

Develop a customizable budgeting system.

### Core requirements

Calculate:

- Total monthly income
- Necessary monthly expenses
- Discretionary monthly expenses
- Total spending
- Surplus or deficit
- Proposed savings allocations
- Available debt repayment funds

Support irregular income and different income frequencies.

Provide optional budgeting methods such as:

- Zero-based budgeting
- 50/30/20
- Custom percentage allocations
- Fixed-expense and discretionary-expense planning

These methods are planning frameworks, not mandatory rules.

Allow counselors to customize categories and recommendations.

### Financial accuracy

Use decimal-safe money calculations and consistent rounding rules.

Support monthly normalization of weekly, biweekly, semimonthly, monthly, and irregular amounts.

Preserve original transaction amounts and dates.

Flag missing or estimated income and expense information.

Budget changes must not silently modify approved historical transaction records.

---

## 11. DEBT MANAGEMENT ENGINE

Create a reliable debt repayment planning system.

Support:

- Credit cards
- Auto loans
- Student loans
- Personal loans
- Mortgages
- Other installment obligations
- Custom debt accounts

Each debt account should include, where known:

- Account identifier
- Debt type
- Current principal or outstanding balance
- APR
- Minimum payment
- Due date
- Interest calculation method
- Applicable fees
- Introductory or promotional rate information
- Payment frequency
- Remaining loan term
- Source and verification status

Do not invent unavailable loan terms.

### Repayment strategies

Implement:

**Debt Snowball**

Apply extra available payments toward the smallest qualifying debt balance, while maintaining required minimums on other debts.

**Debt Avalanche**

Apply extra available payments toward the highest qualifying interest rate, while maintaining required minimums on other debts.

### Simulations

Display:

- Proposed monthly payment per account
- Payment schedule
- Projected payoff date
- Estimated interest paid
- Total projected payments
- Debt-free milestone
- Remaining balances over time

Allow comparison of both strategies using identical financial assumptions.

Handle:

- Insufficient payment budgets
- Negative amortization
- Zero-interest debts
- Variable or promotional rates when modeled
- Extra lump-sum payments
- Changes in available monthly payment amounts
- Unknown or incomplete account terms

Interest models and compounding assumptions must be explicitly documented.

Do not present projected payoff dates as exact when underlying interest terms or cash flows are uncertain.

### Debt allocation

Coordinate debt payments with emergency savings and other financial priorities.

Never allocate more than the genuinely available monthly cash flow without producing an explicit warning.

---

## 12. SAVINGS PLANNING ENGINE

Develop comprehensive savings tools.

### Emergency savings

Support targets of:

- One month
- Three months
- Six months
- Custom number of months

Base the target on necessary monthly expenses, not automatically on gross income or total discretionary spending.

Clearly distinguish:

- Accessible emergency savings
- Restricted savings
- Sinking funds
- Funds reserved for upcoming obligations

Support adjustable monthly savings contributions.

Estimate:

- Current emergency coverage
- Amount remaining to goal
- Estimated completion date
- Savings contribution scenarios

### Sinking funds

Support savings goals for:

- Car repairs
- Insurance premiums
- Annual bills
- Medical expenses
- Holidays
- Home maintenance
- Other predictable irregular expenses

Each sinking fund should contain a target amount, target date, amount already reserved, and suggested contribution.

### Savings and debt coordination

Allow counselors to establish planning priorities between emergency savings, sinking funds, and accelerated debt repayment.

Warn when selected allocations exceed available funds.

Avoid treating transfers into savings as expenses that artificially inflate household spending.

---

## 13. PAYCHECK-BASED CASH-FLOW CALENDAR

Create a calendar and cash-flow forecasting tool.

Support:

- Weekly pay
- Biweekly pay
- Semimonthly pay
- Monthly pay
- Irregular pay schedules

Use counselor- or client-provided payment dates and expected amounts.

Accept:

- Paycheck dates
- Expected deposits
- Scheduled bills
- Loan payment dates
- Savings transfers
- Other anticipated financial activity

Calculate projected account balances over the selected period.

Identify:

- Potential overdrafts
- Periods of insufficient funds
- Bills due before the next paycheck
- Excessively concentrated payment dates
- Feasible adjustments to payment schedules

Distinguish confirmed scheduled transactions from estimates.

Never manufacture due dates or pay dates.

For multiple accounts, model account balances and transfers separately before producing a combined financial view.

Include calendar and chronological ledger views.

---

## 14. FINANCIAL SCENARIO SIMULATOR

Develop a tool that allows counselors to create and compare hypothetical financial decisions.

Example scenarios:

- Increasing debt payments
- Reducing discretionary spending
- Increasing income
- Making an extra loan payment
- Building emergency savings first
- Splitting surplus funds between debt and savings
- Changing a savings contribution
- Experiencing a temporary income reduction
- Adding a new recurring expense

Scenarios must not change approved historical records.

Each scenario should retain:

- Client association
- Date created
- Baseline data version
- Input assumptions
- Proposed changes
- Projected outcomes
- Uncertainty warnings

Support side-by-side comparisons.

Clearly separate historical financial facts from hypothetical projections.

---

## 15. PROFESSIONAL REPORT GENERATION

Implement two distinct reporting systems.

### A. Counselor Report

A detailed internal financial assessment containing:

- Client identity and assessment period
- Income analysis
- Expense categories
- Account balances
- Debts and repayment terms
- Cash-flow findings
- Savings coverage
- Relevant assumptions
- Missing information
- Transaction discrepancies
- Document sources
- Selected financial scenarios
- Recommendations
- Follow-up notes

### B. Client Action Plan

A clean, professionally formatted PDF suitable for giving directly to the client.

Include:

- Counseling firm's branding
- Client name
- Report date
- Assessment period
- Summary of financial position
- Income and spending breakdown
- Agreed budget recommendations
- Emergency savings target
- Selected debt repayment strategy
- Proposed monthly allocations
- Projected savings and debt milestones
- Three to five prioritized action items
- Follow-up arrangements
- Appropriate disclosures

Reports should have excellent typography, readable charts, and restrained styling.

Allow print, PDF export, and local saving.

### Report validation

Before generating a final report, verify:

- Correct client
- Approved data
- Current calculation version
- Document review status
- Account assignments
- Planning assumptions
- Data completeness
- Selected financial scenario

Do not mix information between clients.

Clearly disclose missing information and estimates.

A finalized report should retain an immutable snapshot of its supporting data and assumptions.

---

## 16. DATABASE AND INFORMATION SECURITY

Because this software handles sensitive personal financial information, security is a release-blocking requirement.

Implement:

- Encrypted local database storage
- Suitable authenticated encryption for stored document files
- Strong local authentication
- Password-based key derivation using a reviewed mechanism
- Secure handling of encryption keys
- Session locking
- Configurable inactivity timeout
- Appropriate permissions and role enforcement
- Audit logging
- Secure backup and restore
- Data export
- Safe client archival and deletion
- Database migrations
- Database integrity verification

Do not store raw passwords.

Do not hard-code encryption keys, administrator credentials, or secrets.

Do not write sensitive account information, document text, passwords, or tokens to diagnostic logs.

Minimize collected personal information.

Use operating-system security facilities when suitable, but ensure the required core functionality works offline.

Document the recovery procedure and the consequences of losing encryption credentials.

### Backup

Provide encrypted local backups with integrity checking and a tested restoration process.

Allow backup to user-selected storage, including external drives.

Prevent accidental overwriting of valid backups.

Test recovery after simulated application and database failures.

### Privacy

No advertisements, behavioral tracking, hidden telemetry, or transmission of client information to the developer.

Any future online integration must be explicitly optional and disabled by default.

Prepare clear documentation describing where financial data is stored and how it is protected.

Do not claim regulatory compliance or certified security without actual supporting validation.

---

## 17. COUNSELOR ACCOUNTS AND PERMISSIONS

Prepare the system for multiple counselor accounts within an organization.

Potential roles:

- Organization Administrator
- Counselor
- Read-Only Reviewer

Define explicit permissions for accessing clients, viewing documents, editing financial data, approving transactions, generating reports, and managing users.

For the first release, prioritize multiple profiles on one installation.

Later releases may support shared access across a local network, but this must be a separate, properly engineered feature.

Never expose one organization's records to another organization.

---

## 18. COMMERCIAL SOFTWARE READINESS

Design the application so it can eventually be sold to independent counselors and financial counseling businesses.

### Organization customization

Allow each firm to configure:

- Business name
- Logo
- Address
- Contact details
- Report branding
- Default spending categories
- Financial counseling templates
- Optional disclosure wording
- Preferred financial planning settings

Keep application product branding separate from organization branding.

Eisenbart & Co. should be the default organization configuration, not a permanent restriction.

### Licensing architecture

Prepare for:

- Individual licenses
- Small-practice licenses
- Future commercial editions
- Optional paid feature tiers

The software must not require constant internet access to validate a license.

Prefer locally verified, cryptographically signed license files where commercial licensing is implemented.

Protect users' ability to retrieve and export their data if a license expires.

Do not introduce artificial restrictions into the first development build.

### Commercial distribution

Review:

- Third-party software licenses
- Copyright notices
- Trademark considerations
- Installer signing requirements
- Windows distribution requirements
- End-user licensing terms
- Privacy documentation
- Support and update procedures

List unresolved legal or commercial matters rather than inventing legal conclusions.

---

## 19. TESTING AND QUALITY ASSURANCE

Develop a meaningful automated test suite.

### Required financial calculation tests

Test:

- Income frequency conversions
- Budget totals
- Cash surplus and deficit
- Transfers and double-counting prevention
- Credit card payment accounting
- Debt repayment schedules
- Interest calculations
- Snowball and avalanche ordering
- Minimum payment handling
- Negative amortization
- Emergency savings targets
- Savings completion estimates
- Sinking fund allocations
- Cash-flow calendar forecasts
- Report totals

### Data integrity tests

Test:

- Client isolation
- Organization isolation
- Database persistence
- Schema migrations
- Approval and rejection transitions
- Approval reversal
- Duplicate detection
- Backup and restore
- Corrupt database handling
- Interrupted imports
- Invalid financial documents

### User experience tests

Test complete workflows from beginning to end.

At minimum:

1. Install application.
2. Complete initial setup.
3. Create fictional counselor and organization profiles.
4. Add a fictional client.
5. Import synthetic financial statements.
6. Review extracted transactions.
7. Approve financial information.
8. Generate a financial dashboard.
9. Produce debt payoff projections.
10. Create savings goals.
11. Generate a cash-flow forecast.
12. Compare planning scenarios.
13. Generate a client action plan.
14. Close and reopen the application.
15. Verify all records persist.
16. Back up the application database.
17. Restore the backup.
18. Disconnect internet access and repeat all essential workflows.

Verify that the installed application—not merely the development server—works.

Document test results and failures.

Do not claim tests passed unless they were executed successfully.

---

## 20. SOFTWARE DEVELOPMENT STAGES

Complete the project incrementally.

Each stage should produce working, inspectable deliverables.

### Stage 1 — Inspection and architecture

- Inspect the accessible existing workbook.
- Inventory established functionality.
- Identify reusable financial rules.
- Design application architecture.
- Define database schema.
- Define financial data types and validation rules.
- Create repository structure.
- Create project documentation.

Immediately proceed to development.

### Stage 2 — Working desktop foundation

Build a functioning desktop application with:

- Installer-capable Tauri project
- React interface
- Navigation
- Local database
- Initial organization setup
- Local authentication
- Client profiles
- Persistent records

Compile and launch this version.

### Stage 3 — Financial calculation core

Implement:

- Income and expense calculations
- Budgeting
- Debt payoff engine
- Savings engine
- Cash-flow forecasting
- Scenario simulation

Add unit tests and verify results.

### Stage 4 — Document processing

Implement:

- Local document imports
- Transaction parsing
- Statement extraction
- Validation
- Category suggestions
- Review workflow
- Approval controls
- Source tracking

Prioritize reliable text-based PDF and CSV support before attempting broad scanned-statement OCR compatibility.

### Stage 5 — Reports and dashboard

Implement:

- Charts
- Financial summaries
- Counselor reports
- Client action plans
- PDF generation
- Report validation
- Printable financial schedules

### Stage 6 — Security and commercial architecture

Complete:

- Encryption
- Authentication safeguards
- Backups
- Permissions
- Audit logs
- Organization branding
- Commercial dependency audit
- Offline licensing architecture

Security cannot be considered complete until independently tested to the extent available.

### Stage 7 — Production packaging

Produce:

- Application executable
- Windows installer
- Required offline runtime components
- Installation instructions
- User guide
- Administrator guide
- Backup and recovery guide
- Technical documentation
- Dependency license inventory
- Known-issues document

### Stage 8 — Final verification

Perform final automated and manual testing.

Verify installation on a clean supported Windows environment where available.

Demonstrate essential functionality without internet access.

Identify remaining blockers before calling a build production-ready.

---

## 21. DEVELOPMENT EXECUTION RULES

These instructions are critical.

### Do the work, not just the planning

Do not respond with only a roadmap or instructions for me to write the code myself.

Create the actual source files, database migrations, UI components, financial engines, test files, and packaging configuration.

Use available development and filesystem tools.

Create working artifacts rather than illustrative snippets.

### Make reasonable decisions independently

Choose sensible implementation details without repeatedly asking for minor preferences.

Use the agreed architecture and visual direction.

Ask for input only when an essential decision cannot be safely made.

Never fabricate passwords, signing certificates, customer records, or external credentials.

### Avoid endless development loops

Do not become trapped repeatedly debugging the same issue.

If an operation fails:

1. Identify the actual error.
2. Attempt a targeted correction.
3. Re-run the relevant test.
4. Record the result.
5. After three unsuccessful substantially similar attempts, stop repeating that approach.
6. Document the blocker and continue with independent work where possible.

Do not make the same unsuccessful tool call indefinitely.

### Preserve checkpoints

At the end of every major stage:

- Save the working source.
- Commit changes if version control is available.
- Run available tests.
- Record passing and failing functionality.
- Preserve a recoverable checkpoint.
- Update the development status.

Avoid replacing a functioning build with an untested rewrite.

### Limit scope intelligently

First create a stable, complete minimum viable application.

Then progressively implement the advanced features.

Do not create 15 partially functioning modules when five fully functioning integrated modules would provide a better foundation.

A module counts as complete only when it works, persists data correctly, handles relevant errors, and passes its defined tests.

Do not sacrifice financial correctness or security to increase the number of features.

### Report truthfully

Never describe a mockup as a completed application.

Never describe placeholder calculations as production-ready.

Never claim a Windows installer was produced if only source code exists.

Never claim a security audit, privacy certification, or regulatory approval that has not occurred.

If an environment limitation prevents completion, report exactly what was implemented, tested, and blocked.

---

## 22. FINAL DELIVERABLES

The complete project should ultimately contain:

**A. Source code**

A structured, documented project repository containing all application source files and build instructions.

**B. Installable application**

A Windows installer that launches the desktop application and supports offline use.

**C. Persistent local database**

An encrypted local storage system with migrations, backup, and restore.

**D. Working financial modules**

Functioning client management, document review, budgeting, debt, savings, cash-flow planning, and reports.

**E. Test suite**

Automated tests covering essential financial calculations and application integrity.

**F. Documentation**

Installation, operation, troubleshooting, security, backup, and development documentation.

**G. Commercial readiness report**

An assessment of remaining work required before distributing or selling the software to other financial counseling practices.

**H. Demonstration dataset**

Fictional counselor profiles, client accounts, transactions, debts, savings, and financial reports so the software can be evaluated without exposing real financial records.

Do not include real client data in the demonstration package.

---

## 23. RELEASE ACCEPTANCE CRITERIA

The first stable offline release should satisfy all of the following:

- Installs and launches successfully on its supported Windows environment.
- Can operate without internet access.
- Can create and manage multiple fictional clients.
- Stores information persistently and securely.
- Provides accurate financial calculations.
- Can import supported financial documents.
- Provides a reliable transaction review and approval workflow.
- Produces debt and savings projections.
- Generates a professional PDF action plan.
- Supports secure backup and restore.
- Does not mix records between clients or firms.
- Does not transmit sensitive information externally.
- Has a documented and tested recovery procedure.
- Includes appropriate financial planning disclosures.
- Passes defined release-blocking automated tests.

Any unmet criterion must be marked clearly as incomplete.

Commercial release should additionally require appropriate independent security review, usability testing, dependency and licensing review, installer signing, and legal review.

---

## 24. WHAT I EXPECT FROM YOU

Begin immediately.

First, inspect available resources and establish the project structure.

Then build the smallest end-to-end functioning version of the application.

Advance through the stages, continuously testing and improving the software.

Show completed results at logical checkpoints rather than repeatedly asking me to authorize the next minor development action.

Use actual tools to write files, run the application, execute tests, and create distributable artifacts.

If a stage is blocked, preserve the working code and provide a specific technical explanation. Continue other independent stages when safe.

Prioritize complete, correctly functioning financial workflows over superficial visual polish.

Use Eisenbart & Co. as the initial brand, but design the system as a standalone commercial software product that could eventually be adopted by other counseling firms.

**Your mission is to deliver the most complete, reliable, secure, professionally designed, offline-first financial counseling desktop application achievable in the available development environment—not merely to describe how one could be built.**

Begin with Stage 1 and proceed directly into implementation.