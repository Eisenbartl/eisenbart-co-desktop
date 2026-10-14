# Approved transaction financial dashboard — 0.11.0 / 0.12.0 extension

Open a saved client, go to the statement import/review panel, and enter **Assessment
from month** and **Assessment through month** as YYYY-MM. Select **View financial
dashboard**. The period includes both endpoints and all days of each calendar
month, up to 24 months. These view choices are temporary; the underlying sources
and decisions remain in the encrypted workspace.

## Amounts and charts

Rust reads the authenticated client's current imported records and calculates
exact cents. Only Approved rows with verified dates inside the period contribute
amounts. Pending, Needs Clarification and Rejected rows do not contribute money.
Undated rows cannot be assigned to a month and are disclosed separately across
the client. Corrected verified dates determine inclusion; original source fields
remain unchanged. Corrections and reversals trigger a fresh assessment from the
new import revision. Client/revision mismatches and late replies never replace
current results with another client's or an older assessment.

- Income receipts include approved Income and positive Interest. They do not
  assert that all receipts are take-home pay.
- Gross spending includes approved Expense, Fee and negative Interest.
- Refunds reduce net spending, never income. Negative net spending is retained
  when refunds exceed spending; no absent original purchase is invented.
- Operating cash flow is income minus net spending. Transfers and balance
  movements, including principal movements, do not enter this figure.
- All account movement includes every approved signed movement. It is a change
  from imported activity, not an account balance.

Three locally bundled SVG bar charts show gross spending by exact approved
category label, monthly income versus gross spending, and monthly operating cash
flow. Rust computes integer proportions for bar lengths; React formats canonical
money strings without floating-point money arithmetic. Text tables retain exact
values and negative signs, so information does not depend on color or bar length.
All monthly comparison bars share a scale; category bars compare the largest
category and cash-flow bars compare absolute amounts. No CDN, new dependency,
external font or telemetry is used.

## Completeness and missing facts

Every assessment is explicitly **Incomplete**: decided rows alone cannot prove
that all statements/accounts were imported. Review percentage is the integer
floor of (Approved + Rejected) / all dated period rows × 100. An empty period has
no percentage. Undated records are disclosed separately. Zero represents no
approved imported activity, not proof of zero actual income/spending. Empty
months are kept in the comparison and trend tables.

A period ending more than 90 days before the UTC calculation date is labeled
**Outdated period**; this is a disclosed display rule, not an assertion about
institution data freshness. Future transaction dates are flagged. Relevant source
warnings and reconciliation conflicts prompt source review. A matching arithmetic
check does not establish account coverage or verify an asset balance.

Version 0.12.0 adds explicit source-bound expense/payroll classifications and
dated counselor-verified balance facts, conditional net worth, coverage/payment
estimates and a local debt chart. See [verified-financial-facts.md](verified-financial-facts.md).
Gross-income lender DTI and savings progress remain pending. Existing budget/debt/savings/cash-flow panels are separate entered
plans, not approved imported facts. Saved immutable assessment snapshots and reporting are the next checkpoint. Saved final assessment snapshots and
reports belong to later reporting work.

The original 0.11.0 view required no schema migration and recomputes from schema-12 sources and
immutable reviews. It writes no financial records, approvals, plans or audit
history. Restart and both encrypted recovery paths preserve the sources from
which the same view can be recalculated. This milestone does not replace final
Windows installation/offline acceptance or independent security review.

## Independent synthetic examples

September income 1,000.01 + interest 2.22 = **1,002.23**. Rent 300.02 + fee 0.01 +
interest 3.33 = **303.36** gross spending. Refund 20.03 gives **283.33** net spending
and **718.90** operating cash flow. Principal −200 and paired transfers −50/+50
give **518.90** total account movement. October income 200.01 and spending 0.02
give **199.99** operating cash flow. The combined operating figure is **918.89**;
combined account movement is **718.89**. Out-of-period, rejected, undecided and
undated examples never enter these amounts.

## Validation checkpoint

Version 0.11.0 implementation b6d5eae passed 66 interface tests, 103 Linux library
plus 2 executable worker tests, formatting/Clippy and the compiled native
workflow including dashboard corrections/reversals, full restart, normal backup
restore and missing-metadata recovery. The focused dashboard suite passed after
the aggregate-limit example was added. Windows source CI [38587770252](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38587770252)
passed 104 library plus 2 worker tests and all 66 interface tests. Installer
creation, installer acceptance and uploads were skipped. Native diagnostics and
validation logs remain outside the repository; see development-status.md for
the interrupted driver attempt and completed fresh run.
