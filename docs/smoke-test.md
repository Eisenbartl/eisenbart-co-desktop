# Native desktop smoke test

`scripts/desktop-smoke.py` launches the compiled Tauri application through
`tauri-driver` and a native WebDriver. It uses Python 3's standard library;
Selenium and a development web server are not needed.

The test creates a temporary workspace containing fictional clients. It verifies
first-run organization and counselor setup, client creation and editing, session
notes and saved budgets belonging to the correct client, independently checked
monthly budget totals and allocations, snowball/avalanche interest totals and
account payment schedules, emergency/sinking-fund forecasts and contribution
scenarios, dated account balances, paired transfers, calendar/ledger reviews, non-writing date previews, saved debt/savings/cash-flow isolation, workspace locking, rejection of an
incorrect password, unlocking, and persistence after stopping and relaunching the
application and its native driver. It also imports synthetic checking/savings CSVs through visible controls, explicitly maps columns, verifies no automatic approval, bulk-approves validated rows, individually approves a refund, reverses an approved fee with history, pairs equal opposite transfers, rejects a repeated exact source, and checks independently known totals, source retention, restart/client isolation, and backup-restored decisions/history. It additionally creates an encrypted backup, rejects the wrong backup password, interrupts a verified review by stopping the entire process group, confirms original records still open, explicitly restores the backup, verifies the original files remain, and recovers from missing metadata through the storage-error screen. It also checks approved financial dashboard periods, local category/monthly/cash-flow charts, exclusion of pending PDF spending, automatic recalculation after corrections/reversals, and recreation from persisted/restored sources. It also verifies source-bound necessary/discretionary/payroll classifications, dated complete inventory, exact net worth and conservative ratios, correction/reversal invalidation, explicit re-verification, immutable history, and fact persistence through restart and both recovery paths. All mutations target temporary fictional records. It confirms the app responds before stopping
its entire private process group, verifies no live process remains in that group,
and launches a fresh driver against the same temporary data directory. This
avoids WebKitGTK session teardown/reuse races and checks storage across a complete
process restart. It also checks that the application is running inside Tauri and that
private workspace data leaves the page when locked. The test password is randomly
generated and never printed. Temporary application data is removed on completion.

## Linux cloud environment

Build the frontend and native application with the repository's documented
toolchain first. A built application with embedded frontend assets is required;
this test does not start Vite. Provide an X display (a desktop or Xvfb),
`tauri-driver`, and the `WebKitWebDriver` matching the installed WebKitGTK runtime.

```bash
python3 scripts/desktop-smoke.py \
  --binary src-tauri/target/release/eisenbart-counseling-suite \
  --driver /path/to/tauri-driver \
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
