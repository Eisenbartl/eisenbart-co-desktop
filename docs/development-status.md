# Development status

## Stage 3I: recurring-expense proposals and explicit dated bills — 0.22.0 source

New necessary/discretionary recurring expenses now recalculate combined budget,
debt capacity, emergency targets and savings affordability. Optional explicit
account/first/last/second-day inputs add estimated bills to the proposed dated
forecast with the same amount/frequency. Existing obligations stay in effect;
no live plan, approved fact or agreement changes. Saved scenarios and selected
reports retain classifications, payment bindings, canonical inputs and outcomes.
Earlier empty-extension records/warnings remain compatible; schema 16 is unchanged.
See [combined-scenario-comparisons.md](combined-scenario-comparisons.md) for scope,
exact examples, calendar limits and monthly versus dated forecast assumptions.

The integrated native example retains earlier extra-payment/goal controls and
adds 75.00 child care: buffer 833.34 → 758.34, emergency target 5,124.99 with
December 2027 completion. Explicitly scheduling the November bill changes closing
3,300.00 → 3,225.00; debt interest stays 12.20, goal completion December 2026 and
approved operating activity separately 1,820.02. Budget-only additions preserve
dated closing. A separate weekly 12.01 example verifies monthly 52.04 versus four
actual payments totaling 48.04. Unknowns, zero, deficits, limits and out-of-period
schedules remain explicit. Annual expenses are budget-only.

| Check | Result |
|---|---|
| Locked setup | Passed; existing environment configuration/dependencies reused |
| Focused backend | Three new comparison checks and one saved-record/PDF/recovery check passed |
| Focused interface | 14 comparison/control tests passed, including three new expense checks |
| Production frontend / complete interface suite | Passed build and 113 tests |
| Complete Linux backend / PDF workers | Passed: 157 library + 2 PDF-process tests (159 total) |
| Formatting / Clippy, warnings denied | Passed |
| Standalone native build / complete workflow | Standalone debug build passed; complete workflow running |
| Windows source CI | Validation in progress; installer not requested |
| Clean Windows installation/offline release acceptance | Not run |

The existing integrated native proposal/report workflow is extended rather than
creating another parallel report workflow. Prior extra/goal assertions run before
the expense is added; older proposals and client reports remain separate. Full
regression checks still follow repository requirements. Feature details are
consolidated in the existing combined-scenario document.

Validation logs use `/workspace/shared/recurring-expense-*.log`; native artifacts
will be in `desktop-smoke-recurring-expenses`. The original extra/goal warning
was separately checked byte for byte against the prior source for proposals
without new dated expenses. Final results and checkpoint details follow validation.

Next priority: **temporary income-reduction scenarios** with explicit period scope,
then remaining financial-model/report requirements. Seven major roadmap areas
remain; this checkpoint does not complete the master specification or commercial
release acceptance. No new installer is created during feature work.

## Stage 3H: extra debt payments and savings-goal proposal controls — 0.21.0 source

Combined proposals now support additive dated debt extras and explicit monthly
contribution changes to existing savings goals. Rust validates original owned
baselines and all four versions, retains saved debt events/goal targets/reserves,
recalculates existing exact-cent models and returns canonical changes. Goal
contributions, completion dates, forecast status and target applicability appear
side by side. Saved scenarios and selected report PDFs retain those assumptions,
including explicit added-payment/changed-goal descriptions. No live plan,
approved fact, agreement or dated cash-flow schedule is changed.

Schema 16 and existing payload/calculator versions remain compatible: new arrays
default empty and are omitted when empty, preserving legacy field order and the
original six warnings. Extended proposals append a funding/commitment warning.
Extras follow the shared strategy budget, do not target a named account, and
reserve funding conservatively even after an early projected payoff. Unknown
contributions remain unknown; zero is explicit. See
[scenario-payment-goal-controls.md](scenario-payment-goal-controls.md).

Independent zero-interest example: income 3,250.00, spending 1,200.00, debt 100.00,
emergency 200.00 and goal 50.00 yield a 1,700.00 first-month buffer. Adding a
150.00 November extra and changing the goal to 100.00 yields 1,500.00. Retaining an
existing 25.00 December extra, the 1,000.00 loan finishes in July instead of August;
the goal's remaining 200.00 finishes in December instead of February. Target and
reserve remain unchanged. The native fixture's higher 400.00 budget debt reserve
produces 983.34 → 833.34 when a 300.00 extra makes model debt 500.00 and the goal
changes to 100.00; proposed interest is 12.20 and payoff December 2026. Approved
operating activity remains separately 1,820.02 and dated closing 3,300.00.

| Check | Result |
|---|---|
| Locked setup | Passed; existing reusable setup remains sufficient, no new environment configuration or dependency |
| Production frontend build / interface suite | Passed build and 110 tests; three added control/save/unknown/reset/limit checks |
| Focused backend checks | Nine comparison and nine saved-record/report/recovery checks passed, including four new tests |
| Complete Linux backend / worker suite | Passed: 153 library + 2 PDF worker tests (155 total) |
| Formatting / Clippy, warnings denied | Passed |
| Native compile / complete workflow | Passed standalone debug build and complete direct native run, exit 0: four extended comparison/PDF checks, restart, normal encrypted restore and damaged-workspace recovery; all prior workflows retained |
| Windows source CI | [Run 38664552864](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38664552864) passed: production frontend build, 110 interface tests, 154 library + 2 PDF worker tests (156 total); installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

Initial focused checks exposed two fixture errors: the absent-plan test needed a
saved budget to reach the intended validation branch, and the frontend mock
needed canonical returned money strings rather than echoing unnormalized input.
Both fixtures were corrected without changing assertions, timeouts or financial
calculations. Their first failure logs remain available; focused reruns passed.

Logs under `/workspace/shared`: `scenario-controls-setup.log`,
`scenario-controls-ui.log`, `scenario-controls-ui-tests.log`,
`scenario-controls-ui-focused.log`, `scenario-controls-backend-focused.log`,
`scenario-controls-store-focused.log`, `scenario-controls-backend.log`,
`scenario-controls-fmt.log`, `scenario-controls-clippy.log`,
`scenario-controls-final-comparison.log`, `scenario-controls-final-compatibility.log`,
`scenario-controls-ui-final.log` and `scenario-controls-native-build.log`. Final
comparison/recovery tests were repeated after the last warning/PDF validation
wording adjustment. No arithmetic or model limits changed.
The two initial focused failures have the `-first.log` suffix. Complete native
results are `scenario-controls-native.log`/`.exit`; artifacts are in
`desktop-smoke-scenario-controls`. Four `scenario-controls-*.pdf` exports preserve
833.34, 12.20 interest, December payoff/goal completion and separate approved
1,820.02 operating activity. The earlier four selected-proposal PDFs and all
prior report/client-plan/source/recovery checks also passed in the same run.
Rendered PDF inspection confirmed readable added-payment/goal descriptions,
scenario identity and comparison figures; the native screenshot confirmed the
new goal comparison rows and 500.00 debt reserve.

Application source checkpoint:
`381fc8a6db58a7c2601f9d283c9e991ddd82fbe3`. Linux full-suite checks passed before
the final explanatory warning and defensive PDF goal lookup adjustment; affected
comparison and saved-record/PDF/recovery checks were repeated successfully on
this final source. Formatting/Clippy also passed, followed by the standalone
native build/run.
Windows source validation completed successfully on that exact commit in
[run 38664552864](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38664552864),
including the Windows-specific library check. Logs under `/workspace/shared`:
`scenario-controls-windows-validation.log`, `scenario-controls-windows-job.log`
and `scenario-controls-windows-result.json`. No installer was built or uploaded.
Internal source/full-history checkpoints are
`checkpoints/eisenbart-scenario-controls-source.tar.gz` and
`checkpoints/eisenbart-scenario-controls.bundle` under `/workspace/shared`.
Final documentation updates do not change the validated application source.
The final source archive is checked against all 156 tracked files byte for byte;
the bundle retains complete history. Clean Windows installation/offline release
acceptance remains a separate pending requirement.

Next priority: **new recurring-expense proposals**, then temporary income changes,
with exact monthly/datetime scope and unchanged historical facts. Seven major
roadmap areas remain; this checkpoint does not finish the original specification
or establish commercial readiness. No installer is built during feature work.

## Stage 3G: saved combined scenarios and report selection — 0.20.0 source

Reviewed combined proposals can now be recorded independently of live plans and
reopened as immutable baseline/proposed comparisons. Records retain client/time,
all four baseline versions, canonical changes/assumptions, complete model results
and warnings. Later live-plan revisions flag history Needs review without
rewriting values. Counselor assessments can explicitly select a current saved
scenario by ID/digest, capture its proposed model set and export selected
comparison context and milestones as an offline internal PDF. Approved activity
remains separate. See [saved-combined-scenarios.md](saved-combined-scenarios.md).

Schema 16 adds append-only owned scenario records and compound report links. A
scoped immediate transaction checks history count and baseline versions, performs
Rust calculations, and records payload/digest/audit atomically. The count guards
uncertain save responses; mutations are never automatically retried. Read/recovery
recompute captured models and verify scope/metadata/digest/arithmetic/linkage.
Failed audit, stale context or invalid selections leave no partial record/link.
Format 4 reports support selected proposals and optional session/payment evidence;
formats 1–3 and existing client agreements remain unchanged. Unknown projections
stay unavailable; unsupported selected repayment projections cannot enter reports.
No dependency or installer is introduced.

The earlier native blocker is resolved: a direct WebKit driver connection uses
the same binary capabilities and automation flags as tauri-driver, removing its
pooled forwarding hop. The complete pre-stage 0.19.0 workflow passed once, exit 0,
including the formerly interrupted final report check and both recovery paths.
Assertions and no-replay rules remain intact; the older forwarding mode stays
available only for diagnosis. Its previous failure logs are preserved.

Independent backend example: income 3,250.00 → 3,466.67; buffers 1,700.00 →
1,816.67; proposed snowball interest 23.01/payoff February 2027/emergency
completion May 2027, while approved report activity stays 0.00. Native data has
budget debt reserve 400.00, preserving 766.67 → 983.34 buffers and 35.35 → 23.01
interest, with November 2027 emergency completion and 3,300.00 dated closing.
The selected report retains separate approved operating activity 1,820.02.

