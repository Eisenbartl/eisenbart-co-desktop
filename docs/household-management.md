# Household management and shared planning

Households extend the existing encrypted desktop workspace. Open **Clients →
Households**, or the **+** beside the workspace tabs. Create a named household,
then add existing clients by stable ID or limited-information members. Individual
records remain independent. Archive and reopen households through **Edit
household**; end a membership with an end date instead of deleting its history.

The supplied [implementation brief](household-management-specification.md) is
retained separately. Its image guides the navy rail, white panels, slate text,
muted gold accents, summary cards, category donut, member bars and compact tables.
Screenshot amounts and people are not seeded into real workspaces.

## Working sections

- **Overview:** monthly agreements, known authorized income, necessary and
  discretionary obligations, actual confirmed bill contributions, unfunded bill
  obligations, category/member charts and reimbursement suggestions.
- **Members & Permissions:** client links, limited members, relationship labels,
  membership dates and documented sharing authorization. A client may belong to
  several households. Departed members remain in historical financial records.
- **Shared Expenses:** recurring/one-time agreements, exact allocation preview,
  expected/actual entered amount, type, review status, payer/account, reimbursement
  date, effective dates and notes. Revisions preserve previous versions.
- **Allocation Rules:** effective category defaults; an expense override takes
  priority. Temporary agreements revert after their end date.
- **Payments & Settlements:** immutable dated bill payments, refunds, internal
  settlements, joint-account contributions, account transfers and authorized
  income movements. Void an incorrect record and enter its replacement.
- **Household Budget:** individual shares and explicitly confirmed budget
  reconciliation, including separately labeled shared goal contributions.
- **Household Cash Flow:** authorized account balances and dated movements,
  projected account balances/shortfalls, expected reimbursements, and controlled
  reconciliation into existing individual cash-flow accounts/schedules.
- **Spending Analysis:** current approved/categorized source links, purpose totals,
  category/member/trend charts, discretionary categories, discussion prompts,
  search/type filters, and discussion-bound purpose changes/history.
- **Goals & Scenarios:** shared sinking funds use the existing savings engine.
  Member/expense proposals preview changed obligations and known income after
  shared costs; explicit acceptance adds an agreement. Individual plan writes
  remain separate reconciliation steps.
- **Reports & History:** immutable internal household report snapshots, offline
  PDF export and the household journal. PDFs use the existing bundled font,
  pagination and no-overwrite writer. Opening/printing uses a local PDF viewer.

Open household tabs retain their section, filters and drafts while switching
between individual/household workspaces. Closing a draft or locking requires an
explicit discard. Lock, expiration and password changes remove household state
from the interface. Restarting retains saved data; unsaved drafts and open-tab
state are session-only.

## Calculation basis

All financial math and authorization execute in Rust with integer cents. The UI
formats authoritative amounts and chart scales; it does not calculate budgets,
splits, forecasts or settlement amounts.

Allocation supports equal, percentages, fixed-only, income-proportional and
individual responsibility. Each participant may first receive a fixed dollar
obligation; the remainder uses the selected method and specified participants.
A zero remainder weight makes a participant fixed-only. Percentages of a
nonzero remainder must total exactly 100%. Invalid/overallocated agreements
cannot be finalized. Largest fractional remainders receive leftover cents;
stable member IDs break ties. Allocations always sum to the entered amount.
Displayed percentages are independently truncated to two decimals, so displayed
percentages can sum to 99.99% even though cents reconcile exactly.

Income-proportional approvals require explicitly authorized, known monthly
income for participating members. Zero income receives zero proportional
remainder; an all-zero or unknown basis cannot distribute a nonzero remainder.
Approved income proportions are stored as reduced integer ratios, preserving
past agreed shares when private income later changes. Approving a new revision
captures the new authorized basis. No private income is fetched automatically.

Agreements use the version in force on each **due date**, inclusive of start/end
dates. Monthly dates clamp to the last day when needed; weekly, biweekly, annual,
irregular and one-time dates are supported. A midmonth revision affects later
due dates, without silently prorating an already due monthly bill. Enter an
explicit one-time adjustment when the household agrees to a partial month.
Income shown is an authorized month-end run rate, not verified cash received.
Membership changes and missing/restricted data are flagged.

