# Windows evaluation guide

The installer below is 0.23.0. Current 0.28.0 source additionally supports optional
client intake, dated follow-ups, a filtered dashboard overview and selected intake
in immutable internal reports;
see [the intake guide](client-intake-followups.md) [overview guide](follow-up-overview.md) and [report intake](report-intake-context.md).
Those additions require a newer build and are not in the linked 0.23.0 installer.

Version 0.23.0 is a Windows x64 evaluation build with encrypted local records,
client notes, budgets, debt/savings/cash-flow plans, reviewed statement imports,
approved-data charts, immutable scenarios, counselor reports and client-plan PDF
export. New temporary-income periods show monthly funding shortfalls and recovery;
new expenses, debt extras and savings-goal changes can be compared together.
Use fictional records while evaluating. Scanned-document OCR, advanced terms,
full intake/follow-up and commercial release requirements remain in development.
There is no forgotten-password reset or completed independent security review.

## Prototype navigation (0.27.0 source)

The sidebar has Dashboard, Clients, Financial Review, Transactions, Documents,
Budget Planner, Debt Planner, Savings Planner, Cash-Flow Calendar, Financial
Scenarios, Reports, and Settings and Administration. Choose a client at the top
of a focused page; their context carries between pages until changed or locked.
Use Clients for Profile, Intake & follow-ups, Session notes and History & records.
Complete record retains the integrated workflow. Save editable records before
leaving, or explicitly confirm discarding edits. Temporary unsaved scenario
previews reset on leaving; record a reviewed scenario to keep it.

Import statements in Documents, review their rows in Transactions and use
Financial Review for approved-data charts, facts, balance and payment checks.
Reports contains assessments and their client action-plan history/PDF controls.
Settings → Backup & recovery creates backups and routes you to locked-screen
verification for restore. Other settings areas include security, firm branding,
accounts/roles, data management, licensing and help. Future areas say **Planned**
and do not save data. See [the development map](prototype-development-map.md)
for working CRUD behavior versus remaining features. Archiving preserves history;
permanent deletion is not implemented.

## Track requested documents (0.28.0 source)

Open Documents → Collection checklist, select a client and add a request. Save
its title, type and optional account, requested period, due date and notes.
Mark received only after receipt and enter the received date; optionally link an
imported source belonging to that client/account. Receipt does not approve the
financial contents. Archive saved requests to keep them out of the active view;
uncheck Archive to reactivate. Past versions are read-only. See
[document collection](document-collection.md) for history, limits and recovery.

## Install and open

1. Download the **windows-installer** ZIP from a successful Windows build. See
   the [Windows download instructions](windows.md).
2. Right-click the ZIP in File Explorer and choose **Extract All**. Open the
   extracted folder and read `README.txt`.
3. Open the installer `.exe` and follow its instructions. This development
   installer is unsigned, so Windows may show an unknown-publisher warning.
   Check that your download came from the expected repository build.
4. Open the Windows Start menu and search for **Eisenbart Counseling Suite**.
   Open that installed application.

## Create your workspace

On the first launch, enter **Organization name** and **Counselor name**. You may
replace the initial Eisenbart & Co. organization name. Choose these names
carefully; changing them later is not available in this build.

Enter a long, unique passphrase in **Workspace password**, with at least 12
characters, then enter it again in **Confirm password**. Retain it securely
outside the application. Choose **Create workspace** and wait for the dashboard.
There is no forgotten-password reset, and reinstalling cannot recover access.

## Add and update a fictional client

1. Choose **Add client**.
2. Enter **First name**, **Last name**, and **Household size**. Email, phone, and
   counseling goals are optional. Use invented details throughout evaluation.
3. Choose a **Counseling status**, then choose **Create client**. Wait for the
   client's name to appear in the directory and at the top of the saved record.
4. To edit a saved profile, choose **Clients** and select the client's name.
   Change the details or status, choose **Save changes**, and wait for
   **Client saved**.

In **Clients**, use **Search name, email, or phone** to find a record. The dropdown
beside it filters by counseling status; choose **All statuses** to remove that
filter. **Archived** records remain available in the directory. The workspace dashboard summarizes client counts and counseling stages.
Approved financial charts appear with the client’s reviewed statement data.

## Save session notes

Open the intended client and enter text in **New session note**. Choose **Save
session note**, then confirm it appears in the history with the counselor's name
and date. A client must be created before notes can be added.

Save profile edits and session notes separately before switching records,
closing, or locking. If a prompt warns about unsaved changes, cancel it to return
and save your work. Saved notes remain in the history; add another note to record
a correction or update.

## Create a client budget

Open a saved client in **Clients** and scroll to **Monthly budget**.

