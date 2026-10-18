# Native desktop smoke test

`scripts/desktop-smoke.py` launches the compiled Tauri application through
the native WebKit WebDriver directly on Linux. It uses Python 3's standard library;
Selenium and a development web server are not needed.

The test creates a temporary workspace containing fictional clients. It verifies
first-run organization and counselor setup, client creation and editing, session
notes and saved budgets belonging to the correct client, independently checked
monthly budget totals and allocations, snowball/avalanche interest totals and
account payment schedules, emergency/sinking-fund forecasts and contribution
scenarios, dated account balances, paired transfers, calendar/ledger reviews, non-writing date previews, saved debt/savings/cash-flow isolation, workspace locking, rejection of an
incorrect password, unlocking, and persistence after stopping and relaunching the
application and its native driver. It also imports synthetic checking/savings CSVs through visible controls, explicitly maps columns, verifies no automatic approval, bulk-approves validated rows, individually approves a refund, reverses an approved fee with history, pairs equal opposite transfers, rejects a repeated exact source, and checks independently known totals, source retention, restart/client isolation, and backup-restored decisions/history. It additionally creates an encrypted backup, rejects the wrong backup password, interrupts a verified review by stopping the entire process group, confirms original records still open, explicitly restores the backup, verifies the original files remain, and recovers from missing metadata through the storage-error screen. It also checks approved financial dashboard periods, local category/monthly/cash-flow charts, exclusion of pending PDF spending, automatic recalculation after corrections/reversals, and recreation from persisted/restored sources. It also verifies source-bound necessary/discretionary/payroll classifications, dated complete inventory, exact net worth and conservative ratios, correction/reversal invalidation, explicit re-verification, immutable history, and fact persistence through restart and both recovery paths. It additionally saves original and post-withdrawal counselor assessments, verifies old approved figures remain immutable and flagged Needs review, and restores both records/evidence through restart and both encrypted recovery paths. All mutations target temporary fictional records. It confirms the app responds before stopping
its entire private process group, verifies no live process remains in that group,
and launches a fresh driver against the same temporary data directory. This
avoids WebKitGTK session teardown/reuse races and checks storage across a complete
process restart. It also checks that the application is running inside Tauri and that
private workspace data leaves the page when locked. The test password is randomly
generated and never printed. Temporary application data is removed on completion.

PDF destination replacement uses native select-all/delete keyboard events. The
test observes the empty field, types the new destination once and verifies the
complete value before clicking export. It never retries typing or export clicks,
and verifies the resulting file and its independently expected report contents.

Native date fields use the observed Linux widget typing format, followed by an
exact ISO value read before saving. No typing or saves are retried.

The 0.24.0 workflow also saves optional household/employment intake, a dated
appointment with time, an overdue task and completion/cancellation outcomes. It
checks second-client isolation, lock privacy, restart, encrypted restore and
missing-metadata recovery while retaining all previous financial/PDF checks.

The 0.25.0 workflow opens the saved overdue item from the practice dashboard
before completing it. It checks latest counts, closed history, name/kind/date
filters and exact client/item keyboard focus after restart and both recovery
paths, without adding a duplicate financial scenario/report workflow.

## Linux cloud environment

Build the frontend and native application with the repository's documented
toolchain first. A built application with embedded frontend assets is required;
this test does not start Vite. Provide an X display (a desktop or Xvfb),
the `WebKitWebDriver` matching the installed WebKitGTK runtime.

```bash
python3 scripts/desktop-smoke.py \
  --binary src-tauri/target/release/eisenbart-counseling-suite \
  --native-driver /path/to/WebKitWebDriver \
  --artifacts /tmp/eisenbart-desktop-smoke
```

If the cloud environment uses extracted system libraries, run this command with
the environment helper documented in the cloud setup instructions. On Linux the
script requires `DISPLAY`. Container restrictions may require an explicitly
configured WebKit sandbox override for this synthetic automated test. Such an
override is a testing environment setting and must not be used in the shipped
application.

## Windows

This automated test currently runs on Linux, where temporary XDG directories
isolate application data. Windows uses system known folders, so changing an
environment variable alone would not safely isolate an existing workspace.
Follow the [Windows manual acceptance checks](windows.md) for the compiled `.exe`
and installer. Linux native validation does not replace testing the Windows
installer and WebView2 runtime.

## Results and diagnostics