Responsibility and payments are separate. For $2,400 rent at 25%/37.5%/37.5%,
obligations are $600/$900/$900. If the first member fronts $2,400, the other two
may each reimburse $900. Household spending is not increased by those transfers.
Confirmed external bill payments minus refunds establish contribution credit;
settlements/contributions increase the sender's net contribution and reduce the
recipient's. Suggestions only reimburse confirmed overpayment. Planned and
unverified records do not count as completed payments. The obligation period is
explicit, separate from the actual payment date.

Cash-flow projections use the existing dated engine. Full payer outlays and later
reimbursements remain separate events. Explicit reimbursement dates can generate
estimated transfers when a unique personal source account is available; otherwise
the view requests a dated planned settlement. No private paydays or account
identity are guessed. Unknown balances stay unknown. A balance must be dated at
the selected month's opening to supply that month's projection. Unverified
balances and planned payments remain labeled estimates.

## Sources, privacy and reconciliation

Import/review stays in the individual client workspace. Enable that member's
transaction-sharing permission, record its authorization, then match a payment
with an approved transaction. Source identity/approval, date, amount and permitted
transaction kind must match. Links reference the original bank/card record and
its immutable document locator; no original is copied, edited or deleted.

Only current approved, categorized, nonduplicate eligible evidence contributes
source actuals. Each complete source transaction may have one active household
payment link across households. Partial source attribution to several households
is not supported. Voiding a link permits an explicit replacement. Changed
approvals, missing evidence or revoked permission remove the payment from live
confirmed metrics and flag reconciliation; the original journal entry remains.
Manual confirmed payments remain separate counselor-entered evidence and do not
appear as imported spending.

Spending analysis separates gross spending, refunds/net spending, debt payments
and internal transfers. Its scope is linked evidence, not all household activity.
Small-purchase prompts require at least five reviewed discretionary purchases
under $15. Subscription prompts require existing counselor-confirmed recurring
reviews, and overspending compares approved discretionary actuals with the
household plan. Coverage/unknown-purpose warnings stay visible.

After a client discussion, change Necessary/Discretionary in the household
analysis. The existing individual classification service saves the source-bound
purpose/history. Household and individual analysis then use that same decision;
original source bytes/fields and historical report payloads remain unchanged.
Planning agreements change only through their own explicit revision workflow.

Membership grants no private sharing. Income, entered account balances,
transaction links and individual budget-expense reconciliation require separate
recorded permissions. Restricted income is omitted from household budget previews
and is preserved unchanged when other lines reconcile. Individual cash-flow
handoffs omit private opening balances and mask unshared income schedules.
Personal debt principals/assets are never merged automatically. Shared debt
payment obligations can be explicitly designated as such; joint amortization
models and personal payoff strategies remain in the existing individual tools.

Individual reconciliation previews current saved lines/schedules, identifies
previous household entries, asks the counselor to select overlapping personal
entries, and requires acknowledgment before saving. The server checks both
household and individual revisions, prevents duplicate prior household entries,
retains before/after data in the encrypted journal, and saves the plan and audit
records in one transaction. Income cannot be removed through these workflows.
New budget context feeds existing debt/savings/cash-flow tools without silently
replacing their independently saved assumptions. Goal deposits need explicitly
dated individual savings schedules; no contribution date is invented.

Household report snapshots retain the applicable agreement revision and approved
source evidence. If sharing permissions later change, the retained historical
payload is restricted from household display/export. Original source history
remains in the individual review system. Household reports are internal planning
records, not the existing final-client-assessment approval workflow.

## Storage and operational bounds

Schema 31 adds an append-only, organization/counselor-scoped household journal and
an audit event. Safe transactional migrations preserve prior sources and report
bytes. Events use contiguous revisions, canonical typed payloads, digest chaining
and replay validation. The backup recovery validator checks household history
before activation. Existing workspaces with no households migrate to an empty
household journal; no real client information is seeded.

Current bounded limits: 500 households per workspace, 100 members per household,
1,000 expense revisions, 500 category-rule revisions, 100 entered household
accounts, 5,000 payment records, 50 goals, 100 proposals and 100 report snapshots.
Each journal permits 10,000 events / 32 MiB replayed payload, with a 1 MiB event
limit. Individual budget reconciliation retains the existing 100-line budget limit.
The reused cash-flow engine permits 20 accounts and 100 schedules per
forecast; exceeding its scope produces an explicit unavailable forecast/warning,
not a partial result. Existing source-analysis limits still apply.

This remains a development build for fictional records. Native Linux validation
and automated financial examples do not certify Windows installed operation or
commercial readiness. This stage creates no installer.
