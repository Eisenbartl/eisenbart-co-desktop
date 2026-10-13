# Windows evaluation guide

This development build provides a local workspace, client profiles, counseling
statuses, dated session notes, client budgets, and debt payoff comparisons. Use fictional records only. Advanced financial
calculations, statement imports, savings goal plans, PDF reports, and
commercial licensing are still being developed. Password recovery and tested
backup/restore are not available.

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
filter. **Archived** records remain available in the directory. The dashboard
shows client counts and counseling stages; it does not calculate financial results.

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

The current 0.3.0 version upgrades the database to schema 4. Older application
versions cannot open it afterward, so keep using the newer version. Read
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

Version 0.3.0 upgrades the database to schema 4. The older 0.1.0 and 0.2.0 apps
cannot open it afterward; keep using the newer version.

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