1. Choose **Add income**, **Add expense**, or **Add allocation**.
2. Enter a **Category or source** name, choose the **Type**, and enter an
   **Amount (USD)** and **Frequency**. Use take-home income. Enter amounts such
   as `1500.50`, without dollar signs or commas. **Every two weeks** and
   **Twice a month** are different frequencies.
3. Leave an unknown amount blank; enter `0` for a known zero. Mark uncertain
   amounts as **Estimated amount**. For irregular income, enter your estimate
   of the monthly total; the application marks it estimated automatically.
4. Put planned savings and debt payments in their allocation types. Avoid
   entering the same debt payment again as an expense.
5. Choose **Save budget and calculate**. Review the monthly totals and warnings.
   A deficit means spending and allocations exceed the entered income.

Use **Remove** to delete a planning line, then save. Profile edits, notes, and
budgets have separate save buttons. Totals disappear while a budget has unsaved
changes; save to recalculate them. The monthly averages do not predict actual
paycheck or bill dates. The debt planning section below provides separate
fixed-monthly APR payoff estimates.

If another window saved a newer budget, close and reopen that client's record
before editing again. If loading fails, use **Retry loading budget**. Do not
replace the data folder to work around an error.

This version uses schema 16. Preserve the existing workspace and keep using the
newer application after upgrading; older versions may not open it. Read
[budget-calculations.md](budget-calculations.md) for calculation rules and examples.

## Compare debt repayment plans

Open a saved client and scroll to **Debt payoff planning**.

1. Enter **First planning month** as `YYYY-MM` (for example, `2026-11`). Enter
   **Total monthly debt payment (USD)**, including all minimum payments.
2. Choose **Add debt**. Enter a **Debt name**, type, outstanding balance, APR
   percentage, monthly minimum, and known monthly fee. Enter `0` for a known
   zero APR or fee. Leave unavailable terms blank; the app will save them and
   explain why it cannot yet calculate a complete comparison.
3. Choose the interest model. **Fixed monthly APR estimate** is supported.
   Daily, variable, promotional, and unknown models cannot be calculated yet.
   Check lender terms and retain any useful source notes. **Terms checked by
   counselor** records your own review; it does not verify information for you.
4. Add the other debts, then choose **Save debt plan and compare**.
5. Review **Strategy comparison**, its assumptions, and any warnings. Choose a
   **Schedule strategy** and **Schedule month** to inspect account payments,
   interest, fees, closing balances, and estimated payoff months.

**Extra payments and future payment changes** opens optional controls. An extra
payment adds money once in its month; a payment change replaces the entire monthly
payment budget from its month onward. Verify funding for those plans. Both
strategies use the same events and entered minimums.

A minimum-payment shortfall stops the schedule. A growing balance means some
payments did not cover interest and fees. An unfinished projection has no global
debt-free date, and its totals cover only the simulated months. Estimated monthly
payoff results do not replace lender statements or a dated cash-flow plan.

The budget reference shows the version last checked. Saving the debt plan checks
the latest saved budget and warns if payments exceed its monthly capacity after
savings. It does not change the budget. Keep the budget's debt allocations and
this plan consistent yourself for now.

Use the separate save buttons for profile details, notes, budgets, and debt plans.
If another window changed a saved debt plan, close and reopen the client before
editing again. Use **Retry loading debt plan** if loading fails. See
[debt-calculations.md](debt-calculations.md) for exact assumptions and examples.

Version 0.23.0 uses schema 16; keep using the newer version after upgrading.

## Plan savings and sinking funds

Savings planning is included in the current installer.

Open a saved client and scroll to **Savings planning**.

1. Use **Refresh budget and debt references** after saving those plans. It keeps
   your unsaved savings edits. Choose **First contribution month** and a 1-, 3-,
   6-, or custom-month emergency target.
2. Enter **Total accessible savings before earmarks**. This is the entire liquid
   pool before separating bill and sinking-fund reserves. Enter restricted
   savings separately; it will be excluded. Enter the amount **Reserved for
   upcoming bills** and a **Monthly emergency contribution**. Blank means
   unknown; enter `0` for a known zero.
3. Choose **Add sinking fund**. Enter its name, target amount/date, money already
   reserved from the liquid pool, and selected monthly contribution. Do not
   count the same money in more than one reserve bucket.
4. Optionally open **Compare emergency contributions** to enter alternatives.
   Record agreed **Funding priorities and notes**, then choose **Save savings
   plan and calculate**.
5. Review emergency coverage, target, estimated dates, suggested goal
   contributions, and funding warnings. Choose an **Allocation month** to inspect
   capped transfers, reserved debt payments, and the remainder or deficit.

