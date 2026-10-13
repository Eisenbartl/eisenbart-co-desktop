# Native desktop smoke test

`scripts/desktop-smoke.py` launches the compiled Tauri application through
`tauri-driver` and a native WebDriver. It uses Python 3's standard library;
Selenium and a development web server are not needed.

The test creates a temporary workspace containing fictional clients. It verifies
first-run organization and counselor setup, client creation and editing, session
notes and saved budgets belonging to the correct client, independently checked
monthly budget totals and allocations, snowball/avalanche interest totals and
account payment schedules, saved debt isolation, workspace locking, rejection of an
incorrect password, unlocking, and persistence after stopping and relaunching the
application and its native driver. It confirms the app responds before stopping
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

This test checks desktop behavior and persistence. It does not audit encryption,
prove the absence of outbound network traffic, or certify production security.