The command exits with `0` after all checks pass or `1` on failure. Successful
checks print concise `PASS` messages. The artifacts directory contains a native
desktop screenshot and driver logs; a failure screenshot is saved when the
driver remains reachable. Artifacts must be outside the repository and contain
only the test's fictional records. Ports default to `4444` and `4445`; use
`--port` and `--native-port` if needed. A missing executable, display, or occupied
port fails before the test changes any application data.

Read-only GET observations and element locators retry one transient native-driver connection closure;
typing, clicks, and saves are never replayed. Assertions remain required.

This test checks desktop behavior and persistence. It does not audit encryption,
prove the absence of outbound network traffic, or certify production security.

The harness centers visible controls before a native click to avoid WebKit partial-viewport coordinate errors. Unicode labels are preserved in XPath literals. Explicit read-only DOM/runtime observations, locator reads, and GET observations retry one connection closure; clicks, typing, financial writes, and general scripts are never replayed. Source-retention and acknowledgment checkboxes must be observed selected before submission.

The PDF checkpoint extends the same visible native workflow with a generated fictional text PDF, separate parser execution, damaged-file rejection, explicit transaction-line selection, page/line provenance, individual source approval, pending-expense exclusion, original retention, and backup/error-screen recovery of PDF decisions/history. The standard-library fixture generator is `scripts/synthetic_pdf.py`; it is a development helper, not part of the shipped application.

The 0.9.0 workflow also creates Casey Pattern with three fictional month-end
service payments ($10.01, $12.03, $10.01). It verifies pending exclusion,
merchant evidence, a monthly suggestion and the independent $32.05 total;
explicitly confirms a subscription; reverses an approval ($22.04 remains);
checks **Needs review**, reapproves without reviving confirmation, and explicitly
reconfirms. It also approves then withdraws a new fourth charge, verifies old facts still require review, and reconfirms. All three captured decisions must survive a complete process restart,
normal backup recovery and missing-metadata recovery, with client isolation and
unchanged planning inputs. Artifacts include `native-recurring-review.png`.

The 0.10.0 workflow adds Taylor Balance with a declared after-transaction CSV
balance column and $1,000.00 opening balance. It validates $500.01 income,
−$300.02 rent and −$0.01 fee against $1,500.01/$1,199.99/$1,199.98 balances;
records a wrong order and its explicit correction; verifies no automatic approval;
bulk-approves validated rows; corrects rent to −$300.01 and checks two +$0.01
approved discrepancies while original steps remain matched; then rejects rent
and verifies $500.00 approved movement and a $300.02 comparison difference.
Two immutable order decisions and all comparisons must survive complete restart,
normal backup restore and missing-metadata recovery. Screenshot:
`native-statement-balances.png`. Existing clients' totals remain unchanged.

The 0.14.0 workflow exports the original fictional assessment to new local PDFs,
verifies independently expected historical figures/evidence/review flags with
Poppler `pdftotext`, refuses overwrites without changing file bytes, and repeats
after restart and both recovery paths. Install `poppler-utils` for this development
check if `pdftotext` is unavailable; it is not an application dependency.

Build the standalone native executable **after** Cargo tests and Clippy, immediately
before the smoke run. Cargo integration tests can replace the debug application
path with a binary expecting the development server. A refused localhost page
means rebuild with `npm run tauri -- build --debug --no-bundle`; do not start a
web server to make this standalone desktop check pass.

The 0.15.0 workflow saves Alex's named planning scenario with explicit Avalanche
selection, checks the entered 3,250.00 income / 716.67 unallocated / 35.35 interest /
2028-06-30 savings milestone / 3,300.00 cash-flow closing separately from approved
1,820.02 activity, changes/reverts the active budget without rewriting/reactivating
the scenario, and repeats its PDF/identity/review/evidence checks after restart and
both encrypted recovery paths. Sources and plans remain isolated by client.

Viewport movement now uses a dedicated helper with fixed `scrollIntoView` code
and only an element/alignment argument. It retries one driver connection closure
while moving the viewport, including positioning before a click. It cannot type,
click, invoke application commands or replay a financial mutation. General scripts,
clicks, typing, approvals and saves still have no automatic retry.