Contributions arrive at month end in this model. A goal due before month end
cannot use that month's contribution. Suggested contributions round upward to
cents; emergency coverage rounds down. Dates assume no interest, withdrawals,
inflation, or changed expenses and do not guarantee actual funding.

Saving rechecks the latest budget/debt references and keeps those plans separate.
Adjust and reconcile their allocations yourself, including any annual bill
already budgeted as an expense. A scenario marked **Unavailable** has incomplete
funding information. A negative remainder or **Exceeds entered funds** needs
reduced allocations or verified additional funding.

If a savings plan changed in another window, reopen the client before saving.
Use **Retry loading savings plan** on a loading error. Keep using the current application after a schema upgrade. See
[savings-calculations.md](savings-calculations.md) for exact rules and examples.

## Lock and return

Save your work, then choose **Lock workspace** when stepping away. Enter your
password and choose **Unlock workspace** to return. The application also locks
after 15 minutes of inactivity. That interval is fixed in this build. Automatic
locking clears unfinished edits and notes, so save frequently.

Close the application and reopen **Eisenbart Counseling Suite** from the Start
menu. Use **Unlock workspace** with the original password, then check that your
saved fictional client and notes are present. Records are kept on this computer
for the Windows user who created the workspace.

For offline evaluation, disconnect from the internet after installation and
repeat creating a fictional client, saving a note, locking, and reopening. Report
any failure; fresh Windows 11 offline acceptance remains a separate test.

## If something fails

Read the displayed message. For a password error, check your typing, Caps Lock,
and keyboard layout before trying again. For a loading error, use **Try again**,
**Retry loading clients**, or **Retry loading notes** when offered.

If the error continues, stop and report the exact message and what you were
doing. Keep your password private and preserve the local files. Do not delete,
edit, or reset the data folder, create a replacement workspace, or reinstall as
a recovery shortcut. The [Windows guide](windows.md) explains where files live;
the [security guide](security.md) explains password and recovery limits.


## Forecast cash flow

Open a
saved fictional client and scroll to **Cash-flow calendar**.

1. Enter a start/end date, up to 366 days inclusive.
2. Add accounts and their balances immediately before the first day's activity.
   Include savings as another account if you intend to model internal transfers.
   Blank means unknown; 0 means a known zero. Negative opening balances are allowed.
3. Add dated paychecks, bills, loan payments, savings, or other activity. Dates
   and amounts must come from known schedules or clearly marked estimates.
   Weekly, every-two-week, twice-monthly, monthly, and irregular one-time schedules
   are supported. Twice-monthly activity needs a second day of month.
4. For internal transfers, select a different destination account. Use external
   savings payments only when the receiving account is outside this model.
5. Choose **Preview cash flow / refresh references**. Check calendar days for
   account shortfalls and concentrated payments; choose **Chronological ledger**
   to examine balances after individual events. Change the month to inspect longer
   periods. Preview does not save changed dates or amounts.
6. If a bill precedes payday, confirm whether the payee permits a different date.
   Edit that date and preview again to compare. Choose **Save cash-flow plan and
   calculate** to preserve the intended plan when ready.

A positive combined balance can coexist with insufficient funds in checking.
Same-day debits precede credits conservatively, so a payment can warn even if a
later deposit makes the day close positive. Unknown activity makes affected
balances unknown; other accounts can still be checked. Blank dates are rejected,
not invented. Bank holds, holidays, fees, delayed transfer settlement, and actual
posted transactions are not modeled. Confirmed schedules are not bank verified.

Budget, debt, and savings references help you review allocations, but their
monthly amounts do not provide verified transaction dates. Enter all dated
obligations once, including variable expenses, future debt extras, and intended
savings transfers. No budget, debt, or savings plan is changed automatically.
See [cash-flow-calculations.md](cash-flow-calculations.md) for exact assumptions.


## Back up and recover

Open **Settings**, choose a backup folder outside the app data folder, enter your current password, and choose **Create encrypted backup**. Keep multiple copies, preferably on a separate drive. To recover, lock the workspace and expand **Restore an encrypted backup**. Verify the selected file using its password, review the workspace, type **RESTORE**, and confirm. The app keeps prior files separately and does not merge newer records into the backup. Use the [backup and recovery guide](backup-recovery.md) for the complete procedure and password-loss consequences.

## Import and review CSV statements

Open a saved client, choose a synthetic USD checking/savings CSV, and click **Inspect CSV**. Select the actual date, description, amount, and optional classification columns; select the account and enter only known statement metadata. The complete original file is retained only if you check its authorization box. Click **Import into review queue**, then review and explicitly approve eligible rows. Nothing is approved by importing.

Refunds offset spending; paired internal transfers avoid extra income/expense. Corrections, conflicting rows, and reversals need individual review and reasons. Totals use only approved records and do not alter the client's planning inputs. See [CSV steps, supported formats, examples, and limits](csv-import-review.md).