| Check | Result |
|---|---|
| Locked setup / production frontend build | Passed |
| Interface suite | 107 passed; five new history/save/selection/stale/client-isolation checks plus prior financial/private-data guards |
| New backend checks | Eight scenario store/report/PDF/recovery/upgrade/corruption/audit/limits checks and nine migration-focused checks passed |
| Complete Linux backend / worker suite | 149 library + 2 worker tests passed (151 total); new scenarios, report/PDF, recovery, schema 1–16 compatibility and all earlier financial workflows |
| Formatting / Clippy, warnings denied | Passed |
| Native compile / complete new workflow | Passed standalone debug build and complete direct native run, exit 0: saved scenario/report/PDF selection, unchanged live/approved data, historical flags, restart and both recovery paths, with all previous workflows |
| Windows source CI | [Run 38648010362](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38648010362) passed: production frontend build, 107 interface tests, 150 library + 2 worker tests (152 total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

Initial complete backend validation found 15 legacy migration fixtures that still
held schema 16 tables while declaring an older version. Fixtures now remove both
new tables before simulated downgrades; collision/rollback, original-data and
append-only assertions are unchanged. The failed run is preserved separately.
The frontend global-lock fixture now mocks the added history read, and an existing
statement-review fixture awaits its asynchronous mount before editing, matching
the file’s established setup helper. No financial assertion or timeout is relaxed.

Native verification created four selected-scenario PDFs (initial/restart/normal
restore/damaged-workspace recovery), preserving 766.67 → 983.34 buffers and the
separate 1,820.02 approved report basis. Saved comparisons reopened with original
versions/outcomes and Needs review after live budget changes. All prior source,
report, agreement, lock and recovery workflows passed in the same complete run.
Rendered PDF inspection confirmed readable selected identity, comparison values,
Chicago time, preserved warnings and separate planning assumptions.

All application checks validate source commit
`02dfd5b83c2104c0796c9d71176e584235e5b5b5`. Windows source CI completed successfully
on that exact commit; it includes the Windows-specific library check. The final
checkpoint only updates this verification documentation. No installer was built,
uploaded or downloaded, and clean Windows installation acceptance remains open.

Logs under `/workspace/shared`: `saved-scenarios-setup.log`,
`saved-scenarios-baseline-native.log`/`.exit` (complete pre-stage check),
`saved-scenarios-ui.log`, `saved-scenarios-ui-tests.log`,
`saved-scenarios-backend.log`, `saved-scenarios-new-tests.log`,
`saved-scenarios-migration-focused.log`, `saved-scenarios-fmt.log`,
`saved-scenarios-clippy.log`, `saved-scenarios-native-build.log`,
`saved-scenarios-native.log`/`.exit`, `saved-scenarios-windows-validation.log`,
`saved-scenarios-windows-job.log` and `saved-scenarios-windows-result.json`.
The initial fixture failure run is
`saved-scenarios-backend-first.log`. Native artifacts are in
`desktop-smoke-direct-baseline` and `desktop-smoke-saved-scenarios`.
Source/full-history checkpoints are `checkpoints/eisenbart-saved-scenarios-source.tar.gz`
and `checkpoints/eisenbart-saved-scenarios.bundle` under that same shared directory.

Next priority after this checkpoint: **further scenario proposal controls**,
including extra payments, new recurring expenses, goal contributions and temporary
income changes, preserving exact calculations and immutable report provenance.
Seven major roadmap areas remain; this advances financial models/reports without
completing the original specification or establishing commercial readiness.

## Stage 3F: temporary combined scenario comparisons — 0.19.0 source

Counselors can now explicitly load all four saved models and compare them side by
side with a temporary proposal. Existing budget amounts/frequencies, debt base
payment and strategy, primary emergency contribution, and explicit dated-entry
amounts/dates can be adjusted. The comparison shows allocation buffers, repayment
cost/payoff/status, savings target/completion/status and dated closing/warning
counts, with canonical changes and both sets of model limitations. Scratch inputs
reset across clients/locks/restart and live-plan edits. Historical scenarios,
reports, client agreements, approved facts, live plans and audit events remain
unchanged. See [combined-scenario-comparisons.md](combined-scenario-comparisons.md).

Rust reads a scoped atomic baseline, checks all four expected versions and
canonical model calculations, validates bounded indexed input-only proposals and
recalculates shared budget/debt/savings/cash-flow contexts in order. It preserves
exact cents/rounding, unknowns and unsupported terms. Changed budget/dates are
estimates; cash-flow entries are never automatically synchronized. Existing future
debt changes/extras and savings goal/reserve commitments stay in force. The
interface blocks competing writes/unfinished edits, invalidates old comparisons,
retains failed drafts and rejects late/wrong-client/version/calculator replies.
No migration/dependency/installer is introduced; schema remains 15 and historical
`entered-plans-v1` captures stay unchanged. This preview uses `combined-preview-v1`.

Independent examples include 1,600 biweekly → 3,466.67; income/spending proposals
propagate to the savings allocation check; debt 200.00/primary savings 300.00 plus
retained goal 50.00 yield 1,816.67 buffer in the backend fixture, or 983.34 in
the native fixture with 1,633.33 necessary spending. Proposed snowball interest
23.01/payoff February 2027 compares with baseline avalanche 35.35. A delayed bill
removes the projected shortfall without writing its saved date or changing the
3,300.00 closing. Future debt changes/extras remain reserved and later deficits
are visible. Unknown values and missing optional models remain unavailable.

| Check | Result |
|---|---|
| Locked setup / production frontend build | Passed |
| Interface suite | 102 passed; proposal input-only IPC, canonical outcomes/unknowns, preserved failures, dirty/concurrent guards, reload and client/lock/version isolation |
| Linux backend / worker checks | Full 138-library + 2 worker run passed; subsequent six focused scenario tests passed, including three added retained-commitment/absent-model/higher-reserve checks (141 unique library tests covered) |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile / feature workflows | Standalone build and all four new comparison checks passed: initial/restart/normal restore/damaged-workspace recovery. Complete native run remains incomplete: WebDriver disconnected on a click opening the older counselor-report panel after the fourth comparison |
| Windows source CI | Passed b7fd415: [run 38641952017](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38641952017), 102 interface + 142 library + 2 worker tests (144 backend/process total). All installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

The first native run exposed a locale-dependent date control; proposals now use
explicit YYYY-MM-DD text with Rust validation. The second exposed incorrect
harness expectations: savings reserves the higher budget/model payment, yielding
983.34, and the correct emergency date is November 30, 2027. The third passed all
four new comparisons and unchanged-input checks, including both recovery paths,
then the driver closed an uncertain POST click while opening Casey’s existing
report panel. The final counselor-evidence export/dashboard checks and whole-run
completion were not reached. No click or financial mutation was replayed. Following
the original specification’s three-attempt limit, further full reruns are deferred
and the blocker is preserved. Click errors now include their intended selector for
future diagnosis without changing assertions or retrying mutations.

Shipping application source is `b7fd4151061b86cba7df3d5a6d8b78909008f9ae`; final
checkpoint changes only documentation and click-error diagnostics.

Visual review confirmed both columns, 400.00 reserved debt, 766.67 → 983.34 buffer,
35.35 → 23.01 interest, June 2028 → November 2027 emergency dates and three → zero
cash-flow warning days. The full-run exit is 1; the four feature checks pass, but
this must not be described as a completely passing native regression run.

Logs under `/workspace/shared`: `scenario-comparison-setup.log`, `scenario-comparison-ui.log`,
`scenario-comparison-backend.log`, `scenario-comparison-focused.log`,
`scenario-comparison-fmt.log`, `scenario-comparison-clippy.log`,
`scenario-comparison-native-build.log`, `scenario-comparison-native.log`/`.exit`,
`scenario-comparison-windows-job.log`, `scenario-comparison-windows-result.json`
and `scenario-comparison-windows-verified-validation.log`.
Earlier failures are `scenario-comparison-native-first.log`/`.exit` and
`scenario-comparison-native-second.log`/`.exit`; final artifacts are in
`desktop-smoke-scenario-comparison-verified`. Source/full-history checkpoints are
`checkpoints/eisenbart-combined-scenarios-source.tar.gz` and
`checkpoints/eisenbart-combined-scenarios.bundle` under the same shared directory.

Next priority: **diagnose the native-driver interruption and complete full workflow
validation, then persistent combined scenarios and explicit selection for reports**,
retaining input assumptions, versions, outcomes and uncertainty independently of
approved history. Seven major roadmap work areas remain; this checkpoint advances
the financial-model area without completing it or establishing release readiness.

## Stage 3E: optional budget method comparisons — 0.18.0 source

The saved-budget panel now offers read-only zero-based, 50/30/20 and custom
percentage references. Required minimum debt is classified from existing planned
debt into needs; savings and extra debt share the future bucket. The zero-based
check shows assigned/unassigned income or deficit without assigning funds.
A side-by-side table shows entered amounts, targets, exact signed differences and
reference positions. These optional guidelines never change plans, agreements,
reports, approved sources or audit events. Inputs/results are temporary and reset
across clients/restart; the saved budget remains persistent.

Rust rechecks authenticated scope, saved revision and canonical budget arithmetic.
IPC accepts strings/revision only. Percentages use integer basis points, must total
exactly 100.00%, and targets use largest-remainder cents (ties needs/wants/future)
so the target sum exactly equals entered monthly income. Unknown income/amounts
and minimums withhold affected figures, rather than becoming zero or compliance.
Estimates, incomplete expenses, zero income and separate model assumptions stay
explicit. Failed inputs are retained; edits hide old results, competing writes are
blocked, and stale budgets can be reloaded without a save. No migration/dependency
or installer is added. See [budget-method-comparisons.md](budget-method-comparisons.md).

Independent example: income 3,250.00, necessary 1,633.33, wants 200.00, savings
300.00/debt 400.00; zero-based assigned 2,533.33/remaining 716.67. Minimum debt
100.00 gives needs 1,733.33/future 600.00. Standard targets 1,625.00/975.00/650.00
produce differences +108.33/-775.00/-50.00; custom 60/20/20 needs target 1,950.00
produces -216.67. For 1.01 at 33.33/33.33/33.34, targets 0.34/0.33/0.34 conserve
all cents. Known balanced/deficit/zero, missing income/expenses, unknown minimums,
invalid precision/percent sums, large aggregate amounts, and cent conservation
are independently checked.

| Check | Result |
|---|---|
| Locked setup / production frontend build | Passed |
| Interface suite | 96 passed; lazy/input-only comparison, canonical figures, failed assumptions, unknowns, client/version mismatch/late replies, dirty/concurrent guards and read-only budget reload |
| Linux backend / worker suite | 135 library + 2 worker tests passed (137 total); four pure framework checks plus scoped/stale/locked/no-audit-write storage verification |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile / complete workflow | Passed standalone debug build and full native run, exit 0: exact comparisons/unknowns/failed assumptions with unchanged plans, all report/client workflows, restart and both recovery paths |
| Windows source CI | Passed source 2e07be0: [run 38637078036](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38637078036), 96 interface + 136 library + 2 worker tests (138 backend/process total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

The initial Windows run stopped during frontend checks: the first application
CSV-lock test and an assessment test hit the existing five-second deadline, then
pending application work interfered with later test isolation. Vitest now limits
concurrent DOM workers to two; no timeout or financial/privacy assertion is
removed or relaxed. The source application is unchanged by this runner adjustment.
The corrected Windows run passed all 96 interface tests and the complete
136-library/2-worker backend/process suite.

Native visual review confirmed the side-by-side targets/differences and intact
2,533.33 assignment/716.67 remaining balance. Four visible workflows before and
after restart/normal restore/damaged-workspace recovery verified temporary input
reset, unknown minimums, exact classification and rejected 101% assumptions.
All previous financial, internal/client PDF, source review and recovery checks
passed in the same complete native run. Shipping application source is
`c1f760ac89d51e9d32db6ccf39c7d7b0be30f7aa`; source `2e07be0` adds only the
bounded test-runner configuration and verification notes.

Validation logs are under `/workspace/shared`: `budget-methods-ui.log`,
`budget-methods-backend.log`, `budget-methods-clippy.log`,
`budget-methods-native-build.log`, `budget-methods-native.log`/`.exit`, and
`budget-methods-windows-job.log`. The initial Windows failure log is
`budget-methods-windows-failed.log`. Final native artifacts are in
`desktop-smoke-budget-methods`. Source/full-history checkpoints are
`/workspace/shared/checkpoints/eisenbart-budget-methods-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-budget-methods.bundle`.

Seven major work areas remain in the roadmap: further financial models/scenarios,
document/OCR review, fuller intake/workflow, security/permissions, commercial
branding/support, report refinements and Windows release acceptance. Each requires
smaller checkpoints; this feature does not complete the whole financial-model area.
See [remaining-requirements.md](remaining-requirements.md). Original stages 3–8 are
still partly open; the count is not a time or percentage estimate.

Next priority: **combined non-writing scenario comparisons**, building on entered
budget/debt/savings/cash-flow models while preserving historical records.

## Stage 5G: expanded counselor report evidence — 0.17.0 source

Internal assessments can now optionally capture selected saved session notes and
current merchant/payment groups with the latest counselor decision per reviewed
group. Notes retain their author, original time and ID; decisions retain label,
reason, cadence/subscription, applicability and original evidence digest with
record/approval/source/page/line bindings. Capture retains older approvals needed
by a reviewed payment after later corrections. All imported dates are labeled
separately from the financial assessment period. Narrative money, suggested cadence
and historical subscription decisions never enter approved totals or active plans.

The report viewer and offline internal PDF show these sections separately from
approved activity and hypothetical planning. Evidence times use America/Chicago;
UTC timestamps remain stored. Latest decisions without a current approved group
are explicitly historical. An invalidated decision stays Needs review even when
identical source values return. Client PDFs exclude internal note/payment evidence.
See [counselor-report-evidence.md](counselor-report-evidence.md).

Payload format 3 uses optional supporting context `counselor-evidence-v1`; formats
1/2 remain supported and unchanged. Records without this option continue to save
format 2. Schema stays 15. The existing scoped append-only assessment transaction
and audit check source/profile/plan versions and expected note count, then read up
to ten selected original notes by scoped IDs. Preview shows the latest 100 notes.
Reads/export/recovery recheck note identity against original append-only storage,
canonical text/time and recomputed payment groups/totals/states. New notes flag
supporting report context without replacing selected notes; legacy reports do not
claim notes or gain that new warning. Linked action plans inherit review state.
No new dependency or installer is added.

Independent example: historical payments 10.01 + 12.03 + 10.01 = 32.05 and operating
cash flow -32.05. A selected note mentioning 9999.99 remains narrative only.
Correcting the first payment to 9.99 yields current total 32.03 while retaining the
original reviewed 10.01 approval; withdrawal yields 22.04 without deleting it.

| Check | Result |
|---|---|
| Locked setup / production frontend build | Passed |
| Interface suite | 91 passed; new selection-only IPC, historical context/escaped notes, unchanged figures, failed draft/renewed acknowledgment, note cap, mismatched source/client and late reply tests |
| Linux backend / worker suite | 130 library + 2 worker tests passed (132 total); four new report/PDF/provenance/scope/forgery/restart/recovery checks; relevant PDF checks repeated after adding Chicago evidence timestamps |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile / expanded report workflow | Passed standalone debug build and final full native run, exit 0: selected notes, preserved prior review state after reconfirmation, new-note flags, four counselor PDFs, existing planning/client PDFs, restart and both recovery paths |
| Windows source CI | Passed shipping source 0abe32e: [run 38632476167](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38632476167), 91 interface + 131 library + 2 worker tests (133 backend/process total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

The first native run created the expanded report but expected a Needs review group
after the fixture had already explicitly reconfirmed it. The harness now captures
before reconfirmation, so the original historical status/reason can be checked
against the later current confirmation. A second run lost the driver connection
at a preexisting CSV click; driver logs and screenshot showed a responsive app and
connection reset. The uncertain click was not replayed. Each attempt uses a fresh
disposable fictional workspace; the corrected full run passed with exit 0.

Windows-validated shipping source is
`0abe32e3a6930a727442a73a81321b99eedc36a5`. Later changes concern the native
fixture timing and verification documentation.

Final UI/PDF visual review confirmed the selected note/author/Chicago timestamp,
original payment dates, historical state/reason and source/approval bindings. The
six-page Letter counselor PDF embeds local Unicode Open Sans, with no forms,
JavaScript or original attachments. It preserves 32.05 historical spending and
-32.05 operating cash flow; narrative 9999.99 and later notes remain excluded from
financial amounts and original context. New notes and later reconfirmation flag
context without rewriting captured evidence. Alex's selected note remains scoped,
and the client PDF continues to exclude internal session/payment evidence.

Validation logs are under `/workspace/shared`: `counselor-evidence-ui.log`,
`counselor-evidence-backend.log`, `counselor-evidence-focused.log`,
`counselor-evidence-clippy.log`, `counselor-evidence-native-build.log`,
`counselor-evidence-native-complete.log`/`.exit`, and
`counselor-evidence-windows-job.log`. Final artifacts are in
`desktop-smoke-counselor-evidence-complete`. Source/full-history checkpoints are
`/workspace/shared/checkpoints/eisenbart-counselor-report-evidence-source.tar.gz`
and `/workspace/shared/checkpoints/eisenbart-counselor-report-evidence.bundle`.

Complete coverage/final certification, remaining financial models and combined
scenario comparisons, branding/logo/contact details, direct print commands,
reminders/action completion and commercial/security/release acceptance remain open.
This is a tested development checkpoint, not a commercial release.

Next priority: **optional budget method comparisons** — zero-based, 50/30/20 and
custom percentage frameworks using exact arithmetic and non-writing comparisons.
See [remaining-requirements.md](remaining-requirements.md).

## Stage 5F: immutable client action plans — 0.16.0 source

Client action plans now record three to five ordered agreed steps, responsible
people, optional target dates, budget recommendations, follow-up and reviewed
limitations. A counselor selects a current immutable assessment with a captured
scenario, reviews backend-derived allocations/disclosures and acknowledges the
client, approved decisions, assignments, estimates and missing information.
The separate client PDF includes saved firm/client identity, Chicago report date,
historical position and income/spending breakdown, budget agreement, selected
strategy, emergency/goal targets and conditional milestones, priorities and
follow-up. Local fonts/vector bars remain offline. Existing files are never
replaced. Print the saved PDF through a local viewer.

Schema 15 adds scoped append-only plans, a composite immutable assessment link and
atomic creation audit. IPC accepts notes/selection rather than financial outputs.
Exact cent arithmetic derives selected debt/primary savings allocations; unknown
entered budget amounts and allocations exceeding surplus block recording.
Differences from captured budget allocations are disclosed without editing active
plans. Reads/export/recovery validate hashes, scoped parent identity, canonical
text/dates and recomputed original allocations/disclosures. Chicago date rules
are bundled by locked chrono-tz 0.10.4; no network/system timezone is needed.
Draft and busy guards protect navigation and competing assessment edits. Later
supporting changes mark guidance Needs review without changing original values.
See [client-action-plans.md](client-action-plans.md).

Independent example: income 3,250.00 less necessary 500.00/discretionary 200.00
leaves surplus 2,550.00. Selected debt 100.00 plus primary savings 250.00 leave
2,200.00, separately from captured budget debt 150.00/buffer 2,150.00 and approved
historical cash flow 799.99. Unicode client typography, Chicago winter/summer
midnight dates, failed atomic audit, scoped/locked access, immutable history,
forged rehashed financial outputs, schema 14 rollback/upgrade, restart and
encrypted recovery are covered by new backend checks.

| Check | Result |
|---|---|
| Locked setup / production frontend build | Passed |
| Interface suite | 88 passed, including four client-plan tests and parent draft/concurrency integration |
| Linux backend / worker suite | 126 library + 2 worker tests passed (128 total); five new storage/PDF/recovery tests and Chicago-date/IPC validation |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile / full client-plan workflow | Passed standalone debug build and final full native run, exit 0: reviewed agreement, five client PDFs, immutable figures/review flags after edits, all previous workflows, restart and both recovery paths |
| Windows source CI | Passed source 3c63f44: [run 38618328829](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38618328829), 88 interface + 127 library + 2 worker tests (129 backend/process total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

The first two native runs created the agreement and current client PDF but stopped
on assessment export-field focus during inherited fieldset loading. The runner now
reads the browser's effective `:disabled`/inert state and centers the field before
clear/typing; neither typing, clicks nor financial writes are retried. The next run
passed focus but exposed an older broad export completion selector matching the
nested client-PDF notice. Export helpers now wait for the direct success message
with the exact destination filename. All interrupted runs used disposable fictional
workspaces; no uncertain writes were replayed. The fresh final full run passed
with exit 0, including all exports, restart and both encrypted recovery paths.

Windows-validated shipping source is
`3c63f4499df295d6199fe8ffe213bce017ce6869`; later changes concern the native test
runner, verification documentation and a corrected MIT dependency-license entry.

Final native visual/PDF review confirmed firm/client identity, Chicago report date,
three priorities/responsibility, conditional Avalanche interest 35.35, emergency
target 4,899.99/completion 2028-06-30 and historical operating cash flow 1,820.02.
Agreed monthly debt 100.00/savings 250.00/buffer 1,066.67 remain separately labeled
from captured budget buffer 716.67. The four-page Letter PDF embeds local Unicode
Open Sans with no forms, JavaScript or original attachments. Missing inventories
remain unknown, and historical exports display NEEDS REVIEW.

Verification logs are under `/workspace/shared`: `client-action-plan-ui.log`,
`client-action-plan-backend.log`, `client-action-plan-date.log`,
`client-action-plan-clippy.log`, `client-action-plan-native-build.log`,
`client-action-plan-native-final.log`/`.exit`, and
`client-action-plan-windows-job.log`. Final artifacts are in
`desktop-smoke-client-action-plan-final`. Source/full-history checkpoints are
`/workspace/shared/checkpoints/eisenbart-client-action-plans-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-client-action-plans.bundle`.

This is reviewed guidance with an explicitly incomplete financial basis, not a
commercial release or certification that all accounts/statements are present.
Client signatures, completed-action history/reminders, logo/contact branding,
direct application printing, complete-basis final validation, remaining financial
models and release acceptance remain open. No installer is being created.

Next priority: **expanded counselor reports**, incorporating captured session and
recurring-review evidence with original provenance and historical validation.
See [remaining-requirements.md](remaining-requirements.md).

## Stage 5E: captured saved planning scenarios — 0.15.0 source

Added optional named planning snapshots to immutable internal assessments. A
counselor reviews saved budget/debt/savings/dated cash-flow plans, states assumptions
and explicitly chooses a repayment strategy (or none). Rust captures canonical
inputs, exact contexts/results/schedules, plan versions/timestamps and selected
outcomes. Approved historical activity remains separately calculated and unchanged.
Later plan edits flag context without rewriting the original payload; restoring
prior entered values does not restore the captured plan versions.

Payload format 2 adds optional planning with `entered-plans-v1`; format 1 records
remain readable/exportable. Schema stays 14 and old bytes are never rewritten.
The save transaction checks source/profile and all four saved-plan versions and
requires acknowledgment. IPC accepts selection/notes/versions, not financial
outputs. Reads/export/recovery validate scoped identity, hash, canonical inputs,
derived contexts and exact results from the original captured inputs. Shared pure
context functions reuse existing live model rules without changing calculations.
See [captured-planning-scenarios.md](captured-planning-scenarios.md).

The read-only viewer shows budget assumptions/allocations, entered debt terms and
selected payoff outcome, primary savings/goal milestones and independent dated
cash-flow assumptions, with paginated captured schedules and preserved warnings.
PDFs now include the selected hypothetical scenario, assumptions, terms and summary
milestones; full model schedules remain in the encrypted record/viewer. Draft,
load/client and refresh guards preserve failed drafts and prevent data carryover.

| Check | Result |
|---|---|
| Locked cloud setup / production frontend build | Passed |
| Interface suite | 83 passed; three new tests cover reviewed plan versions, selection-only IPC, separate canonical figures/escaped assumptions, stale draft retention/renewed acknowledgment, refresh/review flags and late/mismatched client replies |
| Linux backend / worker suite | 120 library + 2 executable worker tests passed (122 total); four new tests cover independent models vs approved data, historical preservation/review flags, stale/missing/unsupported selection, scope/lock, forged outcomes/contexts/identity/version/extra fields, corrupted recovery rejection, format 1 compatibility, restart and legitimate historical restore |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile | Passed standalone debug build with no bundle |
| Native full scenario/export/recovery workflow | Passed final shipping source c1a7d8b with corrected readiness/viewport harness: explicit Avalanche capture, separate approved figures, budget edit/reversion without rewriting/reactivating the record, four scenario PDFs, all previous workflows, restart and both encrypted recovery paths; runner exit 0 |
| Windows source CI | Passed final shipping source c1a7d8b: [run 38611157947](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38611157947), 83 interface + 121 library + 2 worker tests (123 backend/process total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

Independent model examples: income 3,250.00 / unallocated 2,150.00, Snowball
interest 43.10 vs Avalanche 35.35, emergency target 1,500.00 / completion
2027-01-31. Captured historical cash flow remains 799.99 when a new approved record
becomes 2,000.01. No frontend output can supply those captured amounts.

Initial Windows run 38610161975 passed 82 interface tests but stopped on an existing
balance-review test before backend checks. The ImportsEditor test helper now waits
for initial render/React effects before exposing review controls to the tests;
financial assertions remain intact. The corrected Windows run passed.

The first native run passed capture, source/planning isolation, plan edit/reversion
and PDF checks, then selected a recovery panel before it had loaded. The recovery
helper now explicitly waits for that panel with a read-only observation. A second
run lost a driver connection while scrolling before a CSV approval; screenshot
and driver log confirmed a responsive app/transport reset. A narrow viewport helper
now retries one connection closure using fixed scroll-only code. It cannot type,
click, invoke app commands or replay approvals/saves; general scripts and financial
mutations still have no automatic retry. A fresh isolated full run completed with
exit 0, without weakening assertions or replaying interrupted writes.

Final shipping source `c1a7d8ba6a8ed2c5a9585aa0b97151e252c73730` is uploaded and
Windows-validated; the final test-harness correction and verification notes are
preserved in the checkpoint. Native UI/PDF visual review confirmed the captured
hypothetical label/assumptions, America/Chicago saved-plan times, original monthly
income 3,250.00 / unallocated 716.67, entered terms and explicit Avalanche outcome
35.35 interest, and disclosures about independent plans/missing information. The
fictional nine-page PDF preserves original approved 1,820.02 activity, savings
milestone 2028-06-30 and independent cash-flow closing 3,300.00 after later changes.
No forms, JavaScript, original attachments or workspace credentials are included.

Source/full-history checkpoints are saved outside the checkout at
`/workspace/shared/checkpoints/eisenbart-planning-scenarios-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-planning-scenarios.bundle`. Validation logs
are under `/workspace/shared`: `planning-snapshot-ui-tests.log`,
`planning-snapshot-backend-tests.log`, `planning-snapshot-focused-tests.log`,
`planning-snapshot-clippy.log`, `planning-snapshot-native-complete.log`,
`planning-snapshot-native-complete.exit` and `planning-snapshot-windows-validation.log`.
Final native artifacts are in `desktop-smoke-planning-snapshot-complete`.

This captures existing saved entered plans; general combined non-writing scenario
variations/side-by-side comparison, a final client action plan, agreed priorities,
remaining advanced debt models, recurring/session evidence and complete original
report/release/commercial requirements remain open. A supported projection can
still show shortfall/growth/horizon warnings; capture never certifies completeness
or feasibility. No new dependency or installer was added.

Next priority: **client action plans** with three to five prioritized actions,
agreed allocations, selected debt/savings milestones, follow-up and final review.
See [remaining-requirements.md](remaining-requirements.md).

## Stage 5D: offline internal assessment PDF export — 0.14.0 source

Implemented local printable PDF export of saved immutable counselor assessments.
Rust rechecks scoped authentication, stored integrity/evidence and original
calculation before rendering; current source/client changes add historical
review notices without replacing captured figures. PDFs include identity, period,
exact approved amounts, local charts, classified/dated facts, counselor notes,
sources/discrepancies and transaction/inventory evidence. Every page discloses
internal/incomplete status. Embedded Open Sans and Unicode maps keep typography
local and text extractable. No schema change (14), new renderer service, source
write or planning inference. See [assessment-pdf-export.md](assessment-pdf-export.md).

Native destination selection and a path input support local saving. Draft and
busy guards prevent conflicting operations and late notices across clients.
Existing files are never replaced; a synchronized private temporary file is
published without clobbering. Unsupported glyphs/shaping characters or export
limits fail without truncation or changing the saved record.

| Check | Result |
|---|---|
| Locked cloud setup / frontend production build | Passed |
| Interface suite | 80 passed; two new tests cover export identity/arguments, cancellation, failed-path retention, concurrent operation/draft blocking and late client replies |
| Linux backend / worker suite | Full suite: 116 library + 2 executable worker tests passed (118 total); all four focused PDF tests also passed after the historical-warning presentation correction; four new tests cover historical 799.99/net worth 4,000.06 vs live 2,000.01, Unicode/font embedding, measured wrapping/page limits, unsupported glyph/no partial file, scoped/locked access and unsafe destinations; corrupt-record export rejection extends existing coverage |
| Formatting / Clippy, warnings denied | Passed |
| Native desktop compile | Passed debug build with no bundle |
| Native full desktop/recovery/export workflow | Passed final implementation 6f6bf1e: four actual printable PDF exports preserve original 199.98/net worth 525.05 versus current 500.00, captured evidence/review flags and fixed-record wording; byte-for-byte preservation after refused overwrites; restart and both encrypted recovery paths |
| Windows source CI | Passed final implementation 6f6bf1e: [run 38605164502](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38605164502), 80 interface + 117 library + 2 worker tests (119 backend/process total); all installer/package/upload steps skipped |
| Clean Windows installation/offline release acceptance | Not run |

Direct in-app printing, complete scenario-backed counselor reports, final client
action plans, organization logos/contact branding, complex-script font fallback,
PDF tags/archival certification and export-specific audit/permissions remain open.
PDFs are readable local files outside workspace encryption. Existing counselor
assessment creation remains audited; exporting does not create another audit event.

Initial Windows run 38604263280 passed 79 interface tests but failed the recurring
review test before backend checks. The test now explicitly settles initial React
loading effects and observes enabled review controls before clicking; its original
verification/withdrawal assertions remain intact. Final Windows checks passed.
Native visual inspection found a captured live-dashboard warning in the initial
PDF; saved views now render historical context wording without changing the
archived warning/financial payload or live dashboard.

The first native implementation run passed. During final validation, a later
focused Cargo test replaced the standalone executable with a development-server
binary; its refused-localhost screenshot identified the cause before data entry.
The standalone build was rebuilt last. A subsequent driver connection dropped
while typing the first report title; screenshot/driver log confirmed a responsive
app and a transport reset. No typing/save was replayed. A fresh isolated run from
the same final committed implementation passed the complete workflow. The build
order and transport limits are recorded in [smoke-test.md](smoke-test.md).

Final implementation `6f6bf1ebb4d7ffd65b7bfcec2cfeb6223f67a220` is uploaded and
Windows-validated. Native PDF visual review confirmed firm/client/counselor,
fixed original period/figures, review notices, readable charts/typography/margins,
page numbers and honest incomplete status. Final fictional PDF: five Letter pages,
embedded font/extractable text, no forms, JavaScript, original attachments or
workspace credentials. Source/full-history checkpoints are saved outside the
checkout at `/workspace/shared/checkpoints/eisenbart-pdf-export-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-pdf-export.bundle`. Validation logs remain
under `/workspace/shared`: `pdf-export-ui-tests.log`, `pdf-export-backend-tests.log`,
`pdf-export-focused-tests.log`, `pdf-export-clippy.log`,
`pdf-export-native-complete.log`, `pdf-export-windows-validation.log`; final native
artifacts are in `desktop-smoke-pdf-export-complete`.

Next priority: **captured planning scenarios and client action plans** with agreed
allocations, debt/savings milestones, prioritized actions and final report review.
The original remaining scope is recorded in [remaining-requirements.md](remaining-requirements.md).
No new installer was created.

## Stage 5C: immutable counselor assessment records — 0.13.0 source

Implemented encrypted, append-only internal assessment records with exact approved
figures, saved profile/firm/counselor identity, captured transaction decisions/raw
evidence, document references/checks and verified balance facts. Counselors can
add findings, recommendations and follow-up. Reused offline charts/text tables
show the captured assessment; paged evidence retains exclusions and original
source locators. Corrections/reversals/profile changes flag prior context without
rewriting historical figures. New records recalculate current approved data.

Schema 14 adds scoped records and atomic audit. Save checks source revision and
profile timestamp, requires acknowledgment and takes only notes/period/context
from IPC. Reads/recovery validate digest, headers, bindings and captured arithmetic
at the original date. Draft/source context and late-reply guards prevent unsafe
updates or cross-client carryover. Saving a report does not alter financial
source decisions/revisions or plans. See [saved-assessments.md](saved-assessments.md).

| Check | Result |
|---|---|
| Locked setup / production frontend | Passed |
| Interface suite | 78 passed, including five saved-assessment tests for scoped lazy load/save, canonical display, failed-draft retention, explicit changed-context review, late-client replies and conflicting-operation guards |
| Linux backend / worker suite | 112 library + 2 executable worker tests passed (114 total); four new tests cover immutable old figures versus new approved results, source/profile changes, scope, stale context, acknowledgment, atomic audit rollback, immutable SQL, malformed figures/dates, schema-13 upgrade/rollback, restart/recovery and corrupted snapshot rejection |
| Formatting / Clippy, warnings denied | Passed |
| Compiled native desktop workflow | Passed full finance/CSV/PDF/recurring/balance workflows, immutable original/new counselor assessments, source-change review flags and preserved original amounts/recommendations, full restart and both encrypted recovery paths |
| Windows source CI | Passed implementation d2b0ab5: [run 38600293809](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38600293809), 78 interface + 113 library + 2 worker tests (115 backend/process total); all installer/package/upload steps skipped |
| New installer / clean Windows offline acceptance | Not run |

Financial examples preserve 799.99 / net worth 4,000.06 before a source withdrawal;
a new record shows 2,000.01 while the original still retains 799.99. Existing
native examples preserve original 199.98 while live/current records change to
500.00. No financial amounts are accepted from the report form.

The first native attempt lost its WebDriver transport while typing an existing
cash-flow input before reaching assessments. A screenshot confirmed the app still
responded. A fresh isolated run completed the full workflow from the same
implementation, with no replayed financial write or weakened assertion. Native
visual review confirmed the saved client/firm/counselor identity, America/Chicago
record time, captured period/approved/incomplete basis, Needs review status and
original independently expected figures after later source withdrawal.

Final implementation `d2b0ab526737bc8ddf7d5a25b4c36c4690717a49` is uploaded and
Windows-validated. Source/full-history checkpoints are saved outside the checkout:
`/workspace/shared/checkpoints/eisenbart-assessments-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-assessments.bundle`. Logs/native artifacts
remain under `/workspace/shared`: `assessment-backend-tests.log`,
`assessment-ui-tests.log`, `assessment-clippy.log`, `assessment-native-complete.log`,
`assessment-windows-validation.log` and `desktop-smoke-assessments-complete`.

Next priority: **offline PDF report export and scenario-backed client action plans**.
The complete original Counselor Report, selected scenarios, saved plan assumptions
and final client plans remain open. See [remaining-requirements.md](remaining-requirements.md)
for the remaining original scope; clean Windows offline acceptance and commercial
readiness are still pending.

## Stage 5B: verified classifications and dated balance facts — 0.12.0 source

Implemented explicit necessary/discretionary and take-home/other income
classifications bound to exact scoped approval decisions. Corrections/reversals
and reapproval leave old classifications inactive until explicit re-verification.
Schema 13 adds append-only dated balance/classification snapshots with evidence,
reason, counselor, authenticated scope, revision checks and atomic audit.

The dashboard shows recorded expense/payroll classifications, dated cash/savings/
debt, sufficient-inventory net worth, a local debt chart and clearly labeled
coverage/payment ratio estimates. Unknowns, stale source decisions, incomplete
inventories, mixed dates and outdated balance facts prevent unsafe conclusions.
Manual planning inputs remain separate; missing transaction coverage remains
explicit even after a complete balance inventory is declared. Gross-income lender
DTI, partial per-account reserve allocations and savings-goal progress remain open.
See [verified-financial-facts.md](verified-financial-facts.md) for formulas/limits.

| Check | Result |
|---|---|
| Locked setup / production frontend | Passed |
| Interface suite | 73 passed, including four facts-editor, two metric-display and one full parent correction/re-verification regression |
| Linux backend / worker suite | 108 library + 2 executable worker tests passed (110 total); new tests cover exact cents/ratios, unknowns, Unicode/bounds, source corrections/reapproval, scoped bindings, audit/revision rollback, append-only history, restart/recovery and schema-12 upgrade/rollback |
| Independent Python Decimal examples | Passed: net worth 4,000.06 / 525.05, coverage 1.99 / 2.00 months, payment ratio 5.00% |
| Formatting / Clippy, warnings denied | Passed |
| Compiled native desktop workflow | Passed full existing finance/import workflows, explicit classifications and dated inventory, exact net worth/ratios, correction invalidation and explicit re-verification, withdrawal exclusion, local debt chart, full restart and both encrypted recovery paths with preserved bindings/history |
| Windows source CI | Passed implementation 96cd863: [run 38593850120](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38593850120), 73 interface + 109 library + 2 worker tests (111 backend/process total); all installer/package/upload steps skipped |
| New installer / clean installed Windows offline acceptance | Not run |

Initial checks caught a JSX delimiter and test helper calls using command names
instead of backend method names; both were corrected. Storage tests caught a
foreign-key reference to a nonexistent client key. The migration now uses the
repository's existing client/organization and counselor/organization keys, with
authoritative counselor ownership enforced in Rust. Focused tests then passed,
followed by the full local suite. A timing-sensitive recurring-detail assertion
failed in a combined interface run while its isolated case passed. It now awaits
the rendered detail before checking the same warning/disabled confirmation; the
complete interface suite subsequently passed. No safeguard was relaxed.

Native testing found that selecting the same prior class in a WebKit dropdown
left the old binding inactive after save. A diagnostic run confirmed that the
saved control remained unclassified, while independent backend and parent
interface tests correctly bound new source decisions. The interface now offers
an explicit Reverify prior classification button that updates the draft from the
current approval. The parent regression checks the correction, explicit action,
exact new approval ID and successful saved state. The complete native run passed
from final implementation 96cd863 after this fix, without replaying any financial
write. The two earlier native attempts preserved inactive classifications when
re-verification had not taken effect. Neither was counted as a passing workflow.

Final implementation `96cd863199d0f4d83b9ee476af10dbc6681942e0` is uploaded and
Windows-validated. Source/full-history checkpoints are saved outside the checkout:
`/workspace/shared/checkpoints/eisenbart-verified-facts-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-verified-facts.bundle`. Validation logs and
native artifacts remain under `/workspace/shared`, including
`facts-backend-tests.log`, `facts-ui-tests.log`, `facts-native-complete.log`,
`facts-windows-validation.log` and `desktop-smoke-facts-complete`.
This source checkpoint does not establish clean Windows installation/offline
acceptance, commercial readiness or independent security review.

Next priority: **immutable saved assessment snapshots and counselor reporting**
using the approved source decisions and verified facts, followed by offline PDF
reports/action plans.

## Stage 5A: approved transaction dashboard — 0.11.0 source

Implemented a client-specific read-only assessment for explicit 1–24 calendar-month
periods. Rust calculates approved-only income receipts, gross spending, refunds,
net spending, operating cash flow and all account movement in exact cents. Locally
bundled charts show approved categories, monthly income/spending and cash-flow
trends. Every view labels period, revision, approved basis, incomplete coverage,
undated exclusions, review decisions and outdated periods. Source corrections and
reversals recalculate results; stale/client-mismatched replies are hidden.

No new schema, financial writes or dependencies. Planning remains separate.
Verified take-home pay, necessary/discretionary classifications, balance facts,
asset/debt metrics, debt/savings charts and immutable final reports remain pending.
See [financial-dashboard.md](financial-dashboard.md) for formulas and limits.

| Check | Result |
|---|---|
| Locked setup / production frontend | Passed |
| Interface suite | 66 passed, including four dashboard interaction tests |
| Linux backend / worker suite | 103 library + 2 executable worker tests passed (105 total); focused dashboard/scoped-storage tests also passed after adding the aggregate-limit example |
| Independent Python Decimal examples | Passed: 918.89 operating flow versus 718.89 account movement across two months |
| Formatting / Clippy, warnings denied | Passed |
| Compiled native application | Passed full financial/import workflows, explicit dashboard periods, approved CSV/PDF totals, pending exclusions, local chart tables, automatic correction/reversal updates, full-process restart and both encrypted recovery paths |
| Windows source CI | Passed implementation b6d5eae: [run 38587770252](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38587770252), 66 interface + 104 library + 2 worker tests (106 backend/process tests); all installer/package/upload steps skipped |
| New installer / clean Windows offline acceptance | Not run |

The first synthetic engine fixture incorrectly used an empty category on an
approved row; approval requires a verified category. The fixture now uses Food,
and the corrected focused tests pass without relaxing approval validation.
The outdated-period example now selects June, which is more than 90 days before
the fixed October reference date. Per-transaction input limits do not cap derived
assessment totals; an independently expected 2,000,000,000.00 aggregate verifies
exact calculation and integer chart scales.

The first native launch lacked the cloud toolchain path; sourcing the existing
cloud environment fixed the launch without application changes. The first full
UI attempt lost its WebDriver connection while selecting an existing CSV mapping
option. A captured screenshot confirmed the application was responsive; no
financial operation was retried. A fresh isolated run completed the entire
workflow from the same committed implementation. Native visual inspection
confirmed the displayed assessment period, approved basis and independently
expected income/spending/refund/cash-flow totals.

Implementation `b6d5eae0b1e120d291a083e6ac4546749293b259` is uploaded and Windows-validated.
Final source and full-history Git checkpoints are saved outside the repository as
`/workspace/shared/checkpoints/eisenbart-dashboard-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-dashboard.bundle`. No installer was created.
Clean installed Windows 11 offline acceptance and commercial release remain open.

Next priority: **approved expense classifications and dated verified balance facts**
for the remaining dashboard metrics, before counselor reports and action plans.

## Stage 4D: statement running-balance reconciliation — 0.10.0 source

Implemented exact integer-cent comparisons for explicitly mapped after-transaction
checking/savings CSV balances and declared oldest/newest-first source order.
Original local steps use reported anchors and original amounts regardless of
review status, only to check source arithmetic. Independent approved cumulative
steps use a known entered opening and approved movements; rejected rows contribute
nothing and never become anchors. An undecided row makes later approved
expectations unknown. Missing dates/amounts/balances remain unknown; contradictory
dates block checks without sorting. Rejected dates are excluded from the approved
sequence. Same-day order is preserved and disclosed.
Final reported-versus-entered-closing checks complement existing whole-source
and approved-total checks. Intermediate differences cannot hide behind a matching
whole-statement total. Matched arithmetic never approves or verifies an asset.

Counselors can correct or clear order declarations with a reason. Schema 12
stores authenticated, client/document-scoped append-only settings and generic
audit events with optimistic revisions. Original source metadata, financial
choices and plans remain unchanged; derived exceptions update immediately after
corrections/reversals and block unsafe bulk approvals. Changing a setting does
not reset Needs Clarification to Pending or automatically approve any row.
Settings/history and comparisons survive encrypted recovery; older JSON defaults
to unknown order. Migrations/write failures preserve existing records. Existing
limits apply, plus 1,000 balance-setting decisions/client. Scanned/text-PDF balance
columns, before-transaction/available/card balances and automatic statement
completeness claims remain outside this checkpoint. See
[statement-balance-reconciliation.md](statement-balance-reconciliation.md).

| Check | Result |
|---|---|
| Locked setup and production frontend | Passed |
| Interface tests | 62 passed, including explicit mapping/order, source-versus-pending display, failed draft/dirty retention, conflicting-operation guards and clearing disabled mapping |
| Linux backend/process suite | 98 library + 2 executable worker tests passed (100 total); seven new engine/storage tests cover exact cents, both source orders, date conflicts, missing/zero/negative figures, pending/rejected exclusion, corrected fields, intermediate discrepancies, scope, stale/audit rollback, Unicode reason limits, immutability, restart/recovery and schema-11 migration rollback |
| Independent Python Decimal examples | Original closing 1,199.98; corrected closing 1,199.99; rejecting rent gives approved movement 500.00, expected 1,500.00 and 300.02 difference |
| Format / Clippy all targets, warnings denied | Passed |
| Compiled native desktop workflow | Passed full existing client/financial/CSV/PDF/recurring workflows, explicit source order and conflict correction, validated bulk approval, one-cent correction, rejection exclusion, immutable settings, client isolation, full-process restart, normal backup restore and missing-metadata recovery |
| Windows source CI | Passed final implementation c052453: run [38525998374](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38525998374), production frontend, 62 interface tests, 99 library + 2 worker tests (101 backend/process tests total). All installer/package/upload steps skipped |
| New installer | Intentionally not built |
| Clean installed Windows 11 offline acceptance | Not run |

Two initial synthetic storage fixtures were corrected: a resolved source-order
setting intentionally leaves imported uncertain records Needs Clarification, and
a second account needs a unique label or existing-account selection. The final
focused storage rerun passed all three tests without weakening either safeguard.
Native UI testing found a selected statement panel closing on a saved revision;
the panel now retains its source selection and displays fresh results. An
interface regression checks results remain visible after a successful setting save.
A second native check found source-record navigation also cleared the source
selection; navigation now retains the statement alongside the transaction editor,
with an interface regression for that context. The selector placeholder closes
the view explicitly.
The final complete native run passed from the final implementation after both
context-preservation fixes. Native visual inspection confirmed the separate
source/approved comparisons and source-record navigation. No financial write was
replayed or required assertion disabled.

Final implementation `c0524535e9b21618dd65f91ed8b775a6f2f94bb7` is uploaded to
main and passed Windows run [38525998374](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38525998374).
Logs: `/workspace/shared/reconciliation-backend-verified.log`,
`reconciliation-frontend-tests.log`, `reconciliation-clippy.log`,
`reconciliation-native-complete.log`, and `reconciliation-windows-validation.log`.
Final native artifacts: `/workspace/shared/desktop-smoke-reconciliation-complete`.
Verified source/Git recovery checkpoints are
`/workspace/shared/checkpoints/eisenbart-reconciliation-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-reconciliation.bundle`; these preserve
development source/history, not client records or a Windows installer.

Next priority: **client financial dashboard** built from approved imported facts,
with explicit assessment dates, missing/outdated-data indicators and locally
bundled charts. Broader statement adapters, scanned OCR, saved cross-module
scenarios, advanced financial models, reports/action-plan PDFs, multiple counselor
roles, firm branding, licensing/signing and full release acceptance remain open.
No new downloadable build was created, as requested.

## Stage 4C: merchant grouping and recurring-payment review — 0.9.0 source

Implemented approved-only historical payment grouping within one statement account
and type. Exact normalized verified merchant names control membership; absent
merchants use the exact description with an explicit fallback warning. Counselors
combine or separate payments by correcting verified Merchant fields in source
review, preserving original provenance and correction history. Expense/fee groups
show exact integer-cent observed totals, amount ranges, dates and source locators;
other types and unapproved records never enter this checkpoint's analysis.

Conservative suggestions require at least three distinct approved payment dates,
no unresolved duplicate conflicts, and every adjacent interval matching weekly,
fortnightly, calendar-monthly, quarterly or annual rules. Month-end/leap-day dates
and variable amounts have independently checked examples. There is no automatic
subscription classification, confirmation, fixed future charge or planning write.
Counselors explicitly choose/correct frequency and label, optionally classify a
verified subscription, or dismiss/withdraw a pattern with a reason.

Schema 11 captures append-only decision evidence, exact approval-history IDs,
reviewer/time/reason, revision/hash checks and atomic generic audit events in the
encrypted workspace. Changed evidence appends a permanent invalidation marker
atomically with the source change. Withdrawing a newly added charge cannot revive
an old decision when the remaining facts match again. Changed/reversed evidence
requires new review; identical
reapproval does not silently reactivate prior confirmation. Absent groups retain
historical decisions. Authenticated client scope, failed-save retention, unsaved
navigation/lock guards, restart, backup and restore validation remain active.
Limits: 500 captured payments/decision, 256 KiB/decision JSON, 1,000 decisions
and 8 MiB captured JSON/client; bounds fail without deleting history. Existing
source limits apply. See [recurring-payment-review.md](recurring-payment-review.md).

| Check | Result |
|---|---|
| Locked setup, production frontend and native debug build | Passed |
| Interface tests | 59 passed; explicit decisions/frequency correction, unchanged exact totals, failed draft/dirty retention, conflicting-operation guards and changed-evidence dismissal |
| Linux Rust suite | 91 library + 2 executable worker tests passed (93 total); seven new tests cover independent patterns, exclusions, duplicates, reversal/reapproval, scope, atomic audit failure, stale writes/hashes, lock/restart, backup restore, permanent invalidation/write rollback, and schema-9/10 upgrades and rollback |
| Independent Python Decimal example | 32.05 observed total; 22.04 after first payment is withdrawn |
| Format / Clippy all targets, warnings denied | Passed |
| Compiled native desktop workflow | Passed final schema-11 full financial/CSV/PDF/recurring workflow, explicit subscription/reconfirmation, permanent invalidation after an added-then-withdrawn charge, exact totals, client isolation, whole-process restart, normal backup restore and missing-metadata recovery, including captured history and invalidation flags |
| Windows source CI | Passed final source 92891dd: run [38517928152](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38517928152), production frontend, 59 interface tests, 92 library + 2 worker tests (94 backend/process tests total). All installer/package/upload steps skipped |
| New Windows installer | Intentionally not built |
| Clean installed Windows 11 offline acceptance | Not run |

Initial backend regression found an older PDF migration fixture retaining the
newer recurring-history table while simulating schema 8; its downgrade fixture
was corrected and the complete suite rerun. A backup test fixture was corrected
to use the existing lock-before-restore rule. An initial native run stopped after
a WebDriver transport closure during the prior cash-flow workflow; no financial
write was replayed or assertion weakened, and the complete run restarted in a
fresh disposable workspace. Native visual review prompted existing compact table
styles and a bordered decision form; the styled build passed the full native
workflow again. Final review found the added-then-withdrawn-charge edge case;
permanent invalidation markers and regressions were added. Schema 11 preserves
the initial schema-10 source checkpoint and flags those earlier decisions for
explicit reconfirmation; upgrade failures preserve the original records.
Intermittent native WebDriver closures also interrupted final checks; financial
writes were never retried in place. The final exact-source native workflow passed after the full backend suite
finished, and passed again after the text-validation patch; no assertion or
financial-write retry policy was weakened. Windows source validation passed for
schema 11 and the final character-length patch.
Source/debug logs and fictional artifacts are under
`/workspace/shared/recurring-*`; final exact-source native artifacts are
`/workspace/shared/desktop-smoke-recurring-complete`. Full backend logs are
`recurring-schema11-backend.log`, interface logs `recurring-frontend-release-final.log`,
and native logs `recurring-native-complete.log`. The final text validation uses
character counts consistently with existing input limits; seven focused tests
passed again, including 120-character accented labels and 500-character accented
reasons surviving lock/restart and encrypted recovery, with over-limit rejection.

Final implementation commit `92891dd0924e420cc528c084c002e1d7d23ee5a6` is uploaded
to main. Windows source CI passed run [38517928152](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38517928152);
full log: `/workspace/shared/recurring-windows-final-validation.log`. Final native
workflow and visual review passed from the same implementation source. Source-only
recovery checkpoints are saved as `/workspace/shared/checkpoints/eisenbart-recurring-source.tar.gz`
and `/workspace/shared/checkpoints/eisenbart-recurring.bundle`; these preserve
tracked development source/Git history, not encrypted client records or an installer.

Next priority: **row-level statement reconciliation** against known reported
running balances, with explicit source/order/sign checks and unresolved exceptions.
Scanned OCR, institution adapters, dashboards/reports, advanced models,
accounts/roles, licensing/signing and full installed/commercial acceptance remain
unfinished. No new download was created, as requested.

## Stage 4B: selectable-text PDF import and review — 0.8.0 source

Implemented bounded local PDF inspection in a supervised child of the desktop
executable before workspace/Tauri initialization. Only source bytes pass through
stdin; no workspace key, password, path, source filename, authentication, or proxy
variables are supplied. No PDF renderer, JavaScript evaluator, attachment
extraction, OCR, or network service is invoked. CPU/data/core limits on Linux,
Windows memory Job Objects, 15-second wall deadlines, and bounded input/output
keep parser failures apart from the active workspace. These are process/resource
boundaries, not a complete operating-system sandbox or security certification.

Counselors explicitly choose page/text lines, actual date style, account, known
period/balances, sign convention, and optional original retention. Supported
starting layout is full date first, description middle, one signed amount last.
Multiple numeric columns or invalid/missing amounts remain unknown; wrapped text
and glyph/ordering uncertainty require human correction. All PDF rows begin as
Needs Clarification, never bulk eligible, and require classification/category,
source acknowledgment and a reason before individual approval. No account number,
transaction type, missing date/amount, merchant, or statement metadata is guessed.

Immutable reported text retains document hash/ID, account, selected page/line,
original date/amount, extraction method and uncertainty. Corrections/reversals
preserve the existing immutable before/after decision history. Approved-only
exact-cent totals, duplicate advisories across CSV/PDF sources, paired transfers,
client scope, optimistic revisions, atomic audit/rollback, lock/cancel/expiry,
and planning separation reuse the CSV foundation. Authorized original PDFs are
SQLCipher BLOBs included in existing authenticated backups. Restore validates
PDF locators and imported decisions/transfer links. Schema 9 preserves previous
records/CSV JSON and adds PDF audit events; older applications cannot reopen it.

See [pdf-import-review.md](pdf-import-review.md) for actual user steps, format,
parser boundaries, completeness limits, and independent examples. Limits: 2 MiB,
100 pages, 1 MiB extracted text, 2,000 nonempty/4,000 total lines, 2,048 bytes/line,
and 500 selected lines/source; existing client/backup limits remain unchanged.
Scanned/image-only or protected PDFs, including owner-protected files with an
empty user password, are rejected. Mixed image/text PDFs require checking every
original page for missing transactions. Selected-line totals cannot prove completeness.

| Check | Result |
|---|---|
| Locked setup/dependency fetch, production frontend, native debug build | Passed |
| Interface tests | 56 passed, including explicit PDF selection/unknown metadata, inert markup, no automatic/bulk approval, failed-draft retention and cancel/no-write |
| Linux Rust full suite | 84 library tests + 2 executable worker integration tests passed (86 total); final focused PDF rerun 7 + worker rerun 2 passed after core-dump/original-encryption safeguards |
| Independent Python Decimal example | Original PDF net 974.90; only approved payroll adds income to 3,000.01 and combined CSV/PDF activity to 1,820.02; pending expense contributes zero |
| Format and Clippy all targets, warnings denied | Passed |
| Compiled native desktop workflow | Passed existing client/planning/CSV workflows, separate PDF parser, damaged rejection, explicit line selection, source locators, individual approval, pending exclusion, authorized retention, whole-process restart/backup interruption, PDF decisions/history restored through normal and missing-metadata recovery |
| Native visual inspection | Passed for reported-text provenance, warnings, and corrected verified fields; final exact-source desktop/recovery rerun passed |
| Windows source CI | Passed: run [38513471590](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38513471590) from 5b6bcbf; production frontend, 56 interface tests, 85 library tests + 2 desktop-worker tests (87 backend/process tests total), including Windows Job Object handling and SQLCipher memory-lock regression |
| New Windows installer | Intentionally not built |
| Clean installed Windows 11 offline PDF workflow/native OS pickers | Not run |

Initial full native artifacts: `/workspace/shared/desktop-smoke-pdf`; exact-source
final artifacts are `/workspace/shared/desktop-smoke-pdf-final`. Logs are under
`/workspace/shared`: `pdf-backend-full.log`, `pdf-focused-final.log`,
`pdf-process-final.log`, `pdf-frontend-tests.log`, `pdf-frontend-build.log`,
`pdf-clippy.log`, `pdf-native-smoke.log`, and `pdf-native-final.log`.
Existing negative SQLCipher fixtures produce expected authentication diagnostics;
no verification or assertion was disabled. During implementation, a form fragment
boundary and a Unix unit-valued binding were corrected before checks passed.
Final review rejects original encryption even after a library decrypts an empty
user password, and disables Linux parser core dumps. Synthetic worker tests
exercise the actual desktop executable/protocol, including deadline termination.
The worker process is not claimed as a complete OS filesystem/network sandbox.

Source commit `5b6bcbf9343c829f074a87ff7ff5f9da1d0f84d3` is uploaded to
main. Windows run [38513471590](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38513471590)
passed; its full log is `/workspace/shared/pdf-windows-validation.log`. The final
compiled exact-source native workflow passed at `/workspace/shared/desktop-smoke-pdf-final`.
Installer creation, installed launch/uninstall, download metadata, installer upload,
and installer-log upload were explicitly skipped. No manual package run occurred.
Windows checks establish source/debug worker compilation/protocol/resource handling;
clean Windows 11 GUI/offline acceptance and release-package verification remain unrun.

Verified tracked-source and full-history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-pdf-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-pdf.bundle`, refreshed after these final
notes. They are development source checkpoints; application-data backups use
`.ebcbackup`. The earlier 0.3.0 installer contains neither this PDF stage nor the
later source-only features. No new download was created.

Next priority: **merchant grouping and recurring-payment review** on imported
records, with advisory suggestions and explicit counselor decisions. Institution
adapters, scanned OCR, automatic table reconstruction, advanced models, dashboard
charts, finalized reports/PDF action plans, accounts/roles, licensing/signing,
clean Windows acceptance, and commercial release review remain unfinished.

## Stage 4A: local CSV import and transaction review — 0.7.0 source

Following the original staged specification, implemented explicit, bounded local
CSV inspection/mapping, client checking/savings account association, entered
statement periods/balances, immutable source provenance, optional authorized
complete-source retention inside SQLCipher, duplicate advisories, a client review
queue, and the exact Pending / Needs Clarification / Approved / Rejected states.
Importing never approves a record. Unknown dates/amounts remain unknown; no
income, account number, merchant classification, or statement balance is guessed.

Rust owns exact-cent approved-only totals and all decisions. Refunds offset
spending; verified equal/opposite transfer legs update separate accounts without
extra income/expense. Bulk approval accepts only 1–100 unchanged validated
Pending rows; conflicts/corrections/uncertain data require individual review.
Approval reversals immediately withdraw their effects and retain immutable
before/after fields, links, reviewer, timestamp, and reason. Reversing a linked
transfer clears both approvals atomically. Imported facts never rewrite planning
inputs. Table filters leave the clearly labeled all-imported-dates totals intact.

Schema 8 preserves all earlier plans and audit history; scoped composite keys,
optimistic revisions, immediate transactions, and atomic audit writes protect
isolation and stale/failing operations. Same-account identical source hashes
cannot import again. Original/corrected row overlaps and reference matches are
advisory; repeated legitimate movements need explicit review. Duplicate diagnostics
are capped at 20 related IDs per row to bound large overlap groups. Captured
source bytes expire after 15 minutes or lock; canceling inspection writes nothing.
Backups include authorized original CSV BLOBs, provenance, choices, and history;
restore also validates imported totals and reciprocal approved transfer links.

See [csv-import-review.md](csv-import-review.md) for user steps, supported formats,
source/accepted statement checks, independently known examples, storage, and limits.
No new Windows installer was built, as requested. The old 0.3.0 download lacks
this stage. Clean Windows 11 installed/offline CSV and recovery acceptance and
independent security review remain unrun. Continue using fictional records.

| Check | Result |
|---|---|
| Locked dependency setup, production frontend, compiled debug native application | Passed |
| Frontend tests | 54 passed, including explicit CSV mapping/retention, pending staging, escaped source text, cancel/no-write, native totals, decision payloads, exact failed-draft retention, transfer pairing, failed-load protection, late responses, and private-path lock/navigation guards |
| Linux Rust tests | Full suite 77 passed; focused import suite 13 passed, including a 2,000-row overlap with bounded diagnostics; schema 7 migration/rollback and existing schema 1–6 fixtures preserve prior records |
| Independent Python Decimal example | Confirmed net spending 1,179.99; checking activity 620.01; savings 200; combined 820.01; source checking closes 1,610 before fee rejection |
| Rust format and Clippy all targets, warnings denied | Passed |
| Full compiled Linux native desktop workflow | Passed: existing plans; explicit CSV mapping/retention; no automatic approval; bulk approval, individual refund, fee reversal/history, transfer pairing, repeated-source rejection; two-client isolation; lock/wrong password; full restart; backup interruption, restored CSV decisions/history/links and missing-metadata recovery |
| Native visual inspection | Passed: review/history readable, checkboxes correctly sized, wide tables scroll inside their own wrapper; final compiled layout/recovery rerun passed |
| Windows source CI | Passed: run [38509546704](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38509546704) from 8054f12; production frontend, 54 interface tests and 78 Rust tests, including the Windows-only SQLCipher memory-lock regression |
| New Windows installer | Intentionally not built |
| Clean installed Windows 11 offline workflows/native pickers | Not run |

Native full-workflow artifacts: `/workspace/shared/desktop-smoke-imports-final-verified`; final exact-source layout/recovery artifacts: `/workspace/shared/desktop-smoke-imports-layout`.
Final logs: `import-backend-complete.log`, `import-bounded-tests.log`,
`import-frontend-final.log`, `import-frontend-build-final.log`,
`import-clippy-final.log`, `import-native-final-verified.log`, and `import-native-layout.log` under
`/workspace/shared`. Negative SQLCipher fixtures reject altered/wrong-key data;
expected authentication diagnostics do not indicate failed checks.

During implementation, a CRLF/multiline source-line locator, fraction grouping,
rejected-row duplicate reapproval, and excessive duplicate diagnostics were
corrected before passing tests. Clippy findings were fixed through clearer parser
control flow and grouped review updates. Native inspection corrected checkbox
sizing. The harness now centers visible controls before clicking to avoid
WebKit's partial-viewport click issue, preserves Unicode in XPath option labels,
and retries one explicit read-only DOM/runtime observation on connection closure.
Early driver interruptions were discarded and rerun in fresh temporary workspaces;
financial writes, clicks, typing, and general scripts are never replayed. No
security verification or assertion was disabled. The final CSV table scrolls
inside its wrapper; the final exact-source layout/recovery rerun passed.

Source commit `8054f12e9a8bb60dcd9f3b2ba8c945e43ee94037` is uploaded to
main. Windows run [38509546704](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38509546704)
passed; the full log is retained at `/workspace/shared/import-windows-validation.log`.
Installer creation, installed launch/uninstall, download metadata, installer upload,
and installer-log upload were explicitly skipped. No manual package run occurred.
These Windows results verify source compilation/tests, not installed GUI/native
pickers or clean Windows 11 offline acceptance.

Recoverable tracked-source and full-history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-imports-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-imports.bundle`, refreshed and verified
after these final validation notes. They are development source checkpoints;
client workspace recovery uses the separate encrypted `.ebcbackup` format.

Next priority: selected **text-based PDF statement extraction**, retaining source
locators and uncertainty and feeding this same review queue. Scanned OCR,
institution/card parsers, inferred recurring groups, row-by-row running-balance
checks, finalized reports, advanced scenarios, licensing/signing, and commercial
release acceptance remain unfinished.

## Recovery hardening: encrypted backup and restore — 0.6.0 source

Following the original staged specification, implemented manual encrypted local
backups and verified whole-workspace restore. The user selects a local/external
folder; unique, exclusively created `.ebcbackup` files never replace existing
backups. SQLCipher transaction snapshots include committed WAL records. The
container authenticates its header and ciphertext with domain-separated
HMAC-SHA256 derived from the workspace key; no password or raw key is persisted.

Restore is available on locked, first-setup, and storage-error screens. It verifies
the password/container, SQLCipher and SQLite integrity, relationships, schema,
and saved plan inputs, then displays the organization/counselor/client count.
An explicit RESTORE confirmation activates a new private generation through
one atomically replaced pointer. Previous files remain intact; records are not
merged. Candidate cancellation/expiry, conflicting operations, source changes,
invalid files, and migration/audit/activation failures preserve the active source.
Existing sessions lock if another instance changes the active generation.

Schema 7 preserves all earlier records and immutable audit history, adding
backup/restore events. Older backups migrate only in the recovery copy. Limits:
256 MiB database, 4 KiB header, and 1,000 clients. Backups require the original
password; there is no password reset. See [backup-recovery.md](backup-recovery.md)
for user steps, exact format, file layout, retained-copy limits, and durability.

| Check | Result |
|---|---|
| Locked setup/dependency fetch and production frontend build | Passed |
| Interface tests | 47 passed: backup folder selection, password clearing, warnings/errors, verified review/confirmation/cancellation, stale responses, and storage-error recovery to a clean locked screen |
| Linux Rust tests | Full suite 63 passed; final recovery-only rerun 10 passed after exclusive-staging/publication safeguards were tightened |
| Rust format and Clippy all targets with warnings denied | Passed |
| Compiled native desktop workflow | Passed: full existing client/financial workflows; encrypted backup; wrong backup password; process stop during verified review leaves original active; explicit restore retains original database bytes and all saved plans; fresh process detects missing metadata and recovers through the error screen |
| Native visual inspection | Passed for backup controls and authenticated restore review; native OS picker dialogs themselves were not exercised by WebDriver |
| Deferred Windows download metadata | Parsed successfully with PowerShell; packaging not invoked |
| Windows source checks | Run [38496639835](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38496639835) passed from 85f0c13: production frontend build, 47 interface tests and 64 Rust tests, including Windows file locks, pointer replacement, backup/restore failures, and SQLCipher memory-lock regression |
| New Windows installer | Intentionally not built, as requested |
| Clean Windows 11 installed/offline recovery and physical removable-drive/power-loss drill | Not run |

Native artifacts: `/workspace/shared/desktop-smoke-recovery`; logs:
`/workspace/shared/recovery-native-smoke.log`, `recovery-backend-final.log`,
`recovery-backend-final-recovery.log`, and `recovery-frontend-final.log`.
Negative fixtures deliberately cause SQLCipher page-authentication diagnostics;
those checks passed by rejecting damaged data. No verification was disabled.
During implementation, compile errors in type inference/audit transaction usage
and fixture error formatting were corrected before tests passed. Final review
also tightened exclusive staging and ensured a failed temporary-pointer claim
cannot delete a pre-existing file. Snapshot sync handles have write access for
Windows FlushFileBuffers compatibility; Windows source validation is recorded separately.

Source commit `85f0c134adcc536bc0d4b2328f1c7cf119a38895` is uploaded to
main. Windows source CI passed, and installer build, install/launch/uninstall,
metadata preparation, installer upload, and validation-log upload were explicitly
skipped. No manual package run was requested. The full Windows source test log is
retained at `/workspace/shared/recovery-windows-validation.log`. This verifies
Windows core compilation and tests, not installed Windows GUI/picker/offline
recovery or real removable-drive durability.

Recoverable tracked-source and full-history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-recovery-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-recovery.bundle`, refreshed and verified
after the final validation notes commit. These are development source archives;
application data backups use the separate encrypted `.ebcbackup` format.

A durable backup survives a subsequent audit-write failure and reports that
warning; its snapshot contains prior audit history. A post-activation directory
sync failure reports an applied restore with a durability warning. Interruption
can leave incomplete backup files or unreferenced encrypted staging folders,
which are never selected automatically. Previous generations are retained;
supported retained-copy selection/pruning is future work. Future document/report
attachments will need inclusion in an extended format; none are stored yet.

Fictional data remains required while independent security review and clean
Windows acceptance are unfinished. The earlier 0.3.0 download does not include
this feature. Installer creation remains manual opt-in. Next: scoped synthetic
CSV import and explicit review/approval, preserving planning inputs separately.

## Stage 3D: paycheck-based cash-flow calendar — 0.5.0 source

Implemented persistent cash-flow plans with up to 20 separate accounts; exact
opening/running/closing balances; entered weekly, biweekly, semimonthly, monthly,
and irregular one-time schedules; dated bills/loan payments/external savings;
paired internal transfers; confirmed/estimated schedules; a calendar with daily
account review; and a chronological ledger. Dated entries are never manufactured
from monthly budgets or forecasts. Current budget/debt/savings revisions and
amounts are references for reviewing completeness, not automatic transactions.

The engine identifies negative opening, intermediate, and daily balances, bills
before the next entered paycheck, missing funding dates, and concentrated payment
days. Same-day debits precede credits conservatively, keeping shortfalls visible
even when daily/monthly totals are positive. Transfers preserve combined cash;
a reserve account never silently covers checking. Unknown amounts remain unknown
and propagate only to affected accounts. Dates respect leap years, original
monthly anchors, inclusive end dates, and semimonthly collision deduplication.

The counselor can edit a date and preview an alternative without saving. Preview
refreshes references, preserves the draft/baseline, and does not write records,
revisions, or audit events. Adjustments require actual payee permission and do
not guarantee bank processing. Encrypted schema 6 preserves all earlier plans
and immutable audit records; atomic saves and optimistic revisions reject stale
windows, failures roll back, and locking clears private state. Older applications
cannot reopen schema 6. See [cash-flow-calculations.md](cash-flow-calculations.md).

| Check | Result |
|---|---|
| Locked setup, production frontend build, compiled debug native desktop | Passed |
| Interface tests | 40 passed, including 7 cash-flow tests and client navigation/lock guards |
| Linux Rust tests | 53 passed, including 8 independent cash-flow calculation tests and 3 persistence/migration/atomicity tests; foreign firm/counselor scope also covers get/preview/save |
| Independent Python Decimal fixture | Checking -700 with combined +300 before payday; transfer conserves 1,800; closes checking 2,100 + reserve 1,200 = 3,300 |
| Rust format and Clippy all targets with warnings denied | Passed |
| Final native GUI workflow | Passed: account/activity entry, calendar/ledger, independently checked balances and next payday, preview without writing, client isolation, lock/wrong password/unlock, full process restart and original saved date |
| Final calendar visual inspection | Passed after widening cells and adding horizontal scrolling to keep amounts readable in narrow client panels |
| Deferred Windows download metadata | Parsed with PowerShell; packaging not invoked |
| New Windows installer | Intentionally not built, as requested |
| Windows source checks | Run [38492199152](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38492199152) succeeded from 75d3934: production frontend build, 40 interface tests and 54 Rust tests passed, including the Windows-only SQLCipher memory-lock regression |
| Clean Windows 11 offline installed cash-flow workflow | Not run |

Final native artifacts: `/workspace/shared/desktop-smoke-cashflow-verified`.
The initial native run stopped on a transient read-only text GET connection
closure after setup. Driver logs confirmed the connection closed before the
response completed. The harness now retries one GET observation, matching its
existing one-retry locator handling; typing, clicks, saves, and JavaScript are
never replayed. The rerun and final layout run passed the full required workflow.
An initially incomplete TypeScript test fixture was corrected before the successful
production build; one ambiguous test locator was narrowed to the account-name field.

Source commit `75d393425d5db79b10913aa14abf70e5dd619048` is uploaded to main.
Windows CI explicitly skipped installer build, install/launch/uninstall checks,
metadata preparation, installer upload, and validation-log artifact upload.
No manual package run was requested. The full Windows test log is preserved at
`/workspace/shared/cashflow-windows-validation.log`. This confirms Windows source
build/tests, not a newly installed Windows GUI or clean-machine offline acceptance.

Recoverable source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-cashflow-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-cashflow.bundle`; the archive contents
and full-history bundle are verified and refreshed after the final notes commit.
These are development source checkpoints, not backups of client workspace data.

Source-only checkpoint; no installer, bank integration, actual transaction
reconciliation, overdraft fees, holds, holiday shifts, or delayed transfer
settlement. Only entered accounts/activity are projected. Fictional records
remain required while supported encrypted recovery and Windows acceptance are
unfinished. The earlier 0.3.0 download contains neither savings nor cash flow.

## Stage 3C: emergency savings and sinking funds — 0.4.0 source

Implemented persistent client savings plans: emergency targets for 1/3/6/custom
whole months of necessary expenses; liquid/restricted/bill/sinking-fund reserve
separation; current emergency coverage; remaining amounts; capped monthly
contributions and estimated completion dates; dated sinking funds with suggested
contributions rounded upward; alternative emergency contributions; and monthly
budget/debt funding review. The counselor can record priorities and adjust
allocations; the application never moves money or silently rewrites other plans.

Reserve buckets partition one liquid pool, preventing double counting. Unknown
inputs remain unknown; missing expense bases or zero contributions do not invent
goal dates. Month-end contributions respect actual deadline days and leap years.
The last transfer is capped and stops at the target. Joint funding checks reserve
the higher of budget/debt payments and include future debt changes/extras, even
after savings finish. Overallocations and conflicting/missing references are
flagged. Scenario affordability is unavailable when it cannot be checked.
See [savings-calculations.md](savings-calculations.md) for assumptions and examples.

Encrypted schema 5 preserves clients, notes, budgets, debts, and immutable audit
history. Writes/audit are atomic, optimistic revisions reject competing windows,
failed loads cannot replace a plan, and locking removes private UI state.
Refreshing budget/debt references keeps unsaved savings drafts and cannot adopt
another window's changed revision. The older versions cannot open an upgraded
schema 5 workspace.

| Check | Result |
|---|---|
| Locked setup and production frontend build | Passed |
| Frontend tests | 32 passed: exact inputs, native forecasts, target choices, goals/scenarios, safe reference refresh, failed loads/saves, missing affordability, stale responses, and lock/leave guards |
| Linux Rust tests | 42 passed: independent reserve/coverage/rounding/deadline/last-cent/scenario examples, zero/missing/funded states, leap days, past dates, caps/horizon, debt events, scope, competing writes, audit rollback, and schema 1/2/3/4 migrations |
| Independent Python Decimal examples | Confirmed remaining 3,899.99; 200/month requires 20 months with final 99.99; 100/month needs 39 months; 300/month needs 13; 200 over 3 eligible months requires 66.67/month; coverage floors to 0.61 |
| Rust format and Clippy, all targets with warnings denied | Passed |
| Final compiled native desktop workflow | Passed: visible savings/reserve/goal/scenario editing, known independent results, cap at final transfer, client isolation, lock/wrong password, and full process restart with saved savings |
| Windows source checks | Run 38489872140 succeeded from e3e06dc; conclusion confirmed during the cash-flow stage after GitHub access returned |
| New Windows installer/package | Intentionally not built, per request to save packaging time |
| Clean Windows 11 installed/offline savings acceptance | Not run |

Native artifacts: `/workspace/shared/desktop-smoke-savings-final`. The initial
run passed financial checks but lost one read-only native-driver locator connection
at relaunch. The harness now retries that read once; typing, clicks, and saves
are never replayed and all workflow assertions remain required. The final run
passed fully. An incorrect expected last-transfer value in one fixture was
corrected to 99.99 after independent Decimal verification; the calculator already
returned that correct value.

The Windows workflow now always runs frontend/backend checks on normal pushes
and pull requests. Installer creation, installer smoke checks, metadata, and
artifact upload require a manual dispatch with `package_installer: true`.
The default is false. This preserves source validation without creating downloads
for each milestone. The 0.3.0 installer remains the earlier download and does
not contain the savings feature. This milestone remains for fictional records,
with no interest/returns, inflation, withdrawals, changing savings expenses,
automatic funding redistribution, recovery, or independent security certification.

### Preserved checkpoint and remaining CI confirmation

Source commit `e3e06dc84a031fdc8e98605ee3d410e323db408e` was successfully pushed
before authentication stopped working. Windows source-check run
[38489872140](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38489872140)
was observed with successful frontend checks and the backend step in progress.
Later authenticated API reads returned `HTTP 401: Bad credentials`; a fresh
uncached request also failed, and normal Git remote access could no longer
authenticate. Runtime status exposed no configured credential refresh capability.
No credential values were inspected or printed, no alternate identity was used,
and no interactive login or replacement token was requested.

At the end of this savings stage, the final Windows result and skipped-package
step records were unverified; this was not a reported build/test failure.
During the cash-flow stage, normal injected Git/CLI access returned and run
38489872140 was confirmed successful. The local notes are included in the next
source push. The workflow definition
makes installer/package upload steps conditional on manual opt-in, and no such
run was requested. Local checks passed and the complete source was already
uploaded. Final validation notes were committed locally for later synchronization.

Recoverable source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-savings-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-savings.bundle`. They are development
source archives, not backups of client workspace data. Recheck CI and synchronize
any local documentation commits when GitHub authentication is available again.

## Stage 3B: monthly debt payoff planning — 0.3.0

Implemented saved client debt accounts and exact fixed-monthly APR payoff
projections, with comparable snowball/avalanche strategies. Each month reserves
entered minimums, allocates extra money by strategy, caps payments at the amount
owed, and rolls excess to the next account. Plans support known monthly fees,
one-time extra payments, and future changes to the total monthly payment budget.
The interface exposes estimated payoff months, interest/fee/payment totals,
remaining balances, and an interactive per-account monthly schedule.

Money uses exact cents; APR uses exact 0.0001-percentage-point units and 128-bit
intermediate multiplication. No money calculation occurs in React. Missing
active terms or unmodeled interest methods block payoff projections while still
allowing incomplete accounts to be saved. Shortfalls stop before unaffordable
payments; growing balances, horizon/safety limits, unverified terms, and saved
budget-capacity/allocation conflicts are flagged. Partial totals cover only
simulated months, with no invented debt-free date. Both strategies share all
payment events and assumptions. See [debt-calculations.md](debt-calculations.md).

Debt inputs are encrypted in schema 4 with firm/counselor/client scope, optimistic
revisions, atomic writes/audit, and rollback on failure. The migration preserves
clients, notes, budgets, and immutable audit history. Calculated projections are
recomputed and do not overwrite budgets or historical transactions. Private
editor state clears on lock; failed loads and delayed client/session responses
cannot become replacement plans. Unsaved debt edits participate in leave prompts.

Executed checkpoint checks:

| Check | Result |
|---|---|
| Locked setup and production frontend build | Passed |
| Frontend tests | 25 passed: input strings/revisions, supported/unknown terms, schedule inspection, events, failed loads/saves, stale responses, unsaved prompts, and lock privacy |
| Linux Rust tests | 31 passed: independent amortization/comparison/final-payment examples, zero interest, fees/events, minimum shortfall, negative amortization, horizon/model limits, rate/date validation, ties, budget context, scope, competing windows, failed writes, and schema 1/2/3 upgrades |
| Independent Python Decimal fixtures | Single loan: 58.98 interest, 1,058.98 paid, 11 months; two debts: snowball 43.10 interest/643.10 paid, avalanche 35.35 interest/635.35 paid, both 7 months |
| Native desktop build and workflow | Passed on the initial complete compiled bundle: visible two-debt editing, independent strategy totals/allocations, budget coexistence, client isolation, lock/wrong password, and full process restart |
| Final Rust format, Clippy (all targets, warnings denied), backend and native checks | Passed on final source after the allocation-warning improvement |
| Windows 0.3.0 build/installer | Passed run 38485910521: 25 frontend/32 Windows backend tests, release build, silent install, responsive x64 native window, uninstall, and isolated cleanup |
| Clean Windows 11 offline/full installed debt workflow | Not run |

Final native artifacts are at `/workspace/shared/desktop-smoke-debt-final`; the
initial run is retained at `/workspace/shared/desktop-smoke-debt`.
This is a development milestone for fictional accounts. It implements fixed
monthly estimates, not daily accrual, variable/promotional APRs, changing lender
minimum formulas, or nonmonthly contractual payments. Source/account-detail
expansion, actual due-date cash flow, savings goals, recovery, and independent
review remain open. The 0.3.0 upgrade prevents older 0.1.0/0.2.0 apps from opening
schema 4; keep using the newer version and preserve the local data folder.

### Verified 0.3.0 Windows download

[Download the debt-planning development installer](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38485910521/artifacts/11859772481).
[Run 38485910521](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38485910521)
succeeded from source commit `631719ba5811c869b4190603c8eddcef53d82b41`.
The package is `Eisenbart Counseling Suite_0.3.0_x64-setup.exe`; its artifact
expires October 27, 2026 and may require GitHub sign-in with repository access.

The actual downloaded ZIP matched GitHub's digest and passed ZIP integrity. The
installer matched `SHA256SUMS.txt`; its README matched the source commit, version,
debt features, and schema-upgrade notice. Downloaded logs confirmed 25 frontend
and 32 backend tests passed, including the real Windows lock-quota regression.
The separate package log confirmed installation, responsive x64 native window,
uninstall, file removal, and removal of only test-created application data.
Archive inspection also verified the bundled WebView2 offline runtime payload.

Installer SHA-256:
`3965d9f9af6a425b1afa0b0ba39cb93ec776da95fd520e194755184bc8745914`.
Installer size: 220,951,701 bytes. ZIP size: 221,001,487 bytes.
Installer artifact: `11859772481`; validation artifact: `11859682545`.
Verified local downloads/logs: `/workspace/shared/windows-downloads/run-38485910521`.
Recoverable source checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-debt-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-debt.bundle`; these are source/history
archives, not backups of client workspaces.

The installer is unsigned and for fictional accounts only. Windows CI launched
the installed native window but entered no client/budget/debt workflow and used a
runner with preinstalled WebView2. Linux native workflow evidence and Windows
backend/package checks do not establish clean Windows 11 offline or full
installed debt-workflow acceptance. Those checks remain open.

## Stage 3A: exact client budgets — 0.2.0

Implemented the first financial-core checkpoint: one persistent USD budget per
client, custom category/source lines, take-home income frequency normalization,
necessary/discretionary spending, proposed savings, planned debt payment
allocations, and signed surplus/deficit results. Amounts and calculations use
integer cents in Rust; the interface receives exact decimal strings. Weekly,
biweekly, semimonthly, monthly, annual, and irregular monthly-estimate inputs are
supported. Missing amounts and estimates are visibly flagged. Original planning
amounts/frequencies remain stored, and no historical transactions are changed.
See [budget-calculations.md](budget-calculations.md) for rounding, formulas,
limits, and independently checked examples.

Budget saving is authenticated and scoped to the current firm/counselor's client.
Encrypted schema 3 adds budgets and preserves earlier clients, notes, and audit
history. Saves and content-free audit events are atomic, with rollback on failure.
Revision checking prevents another desktop window's older plan from overwriting
a newer one. Failed loads cannot become empty writable replacements. Unsaved
edits participate in leave-record prompts; editing hides outdated totals; locking
clears budget state; delayed responses cannot populate another client's editor.

Executed validation for this checkpoint:

| Check | Result |
|---|---|
| Locked setup and production frontend build | Passed |
| Frontend tests | 18 passed; includes exact input payloads, native result display, missing/zero distinction, estimated irregular income, failed loads, private-state clearing, and stale responses |
| Linux Rust tests | 20 passed; includes independent rounding/totals/deficit examples, invalid/large inputs, encrypted persistence, scope, competing connections, atomic rollback, and schema 1/2 upgrades |
| Independent Python Decimal calculation | Matched the documented household example: 3,250.00 income, 1,633.33 necessary expenses, 716.67 unallocated |
| Rust format and Clippy (`--all-targets`, warnings denied) | Passed |
| Native desktop build | Passed with bundled production frontend |
| Native desktop workflow | Passed: visible budget editing, exact six-line example, encrypted saving, client isolation, locking, wrong-password rejection, and complete process restart with persisted budget |
| Windows 0.2.0 installer | Passed run 38482971562: 18 frontend/21 Windows backend tests, release build, silent install, responsive x64 native window, uninstall, and isolated test-data cleanup |
| Clean Windows 11 offline/full installed budgeting workflow | Not run |

Native artifacts: `/workspace/shared/desktop-smoke-budget-final`. An initial
cloud native-driver attempt lost a connection while typing intake; the app
remained responsive for a failure screenshot. The harness now identifies the
failed method/endpoint without printing input payloads. A fresh final build/run
completed all workflow assertions without retrying commands or weakening checks.

Version 0.2.0 upgrades the local database to schema 3. Version 0.1.0 will refuse
that newer schema; switching back is unsupported. The development build remains
for fictional records only, with no supported backup/restore or password recovery.
This is not the full Stage 3 financial engine: optional budget methods, debt
payoff schedules, savings targets, cash-flow dates, and scenarios remain open.

Windows run [38482182798](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38482182798)
finished application validation but failed test-data cleanup when WebView2 still
held `EBWebView/lockfile`. No installer download was uploaded from that run.
Cleanup retains both known-folder ownership checks and the disposable-runner
restriction, waits at most 45 seconds for I/O failures, fails immediately for
other errors, and still requires the test-created directory to be removed.
It does not kill unrelated browser processes or skip the cleanup check.

### Verified 0.2.0 Windows download

[Download the budget development installer](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38482971562/artifacts/11857823874).
[Run 38482971562](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38482971562)
completed successfully from commit `ff5d2e2402f8e7561adfb96f710d5f2ec62fd579`.
The installer is `Eisenbart Counseling Suite_0.2.0_x64-setup.exe`.
GitHub sign-in may be required; artifact retention expires October 27, 2026.

The final downloaded ZIP matched GitHub's SHA-256 digest and passed ZIP
integrity. The installer matched the bundled checksum; its README matched the
exact source commit, version, budget features, and schema-upgrade notice.
Downloaded Windows logs independently confirmed all 18 frontend and 21 backend
tests passed, including the real memory-lock quota regression. The validation
log confirmed native installation, x64 executable/window, successful uninstall,
and removal of only test-created application data. The bounded cleanup
correction passed the complete rerun. NSIS archive inspection passed and found
the 214,090,448-byte WebView2 offline runtime installer payload.

Installer SHA-256:
`663f1c65e8c1919fffa91917590318a703e31f32541c151f29edb86874cb2cc5`.
Installer size: 220,903,103 bytes. ZIP size: 220,952,850 bytes.
Installer artifact: `11857823874`; validation artifact: `11858447750`.
Local verified files/logs: `/workspace/shared/windows-downloads/run-38482971562`.
The native budget implementation checkpoint is `84f474b`; the subsequent
`ff5d2e2` commit changes only Windows test cleanup and documentation.

This package remains unsigned and for fictional records. Windows CI entered
no client/password/budget workflow, and had preinstalled WebView2. Linux native
workflow evidence and Windows backend/package checks do not establish clean
Windows 11 offline or full installed budgeting acceptance. Those checks and
backup/restore remain open.

## Windows development installer milestone — October 13, 2026

**Download available:** [windows-installer ZIP](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38450281912/artifacts/11842130980).
The verified installer is `Eisenbart Counseling Suite_0.1.0_x64-setup.exe`, built
from source commit `370d4414db381edfe3686e1106d15ea6471e1238` on `main`.
[Windows run 38450281912](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38450281912)
completed successfully. The download expires October 27, 2026 and may require
GitHub sign-in with repository access. Extract the ZIP, read `README.txt`, and
run the installer. See [user-guide.md](user-guide.md) for first use.

Executed on the disposable GitHub-hosted `windows-2025` runner:

- Locked frontend production build and **11 frontend tests passed**.
- Locked MSVC Rust build and **12 backend tests passed**, including the real
  Windows memory-lock quota regression, encrypted persistence, integrity,
  incorrect-password rejection, and scope protections.
- Release NSIS installer built successfully with bundled WebView2 offline runtime.
- Silent installation returned exit code 0; the installed program's PE machine
  was AMD64 (`0x8664`). It displayed a responsive native window with the expected
  title. The isolated test stopped its own process, silently uninstalled with
  exit code 0, and verified removal of the installed files.
- Download instructions/checksums and separate validation logs uploaded.

The installer is **unsigned** and remains a development build for fictional
records. The Windows package test entered no password, counselor account, or
client workflow; it tested installation and native-window launch. The runner
already had a browser runtime. A clean Windows 11 offline install without
preinstalled WebView2, full installed client workflows, and upgrade/data-retention
acceptance remain unrun. No security certification or commercial readiness is
claimed.

Independent checks after downloading the actual artifacts:

| Artifact check | Result |
|---|---|
| GitHub installer ZIP SHA-256 | Matched GitHub's digest; ZIP integrity passed |
| Installer SHA-256 | Matched bundled `SHA256SUMS.txt` |
| Download README source commit | Matched the successful run's full source commit |
| Windows validation log | Confirmed installation, responsive x64 app, uninstall, and isolated cleanup passed |
| NSIS archive integrity | Passed 7-Zip archive test |
| Offline runtime payload | Found 214,090,448-byte `MicrosoftEdgeWebView2RuntimeInstaller.exe` inside the installer |

Installer SHA-256:
`d6faff55f9b5d9db6c8f6783ed64eae1ea5e17e587d50e15e30446f11f831408`.
The installer is 220,875,970 bytes; the download ZIP is 220,925,577 bytes.
Artifact ID: `11842130980`. Validation-log artifact ID: `11841143510`.
Local verified downloads and run logs are retained under
`/workspace/shared/windows-downloads/run-38450281912`.

### Windows crash correction preserved in this checkpoint

The initial Windows run
[38445920011](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38445920011)
built the frontend and Rust code but aborted its tests with a stack overflow;
that run produced no installer. Locked SQLCipher 4.14.0 source and its error log
identified recursive native logging: failed `VirtualLock` allocations log a
memory warning through a formatter that itself allocates through SQLite.

The Windows-only correction configures native log sources once before memory
security activates, excluding MEMORY while retaining CORE, PROVIDER, and MUTEX
diagnostics. Encryption, Argon2id, memory wiping, memory-lock attempts, integrity
checks, and application errors are preserved. Memory locking remains best effort;
native memory-lock failure warnings are omitted on Windows. The Windows-only
subprocess regression forces an actual lock-quota failure and now passes on
Windows. See [security.md](security.md) for this limitation and review needs.

After the correction, the existing 11 Linux backend tests passed again and the
compiled Linux desktop passed the complete synthetic setup/client/note/lock/
restart workflow again. Those artifacts are retained at
`/workspace/shared/desktop-smoke-windows-fix`. The Windows packaging scripts also
passed PowerShell parsing and synthetic guard/header/checksum checks before CI.

GitHub API, logs, and artifact downloads worked through the existing injected
authentication after the environment network settings were updated. No new
credentials were requested. Runtime configuration metadata still reports unknown
network enforcement; successful commands establish access to these endpoints,
not a general network-readiness claim.

## Foundation milestone — October 13, 2026

This earlier checkpoint combined specification stages 1 and 2: an established
repository and a functioning desktop foundation. Advanced financial modules had
not been started at that checkpoint. It was a development build for fictional
records only; the later budget milestone is recorded above.

Implemented:

- Tauri 2 desktop host, React/TypeScript interface, original application icons,
  bundled local assets, and Windows NSIS packaging configuration.
- First-run organization and counselor setup with editable Eisenbart & Co.
  default branding; local password unlock; manual and 15-minute inactivity lock.
- SQLCipher-encrypted SQLite storage, Argon2id key derivation, key/password/KDF
  memory cleanup, schema migrations, integrity checks, and guarded setup.
- Persistent client intake, contact details, household size, goals, lifecycle
  status, searching, filtering, and editing.
- Append-only counselor notes with authorship/timestamps, operational audit
  records without note/contact contents, and server-enforced firm/counselor scope.
- Real client counts and lifecycle summaries; no invented financial results.
- Session and client-switch guards that discard stale responses and clear private
  interface data when locked.
- Reusable cloud setup/activation, backend/frontend tests, native desktop smoke
  harness, Windows build workflow, security notes, and staged roadmap.

The original spreadsheet was unavailable and has not been verified or changed.
No real client information was imported.

## Executed checks

Machine: managed Debian 13 Linux x64, Node 24.19.0, Rust 1.99.0, GTK 3.24.49,
WebKitGTK 2.54.0. Native packages were verified against signed Debian metadata.
The writable native sysroot avoids privileged installation.

| Check | Result |
|---|---|
| `bash scripts/cloud-setup.sh` | Passed; locked npm installation and Cargo fetch, tool activation and native dependency checks |
| `npm run build` | Passed; TypeScript and bundled production frontend |
| `npm test` | Passed: 11 tests, including setup, editing, client-note isolation, lock clearing, and stale-session failures |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline -- --test-threads=2` | Passed: 11 backend tests; persistence, encryption, wrong keys, validation, scope, append-only history, migrations, corruption and existing-file preservation |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed |
| `npm run tauri -- build --debug --no-bundle` | Passed; standalone Linux desktop binary with compiled frontend assets |
| Native GUI workflow | Passed with exit 0 on the final compiled bundle, including a complete app/driver process restart and record persistence |
| Windows executable/NSIS installer build | Passed on run 38450281912; automated install/native-window launch/uninstall passed |
| Clean Windows 11 offline install/use | Not run |
| Independent security review and commercial release review | Not performed |

Binary: `src-tauri/target/debug/eisenbart-counseling-suite`. This Linux development
binary is not a Windows executable or installer. `dist/` is generated frontend
output; opening it alone is not the desktop application.

The native test uses real Tauri IPC and compiled assets, not a browser mock. It
creates a fictional firm/counselor and two clients, saves an authored note, checks
the second client has no such note, edits status/goals, verifies locking clears
private DOM values, rejects a wrong password, unlocks, and checks records after
relaunch. The final test terminates all its private native app/driver processes
and starts a fresh driver against the same temporary data. Temporary application
data is removed afterward. Early test-harness issues with CSS-clipped identity
text and native WebKit session teardown were corrected using full identity values
and a full process restart. All final workflow assertions passed. No live test
processes or driver listeners remained.

Final native command, from the checkout after `source scripts/cloud-env.sh`:

```bash
WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 \
WEBKIT_DISABLE_DMABUF_RENDERER=1 LIBGL_ALWAYS_SOFTWARE=1 \
dbus-run-session -- xvfb-run -a -s '-screen 0 1280x900x24 -nolisten tcp' \
  bash scripts/cloud-native-run.sh python3 scripts/desktop-smoke.py \
  --artifacts /workspace/shared/desktop-smoke-foundation-final-process-restart
```

The native screenshot and driver diagnostics are retained at that artifacts
location. No development server was running or required for the tested app.

SQLCipher prints expected HMAC/decryption diagnostics in intentionally wrong-key
and corrupted-file checks. They contain no test passwords or client contents.
The tests exercise actual encrypted-page authentication; these diagnostics are
not failed test outcomes.

## Reusable environment configuration

The environment draft has saved `install_script` and `start_skill`. Installation
calls the repository's tested `scripts/cloud-setup.sh`; startup instructions cover
tool activation, builds, checks, and new-process native GUI validation. No new
secret bindings were required. Later Windows artifact validation used the
user-reviewed GitHub API, Actions log receiver, and Azure Blob download domains.

The trusted synthetic Linux GUI test needs an explicit WebKit sandbox exception
because this cloud container denies WebKit's nested sandbox. It remains confined
to test processes; product configuration and default helpers preserve the sandbox.
This does not validate Windows WebView2 isolation or certify security.

Draft saving does not publish the environment or demonstrate restoration in a
new task. Review and save the draft in environment settings, then publish the
environment through the product. The foundation source checkpoint is now
uploaded to GitHub. Retained cloud files, uploaded Git history, and restoration
in a newly created cloud task are separate outcomes; a fresh-task restore has
not been verified.

Recoverable source checkpoints are retained outside the checkout at
`/workspace/shared/checkpoints/eisenbart-foundation-source.tar.gz` (tracked
source) and `eisenbart-foundation.bundle` (Git history). These are development
source checkpoints, not backups of application client data. A fresh-task restore
has not been verified; retain or upload the source before relying on a new remote
clone.

## Next development priority

Encrypted backup/restore is implemented with authenticated files, retained
previous workspaces, and tested interrupted/failure recovery. Clean installed
Windows acceptance and independent security review remain prerequisites for
actual client use. The first local CSV import/review checkpoint is implemented with bounded parsing, immutable provenance, duplicate/statement checks, explicit decisions, approved-only exact totals, refunds, transfers, and encrypted recovery. Selectable-text PDF extraction now feeds the same review workflow with source page/line locators and mandatory individual verification. Merchant grouping and recurring-payment review now add conservative suggestions and explicit evidence-bound counselor decisions. Statement running-balance reconciliation now adds source/approved comparisons and immutable order settings. The approved-transaction dashboard now provides explicit periods, local category/monthly/cash-flow charts and completeness flags. Explicit approved classifications and dated balance facts now supply the remaining recorded dashboard metrics. Immutable internal assessment records now preserve approved figures and evidence. The next application priority is **offline PDF report export and scenario-backed client action plans**; keep scanned OCR and broad institution adapters in later checkpoints.

Advanced financial models, broader scenarios, finalized PDF reports, multiple counselor roles,
password change/reset, exports, licensing, signing, retained-copy selection,
scheduled backups, and commercial readiness remain incomplete. See
[roadmap.md](roadmap.md) and [windows.md](windows.md) for staged criteria.

## Distance to the complete product

Stages 1–2 provide the implemented architecture and desktop foundation; the
unavailable original workbook remains unverified. A tested Windows development
package also completes part of Stage 7. Stages 3–8 are still open:

| Specification stage | Remaining work |
|---|---|
| 3: Financial core | Exact budgets, fixed-monthly debt schedules, savings goals, and dated cash-flow calendars implemented; optional budget methods, advanced debt terms, delayed transfer settlement, and broader scenario engines remain |
| 4: Document processing | CSV, selected text-PDF review, recurring/grouped payment review and running-balance reconciliation implemented; scanned OCR and broader institution adapters remain |
| 5: Dashboard and reports | Approved activity and debt charts plus recorded balance/classification metrics implemented; savings progress, completed scenario-backed counselor reports/client plans and reproducible offline PDFs remain; internal immutable assessments are implemented |
| 6: Security/commercial | Encrypted backup/restore implemented; clean Windows recovery, password safeguards, accounts/roles, branding, licensing and independent review remain |
| 7: Production packaging | Signed distribution, upgrade behavior, remaining administrator/recovery/support guides |
| 8: Final verification | Complete installed workflows, clean Windows 11 offline tests, usability and release acceptance |

These stages differ greatly in size and will be split into smaller tested
checkpoints. A stage count is not a percentage or a reliable calendar estimate.
The complete specified application remains substantial work; the development
installer is suitable for evaluating the implemented foundation.