The 0.16.0 workflow records Alex's client action plan from the current selected
assessment before editing/reverting his budget. It reviews and records three
priorities, counselor responsibility, recommendations, limitations and follow-up
without invented dates. Independently expected model payments are debt 100.00,
primary savings 250.00 and buffer 1,066.67; the captured original budget separately
retains 716.67. Client PDFs preserve approved historical cash flow 1,820.02,
Avalanche interest 35.35 and emergency completion 2028-06-30. Five client PDFs
cover current basis, changed historical context, restart, normal encrypted restore
and damaged-workspace recovery. Persisted plans inherit Needs review after plan
revision changes while steps/allocations remain immutable. Artifacts include
`native-client-action-plan.png` and `client-action-plan-*.pdf`.

The 0.17.0 workflow captures Casey's selected narrative note and latest invalidated
payment decision with original source bindings in a counselor report. A narrative
9999.99 remains separate from approved historical total 32.05 / cash flow -32.05.
A later note flags context without entering the saved report. Four counselor PDFs
verify before-restart, restart, encrypted restore and damaged-workspace recovery
evidence. Alex's planning report also captures only his selected note; client PDFs
exclude internal counseling evidence. Artifacts include
`native-counselor-report-evidence.png` and `counselor-evidence-*.pdf`.

The 0.18.0 workflow compares Alex's saved budget before model entry and after
restart/each encrypted recovery path. Unknown minimums stay unavailable; a 100.00
minimum yields needs 1,733.33 / future 600.00, standard targets 1,625.00 / 975.00 /
650.00 and custom needs target 1,950.00. Zero-based remaining stays 716.67. Invalid
percentages fail with assumptions retained; no saved budget or source value changes.
Temporary comparison inputs reset across client/restart boundaries. Artifacts
include `native-budget-methods-*.png`.


The 0.19.0 workflow compares a temporary combined proposal before recording Alex’s
saved planning assessment and after restart and both recovery paths. It changes
biweekly income, discretionary spending, debt base payment, emergency contribution
and an explicit bill date, checks exact side-by-side 3,466.67 income/983.34
allocation buffer/23.01 interest/February 2027 payoff/November 2027 savings date,
and confirms the saved model inputs and 3,300.00 dated closing remain intact.
Invalid precision is rejected with the proposal retained; a read-only reload resets
it. A fresh client mount has no temporary baseline or draft. The same complete run
retains all prior report, action-plan, source review and encrypted recovery checks.


The latest 0.19.0 full run passed those four new feature checks but exited 1 when
WebDriver closed a POST connection while opening Casey’s older counselor-report
panel after damaged-workspace recovery. Its final evidence PDF/dashboard steps
and whole-run completion are unverified for this checkpoint. See development
status for logs and the three-attempt boundary. The intended click selector is
now included in error diagnostics; uncertain clicks and financial mutations are
never automatically retried. Do not treat individual PASS lines as a successful
full run.


Direct native transport is now the default. It uses the same
`webkitgtk:browserOptions` binary/arguments and Tauri automation environment flags
as tauri-driver 2.1.0, removing only that intermediary's pooled forwarding hop.
`--transport tauri --driver /path/to/tauri-driver` retains the older path for
transport diagnosis. Assertions, UI interactions and no-replay rules are unchanged.
The pre-stage 0.19.0 full workflow passed with direct transport (exit 0), resolving
the previously documented final report-panel disconnect.

The 0.20.0 workflow additionally saves Alex’s reviewed combined proposal, explicitly
selects it in a format 4 report, exports its selected baseline/proposed PDF and
verifies it after restart and both recovery paths. Later budget revisions flag
both records Needs review while their 983.34 proposal buffer and separate 1,820.02
approved report figures remain frozen. All historical report/client-plan checks
continue in the same full workflow.

The 0.21.0 workflow retains every earlier comparison/report check and adds an
explicit 300.00 November extra plus a 100.00 goal contribution. It verifies
833.34 funding buffer, 12.20 interest, December payoff/goal completion, invalid
extra precision with retained input and unchanged live contributions/payments.
A second immutable scenario and selected report survive restart and both recovery
paths with original milestones and Needs review flags. Four additional PDFs
verify added-payment/goal assumptions separately from approved operating activity
1,820.02 and unchanged dated closing 3,300.00. Artifacts include
`native-scenario-controls-*.png` and `scenario-controls-*.pdf`.