## Import selectable-text PDF statements

Choose **Selectable-text PDF** in the client statement panel, inspect a synthetic PDF, expand its pages, and select only transaction lines. Choose the account and actual date format; leave unknown metadata blank. Original-file retention requires its checkbox. Import, then verify each record against its original page, classify/category it, enter a reason, and explicitly approve or reject it. Every PDF row starts as Needs Clarification; bulk approval is unavailable.

[PDF instructions, supported layout, examples, and limits](pdf-import-review.md) explain wrapped/ambiguous text, protected/scanned files, exact totals, and recovery.

## Review merchant groups and recurring payments

Approve the source transactions first. In the client's statement panel, scroll to **Merchant groups and recurring payments**, choose a group, and check its supporting payments. Correct verified Merchant fields to combine or separate names. Confirm or dismiss the pattern with a reason; choose the reviewed frequency and classify a subscription only after verifying it. Changed evidence requires another review. Decisions persist in encrypted storage and backups without changing planning inputs. See [steps, evidence rules, examples and limits](recurring-payment-review.md).

## Check statement running balances

Map the CSV's actual after-transaction balance column and declare oldest/newest-first order. In **Statement running-balance checks**, choose the statement to compare original source steps with approved cumulative amounts. Unknown data stays unknown; rejected amounts and undecided rows never supply approved balances. Date/order conflicts and discrepancies require individual review. Correct or clear an order declaration with a reason; original source values and transaction decisions remain unchanged. See [steps, examples, limits and supported balance meaning](statement-balance-reconciliation.md).

## Compare scenarios and temporary income

1. Save the client's normal budget and any debt, savings and cash-flow models.
2. Open **Compare combined financial scenarios** and choose **Reload saved scenario
   baseline**. Enter a useful proposal label and assumptions.
3. Compare changes to existing amounts, extra debt payments, goal contributions
   or new recurring expenses. Optional dates/accounts explicitly add proposed bills.
4. In **Model temporary income reductions**, choose **Add temporary income period**.
   Select a normal income line, reduced amount per payment, and inclusive first/last
   months. Blank means unknown; zero means no income. Use `YYYY-MM`.
5. Select a matching paycheck schedule only if you also want affected dated
   receipts changed. Original payment dates and receipts outside the period remain.
6. Choose **Compare temporary proposal**. Review the separate **Temporary income
   funding and recovery** table and warnings. The main monthly model uses normal
   recurring income; payoff and goal dates require the continuing commitments to
   be funded. This does not model savings withdrawals or automatically cut payments.
7. Choose **Save reviewed combined scenario** after review. Live plans and approved
   transactions remain unchanged. Reopen it through **Saved combined scenarios**.

Up to six reductions fit within a 24-month span; one following normal month is
shown when supported. Unknowns remain unavailable. Annual/irregular income,
overlapping reductions to one source and conflicting edits to the selected paycheck
are rejected. Dated forecasts retain their own saved period and 100-schedule limit.

## Save and export counselor reports

Open the saved client's assessment/report panel. Review the approved-data period,
title, findings and limitations. To attach a proposal, enable **Include a saved
combined scenario**, review the directory and select a current saved record.
Confirm the review acknowledgment and save the assessment. Use **Choose PDF
destination** or enter a full private path and export the saved assessment PDF.
Existing files are never overwritten. Reports retain their captured figures after
later changes; **Needs review** flags changed context without rewriting history.

To include intake, first save it in **Intake and follow-ups**. Enable **Include
selected saved intake context in this internal report**, choose **Review saved
intake** and select address, household and/or employment context. Reconfirm the
acknowledgment and save. Selected original values appear in the internal report
and PDF; client action-plan PDFs omit them. Selected details changing later leave
a permanent **Needs review** flag, even if reverted. Unselected intake and task
edits do not change this report context. See [report intake](report-intake-context.md).

Temporary-income reports include the funding/recovery table and conditional
forecast warnings. A separate planning report with reviewed monthly allocations
is required before recording a client agreement from temporary-income analysis.
Other eligible current captured planning reports support **Client action plans**,
three to five priorities, follow-up and separate client PDF export.

## Suggested progress check

Use a fictional household. Enter monthly income 3,000.00 and necessary expenses
1,500.00, then model monthly income 1,000.00 for two months. Review the period
figures and return to 3,000.00 afterward. Without a savings model, allocation checks
use known budget allocations; missing information stays unavailable. Add a 75.00
monthly necessary expense and compare again. Save a scenario/report, export a PDF,
restart the app and reopen them. Create an encrypted backup in Settings before
trying additional changes. Keep your password; backup encryption does not bypass it.