The 0.22.0 run extends the existing extra-payment/goal proposal and its four
selected-report exports with 75.00 monthly child care. It checks budget-only
758.34 buffer / 5,124.99 target / December 2027 emergency completion before
explicitly selecting the payment account and dates, then checks 3,225.00 dated
closing. Invalid dates remain editable. All earlier 833.34/12.20 extra/goal checks
run before adding the expense; the original saved proposal and older reports
remain separately covered. The integrated expense record/report and explicit
bill assumptions must survive restart and both recovery paths. Artifacts:
`native-recurring-expenses-*.png` and `recurring-expense-*.pdf`.

The 0.23.0 integrated proposal extends the existing expense/extra/goal checks with
800.00 biweekly income for November only. It checks -975.00 monthly funding,
December normal income/buffer 858.34, unchanged dated closing before explicit
paycheck selection and 1,825.00 closing afterward. Reversed periods stay editable.
The original baseline/normal models remain unchanged. Four integrated PDFs and
restart/both recovery paths retain monthly assumptions, deficits, recovery and
separate 1,820.02 approved operating activity. Older comparisons/client plans
remain in the same complete run. Artifacts: `native-temporary-income-*.png` and
`temporary-income-*.pdf`.

## Selected report intake (0.26.0 source)

The native synthetic saved-planning-report flow explicitly previews intake and
selects household/employment only. It changes an omitted address and appointment
outcome without invalidating the report, then changes/reverts selected employment
and verifies permanent Needs review. Restart, encrypted restore and storage-error
recovery retain original selected context. Internal PDF text includes selected
household/employer/disclosure and excludes later/omitted values; client action-plan
PDFs exclude internal intake. Existing financial/PDF expected amounts remain.
Artifacts: `/workspace/shared/desktop-smoke-intake-report` for this checkpoint.

## Complete page structure (0.27.0 source)

The native workflow now opens all twelve main navigation destinations, checks
focused document/review/transaction separation, reads original saved planner
values, switches client context to an empty plan and back, opens a labeled OCR
future area and branding page, and returns to the complete record. Settings
backup is reached by name and its dedicated Backup & recovery area. Existing
financial, PDF, restart and both encrypted recovery paths remain enabled.
Artifacts use `/workspace/shared/desktop-smoke-prototype` for this checkpoint.

## Document collection (0.28.0 source)

The native run adds two requests, saves missing status, archives one, records an
explicit receipt and original source/account link, checks read-only original
history and switches clients to verify isolation. Initial, restart, encrypted
restore and error-screen recovery repeat receipt, archive, source/history and
client-isolation assertions. Prior financial/PDF assertions remain enabled.
Artifacts: `/workspace/shared/desktop-smoke-collection`.

The account-management native workflow adds a standalone account, renames it,
archives/reactivates it, checks original/current metadata and all four history
versions, switches to an empty second client, and rechecks persistence after
restart and both encrypted recovery paths. No new installer is needed.

The firm-branding native extension runs after the existing financial/recovery
suite. It saves a synthetic local PNG/name/contact/disclosure, records a branded
assessment and linked client plan, changes current contact details, restarts,
backs up/restores and exports the historical report. It checks retained logo/
original contact, latest current settings, history and review flags through
visible controls. Product branding and mandatory financial warnings remain.

## Financial completeness diagnostics (0.31.0 source)

After the full financial, restart, encrypted restore and branding workflows,
native automation opens the working completeness page with an explicit month,
checks original account names, calendar-day coverage and the finalization block,
then opens a historical report through the working final-report checklist. It
checks the captured two-month period, changed-context flag, versioned checklist
and inventory block. The previously exported historical internal PDF must contain
the same validation disclosures. All prior financial assertions stay enabled.
Artifacts use `/workspace/shared/desktop-smoke-completeness-final` for this stage.
The OCR-area check selects its named tab rather than assuming its position among
working and planned subpages.

## Saved completeness reviews (0.32.0 source)

After all prior financial/recovery/branding/coverage checks, native automation
saves expected account periods and a missing loan, selects coverage evidence,
opens numbered history read-only, captures version 1 in an internal assessment,
amends to version 2, restarts and restores an encrypted backup. It verifies
current and original versions, retained report figures, the later-review flag
and a PDF using original missing-information text. Artifacts:
`/workspace/shared/desktop-smoke-coverage-review`. No direct IPC or financial
assertions are bypassed.


## Final counselor reports (0.33.0 source)

The full workflow first checks that an existing incomplete household is blocked.
A separate fictional Jamie Complete client imports an authorized retained source,
approves two rows, verifies classifications and period-end complete balance facts,
saves a matching completeness review and records an immutable assessment. Native
controls review readiness, record counselor notes/confirmations and approve the
final report. The test checks selector draft protection and exports a final PDF
with independently known USD 799.99 operating cash and USD 1,799.99 net worth.
After a new completeness version, restart and encrypted backup/restore, the same
approval ID/digest and original notes remain, with explicit current review flags
in the screen and historical final PDF. Screenshots include blocked, confirmation,
approved and restored approval views. All earlier assertions remain in the run.


## Credit-card CSV review (0.34.0 source)

The full native workflow adds five fictional card rows through explicit mapping
and individual decisions. Independent expected results are USD 87.08 card spending,
USD 50.03 repayments, USD 37.05 debt movement and USD 1,037.05 closing owed. It
checks original versus one-cent corrected arithmetic, rejection/history with USD
-12.93 remaining net spending, unchanged bank figures and card-related bank-report
review flags. Restart, encrypted backup/restore, source identity, historical
amounts and second-client isolation are verified. Screenshots record reconciled,
correction-history and restored views. All prior assertions remain.

The 0.35.0 extension exercises visible durable card account creation, explicit
statement and repayment links, independently checked cash/spending/payment totals,
format-8 assessment/PDF capture, newer-approval invalidation, rename/archive,
process restart, full encrypted recovery and historical report/client isolation.
No direct IPC mutations are used. Earlier native workflows still run in full.

The 0.36.0 native extension adds recorded card inventory/applicability, exact role
classification, an existing-debt comparison without doubling household debt,
a one-cent verified balance discrepancy, read-only history, format-9 report/PDF,
blocked final approval, restart, encrypted recovery and client isolation. All
mutations use visible native controls; no direct IPC shortcuts are used.


The 0.37.0 extension explicitly renews the stale repayment review, records a known
required payment, updates both bank/card completeness versions and captures all
three contexts in a format-9 parent. The synthetic repayment original is explicitly
authorized for retention at import so the final source gate can pass. It checks
independent USD 1287.10 spending, 712.91 income less spending, 749.96 bank cash and
1037.05 existing debt; missing card confirmation cannot approve. It exports a v2
final PDF, then verifies original figures/identity after a later review, restart,
encrypted recovery and client changes. Earlier v1 approval and full workflows
remain in the same run. No direct IPC mutations or installer are used.

The 0.38.0 extension opens Transactions → Activity history over the existing
recovered synthetic records. It checks stable forward/backward pages, original
and corrected card evidence, date/source/status filters, both bank transfer legs,
exact repayment approval navigation, renamed archived account identity, empty
results, client isolation, lock privacy and another complete process restart.
The fixture first imports 31 pending fictional rows through Documents so a second
page is guaranteed, then verifies paging leaves review counts unchanged. All
activity-page actions are read-only searches through visible controls.

The 0.39.0 extension opens Reports → Print & presentation over existing recovered
records. It exports real summary-first internal/v1/v2 PDFs, checks first-page
identity/period/financial figures and review flags, the original detailed option,
full appendix, exact combined cents/debt once, no-overwrite, lock privacy, full
process restart, original identity and encrypted recovery/client isolation. PDF
page text and rendered images are inspected separately. It does not submit jobs
to a Windows printer or create an installer.

The 0.40.0 extension retains every earlier native assertion. Each presentation
export checks the actual saved path, readable-file lifetime notice and truthful
non-Windows fallback. Changing layout/client must clear the saved-file shortcut.
It captures `native-saved-pdf-print-workflow.png`. Rust/UI fixtures separately
exercise request scope/expiry/bytes/context/session/restore and mocked Windows
acceptance, cancellation and launch errors. Linux smoke and Windows source tests
do not verify an installed viewer, physical printer or print spooler; see
[Windows acceptance steps](windows-pdf-print-workflow.md).

The 0.41.0 extension retains all prior workflows, then imports a fictional bank
source, explicitly approves two independent expenses, pages 31 missing-category
records, displays current approval evidence and fills/discards an unsaved Food
draft. It explicitly saves the category while retaining its unapproved status,
checks unchanged approved financial totals, original history/reason, lock, full
process restart, encrypted recovery and client isolation. Screenshot:
`native-category-suggestions.png`. No installer is created.

The 0.42.0 extension retains all prior assertions. It creates a scoped expense
rule, edits its category to version 2, prepares a protected unsaved draft and
explicitly saves it unapproved. It checks exact original-version application
history, archive/reactivation through versions 3/4, unchanged approved totals,
lock, full restart, encrypted backup recovery and client isolation. Screenshot:
`native-category-rule-history.png`. All mutations use visible native controls
and fictional fixtures. No installer is created.

The 0.43.0 extension retains all earlier assertions. It saves 31 fictional session
notes through visible controls, explicitly corrects one with a reason, reads its
unchanged original/history, pages 32 note events, searches a single correction
and opens the existing note section. New report evidence offers the corrected
head and omits the superseded original. Full lock/restart/encrypted recovery and
a second client's empty search validate persistence and isolation. Screenshots:
`native-note-correction-history.png` and `native-client-history-timeline.png`.

Native driver readiness notes: existing employer replacements use an actual
click and select-all/delete/type sequence, checking the effective enabled state
before typing. Saved-report IDs are read only after their directory refresh
finishes and the selector contains the new selection. These are observation/focus
guards; they do not replay uncertain writes, script application data or remove
any workflow assertions.

## Workspace password changes (0.44.0 source)

The full native workflow ends with wrong-current-password rejection, empty
submitted fields, reviewed retained-copy disclosures, successful change/lock,
old-password rejection, new-password unlock and complete restart. It creates
backups before and after rotation, rejects each other-generation password,
restores both using their original passwords and checks original notes and
unchanged older backup bytes. The backend fixture separately compares exact
source/report values and digests. No password is included in diagnostics.
Screenshot: `native-workspace-password-safeguards.png`.

After verifying termination of its own private process group, driver restart
waits up to 75 seconds for closed TCP ports to be released. Initial occupied-port
checks still fail before creating test data. This addresses observed WebKit
TIME_WAIT timing; UI writes, clicks and assertions are never replayed.

## Configurable inactivity (0.45.0 source)

Before the password-change extension, the full harness saves a one-minute
policy, inspects immutable history and waits for the real Rust timer with only
read-only DOM observations. No product clock/timeout is overridden. It checks
private UI clearing, unlock/full restart, encrypted recovery of the originally
captured one-minute policy after a later change, and original client notes. It
returns the policy to 15 minutes for the remaining workflow. Screenshot:
`native-inactivity-policy.png`. The focused wrapper uses a separate synthetic
workspace and the same visible-control check function.

## Client workspace redesign — 0.46.0

The global navigation now has Dashboard, Clients and Settings; all twelve client
sections live inside stable-ID in-app tabs. The native page object routes legacy
financial fixture intents through these actual UI controls and separate review
subtabs. It never invokes app IPC or scripts financial writes. Negative/locked
DOM observations remain read-only, and source/report/math assertions are retained.

The focused redesign workflow creates real synthetic records through the app,
checks the directory and all twelve sections, saves independently expected budget
and debt inputs, checks the $200/$0 mismatch and unknown balances, cancels and
accepts the actual in-app discard dialog, verifies isolation/tab deduplication/MRU
close, inspects an 800px window and tests locking plus a full offline restart.
Keep WebKit sandboxing enabled. See development status for actual run outcomes.

The 0.47.0 workflow adds a separate Quinn Analysis client with synthetic bank
statements. It checks pending exclusion, explicit individual approvals, source-
confirmed subscriptions and a saved discretionary budget. It classifies eight
expenses via the Spending Analysis Type control, independently checks $1,160 gross,
$1,140 net, $160 discretionary, six small purchases totaling $60, a $10 reviewed
subscription and $60 above the real target. A $100 change to Necessary preserves
gross/net, updates necessary to $1,100 (94.83%) and discretionary to $60 (5.17%),
removes the discretionary Dining bar and budget flag, and preserves both purposes
and original imported values in History. It exercises the actual discard dialog,
table-only filtering, thirteen client sections, desktop/800px page overflow,
private lock clearing, full process restart, encrypted backup restoration of
classifications/reasons and second-client isolation. Screenshots:
`native-spending-desktop.png`, `native-spending-history.png`,
`native-spending-narrow.png`. Results are recorded only after the run finishes.


## Transaction-focused extraction and review (0.48.0)

The complete workflow now also creates Morgan Extraction with fictional savings
and card PDFs. It excludes contact text, fine-print pages, totals and running
balances from the checklist; asserts confident preselection and manual exclusion;
checks exact selected sums and imported row counts; saves source-bound purposes
in separate bank/card review pages; verifies automatic combined Spending Analysis,
classification history, process restart, encrypted recovery and client isolation.
It checks the unclipped inspector and 800px layout. Screenshots are
`native-extracted-candidates.png` and `native-extracted-spending.png`.

The PDF worker can insert extracted blank lines. The harness locates transactions
by their actual text and reads their extracted locators, rather than assuming
printed line positions. Native click failures report read-only geometry and never
replay a click or financial write. Build using `--debug --no-bundle` for this stage;
no installer is created unless explicitly requested.

## Household management (0.49.0)

The full driver now also creates two dedicated fictional household clients and a
limited-information third member. It tests the ten-section household workspace,
explicit sharing permissions, $2,400 rent split into $600/$900/$900, a full payer
outlay, internal settlement, exact shared goal contributions, and a saved
hypothetical rent change that does not alter active agreements. It explicitly
reconciles a $600 individual budget share and verifies the individual plan.
Before saving, a hybrid preview also verifies a $500 fixed-only participant
with a `0.0` remainder weight and $950/$950 equal remainder shares. Overview
checks distinguish planned necessary/discretionary obligations from source actuals.

Native clicks switch between individual and household tabs while a household
editor remains open, retain its draft and section, and exercise explicit Keep /
Discard. Household editors leave global tabs/navigation accessible; keyboard
users can return to the tab strip from the editor. The report checkpoint exports
an immutable household PDF with independently expected figures and a shared goal,
refuses overwrites, checks an 800px layout, clears private state on lock, restarts
the complete process, and verifies encrypted backup recovery of the household.
Screenshots include `native-household-overview.png` and
`native-household-narrow.png`. Fixtures and input gestures are synthetic; typed
replacements are observed exactly before financial submissions.

The full run reaches households with an already populated, horizontally scrolling
workspace tab strip. Native tab actions wait for the actual hit target before
one click; they never retry financial submissions. A separate synthetic probe
also checks offscreen tab targets with eight clients and a household. The sticky
tab header contains its own flex-start scrolling row to preserve correct hit
regions in native WebKit.

Use the same standalone `--debug --no-bundle` build and native driver prerequisites
above. Do not run Cargo while the native application/driver processes are active.
These checks create no installer and establish no Windows installed acceptance.

## Unified transaction review and original inspection (0.50.0)

The full driver adds a fictional Taylor Review client after extraction acceptance.
It imports retained two-page bank/card PDFs through the real bounded parser,
approves each transaction using existing forms and then checks that opening
**Review & reconciliation** defaults to **All transactions**.
It checks one row per record, mixed same-day chronology, oldest-first and combined
account/type filters. Repayments do not inflate the exact $250 purchase total.

Native inspection uses **View Source Document** to open the bank original on its
actual recorded page 2, verifies nonblank canvas rendering, navigates pages and zoom, and saves explicit institution,
last-four, debit-card and posting-date details. Credit/debit/unknown purchase shares
are independently $100/$100/$50 (40%/40%/20% of dollars), while each method
supplies one purchase (33.33% of count). Closing the source removes the split
immediately with the transaction table available. Rejecting a purchase changes
the now-discrepant bank statement’s eligibility; only the matched card source
contributes (100% of eligible purchase count) until the bank source is reconciled.
Financial decisions use the existing embedded bank/card forms and update Spending Analysis. Earlier supplemental
details become inactive on a new approval and remain visible in original history.
The script rejects/confirms a synthetic duplicate, filters that confirmation,
then reapproves after clarification and verifies the restored eligible total.

Screenshots: `native-unified-transactions.png`, `native-unified-original.png`,
`native-unified-payment-shares.png`, `native-unified-narrow.png`. An 800px check stacks source/evidence without document
wide overflow. Lock removes private UI, a complete restart preserves saved data,
and encrypted backup recovery validates source/detail history and displays the
actual PDF again. A second client has neither rows nor leaked institution data.

Use the same compiled standalone `--debug --no-bundle` build and real Tauri native
driver as previous stages. Do not run Cargo while the application/driver are active.
The full prior regression is retained. No installer is produced; Linux checks do
not establish installed Windows acceptance or certify commercial readiness.

The final focused refinement extends the same scenario to nine current rows by
adding bank loan proceeds/payment and a refund/fee pair. The unchanged $250
purchase split excludes them. ACH credit/debit and external/reversal labels must
preserve movement/refund kinds; they must never require invented income or
purchases. Inspection clicks wait for the actual unobstructed hit target once;
a refreshed original canvas must finish painting before subsequent test gestures.
No financial submission is replayed. Execution results distinguish the complete
pre-refinement regression from final focused acceptance on the corrected binary.


## Financial health and behavioral insights (0.51.0)

The existing native driver now includes `check_financial_health` after unified
review. A synthetic Morgan Health client imports a retained CSV and approves its
seven real transactions through existing forms. Missing inventory first withholds
the score. Verified income/purposes, period-end balances and saved coverage yield
95/100, $1,800 net spending, 51.66% necessary expenses and preserved refund/principal
boundaries. The exact balance evidence remains inspectable.

The script enters existing budget/debt/savings/calendar plans, discusses 20% of
$250 net Dining activity, and verifies $50/month and $600/year impacts. Explicit
line mapping uses the existing combined scenario table, loan payoff and dated
calendar calculations, with original saved plan inputs unchanged. The current
Spending Analysis workflow changes Dining to Necessary; live amounts update,
the score waits for renewed coverage, then becomes 85/100 with 2.70 reserve months.
At 800px, components stack and tables scroll within the page. Private health data
clears on lock; full restart and encrypted recovery recompute the same score,
while another client has no transaction or balance evidence from this client.

Screenshots: `native-financial-health-score.png`,
`native-financial-health-reduction.png`, `native-financial-health-narrow.png`,
`native-financial-health-restored.png`. Final focused health acceptance passed
with exit 0, all three checkpoints and the completion marker in
`/workspace/shared/desktop-smoke-financial-health-final`. The score, reduction and
narrow images were visually inspected; source/binary hashes stayed unchanged.
The current unified-review checkpoints passed earlier in the combined first run,
which later stopped on a health-fixture locator error. Final harness fixes cover
CSV record numbering, existing save-status elements and explicit coverage renewal;
no uncertain financial write was replayed. The old complete 117-checkpoint native
regression was not repeated for 0.51.0. Development status and
[the acceptance record](financial-health-assessment.md) distinguish all executions.
No installer or Windows installed-acceptance claim follows from these checks.


## Complete 0.51.0 consolidation acceptance

The entire tracked native script passed with **119 checkpoints**, its completion
marker and exit 0 in `/workspace/shared/desktop-smoke-051-consolidation-fourth`.
This is fresh full-regression evidence beyond the earlier focused 0.51.0 runs.
It includes all planning, reports, normal/error-screen recovery, accounts, card
review/final approval, classification/history, spending/extraction, unified original
PDF inspection, health, household, inactivity and password-change workflows.
Score and 800px images were visually inspected; runtime/binary hashes stayed fixed
and native driver ports closed before later Cargo commands.

Three earlier attempts exposed old bank-queue/default-view assumptions after 9,
13 and 105 checkpoints. Explicit legacy queue selection and unified-default
navigation checks corrected the harness. Original financial/history/isolation
assertions stayed in place, uncertain writes were not replayed, and each rerun
used a fresh fictional workspace. A frontend acceptance-test guard wait changed
during the final native run; no runtime/build input or accepted binary changed.
See [the consolidated source/CI/recovery evidence](branch-0.51-review.md) for all
outcomes and limits. No installer or Windows installed/offline claim follows.


## Workflow integration audit acceptance (0.51.0)

The entire script passed again with **119 checkpoints**, its completion marker
and exit 0 in `/workspace/shared/desktop-smoke-workflow-audit-final`. The selected
intake checkpoint now also checks the assessment directory: unrelated edits leave
it current, selected changes flag it, and reverting selected values never removes
the flag. Existing approval, calculation, plan/scenario/report, original PDF, card,
household, lock/restart and both recovery paths stayed enabled. Intake and 800px
Spending Analysis screenshots were visually inspected. All tracked non-documentation
inputs and the binary stayed fixed; native driver ports closed afterward.

An initial launch omitted the existing cloud-native runtime wrapper and failed
before any workflow action. The fresh full run used `cloud-native-run.sh` with
private D-Bus/display and the existing synthetic-test-only sandbox exception.
No application safeguard or assertion was disabled and no uncertain financial
write was replayed. See [the handoff matrix and evidence](workflow-integration-audit.md).
No installer or Windows installed/offline acceptance claim follows.
