#!/usr/bin/env python3
"""Exercise the actual Tauri desktop application with W3C WebDriver.

Uses only Python's standard library. Never connects to a development web server
or invokes application commands directly. All changes use the visible UI.
"""

from __future__ import annotations

import argparse
import base64
import http.client
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
from typing import Callable, TypeVar
import urllib.error
import urllib.parse
import urllib.request
from synthetic_pdf import build_pdf


ELEMENT_KEY = "element-6066-11e4-a52e-4f735466cecf"
T = TypeVar("T")


class SmokeFailure(RuntimeError):
    """A prerequisite or user-visible behavior did not meet expectations."""


class DriverFailure(SmokeFailure):
    def __init__(self, code: str, detail: str):
        self.code = code
        super().__init__(detail)


class WebDriver:
    def __init__(self, port: int, protected_values: list[str]):
        self.base = f"http://127.0.0.1:{port}"
        # Local WebDriver traffic must stay on loopback, independent of outbound
        # HTTP proxy configuration. No external connections use this opener.
        self.http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        self.session_id: str | None = None
        self.protected_values = protected_values

    def safe(self, value: str) -> str:
        for protected in self.protected_values:
            value = value.replace(protected, "[redacted]")
        return value

    def request(self, method: str, path: str, payload: object = None,
                timeout: float = 45) -> object:
        data = None if payload is None else json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(
            self.base + path, data=data, method=method,
            headers={"Content-Type": "application/json"},
        )
        try:
            with self.http.open(request, timeout=timeout) as response:
                result = json.load(response)
        except urllib.error.HTTPError as error:
            try:
                result = json.load(error)
            except (ValueError, OSError):
                raise DriverFailure("http error", f"WebDriver returned HTTP {error.code}") from None
        except (urllib.error.URLError, TimeoutError, OSError, http.client.HTTPException) as error:
            raise DriverFailure("connection error", self.safe(f"WebDriver unavailable during {method} {path}: {error}")) from None
        value = result.get("value", result)
        if isinstance(value, dict) and "error" in value:
            code = str(value["error"])
            detail = self.safe(str(value.get("message", code)))
            raise DriverFailure(code, f"WebDriver {code}: {detail}")
        return value

    def command(self, method: str, path: str, payload: object = None) -> object:
        if self.session_id is None:
            raise SmokeFailure("No native desktop session is open")
        # Only GET observations are idempotent. Retry a single native-driver
        # connection closure; never replay POST actions or financial writes.
        for attempt in range(2):
            try:
                return self.request(method, f"/session/{self.session_id}{path}", payload)
            except DriverFailure as error:
                if method != "GET" or error.code != "connection error" or attempt == 1:
                    raise
                time.sleep(0.25)
        raise SmokeFailure("Read-only driver retry did not complete")

    def open(self, binary: Path) -> None:
        value = self.request("POST", "/session", {
            "capabilities": {"alwaysMatch": {
                "tauri:options": {"application": str(binary)},
            }},
        }, timeout=90)
        if not isinstance(value, dict) or not value.get("sessionId"):
            raise SmokeFailure("Native driver did not create a desktop session")
        self.session_id = str(value["sessionId"])

    def close(self) -> None:
        if self.session_id is not None:
            try:
                self.command("DELETE", "")
            finally:
                self.session_id = None

    def script(self, script: str, args: list[object] | None = None) -> object:
        return self.command("POST", "/execute/sync", {"script": script, "args": args or []})

    def observe_script(self, script: str) -> object:
        # Call only for the explicit DOM/runtime observations below. A closed
        # native-driver connection may be retried once; mutations use script().
        for attempt in range(2):
            try:
                return self.script(script)
            except DriverFailure as error:
                if error.code != "connection error" or attempt == 1:
                    raise
                time.sleep(0.25)
        raise SmokeFailure("Read-only script observation did not complete")

    def find(self, selector: str, using: str = "css selector") -> str:
        # Finding an element is read-only. WebKitWebDriver can occasionally
        # close a locator connection during native startup/teardown. Retry one
        # such read; never replay typing, clicks, saves, or other mutations.
        for attempt in range(2):
            try:
                value = self.command("POST", "/element", {"using": using, "value": selector})
                break
            except DriverFailure as error:
                if error.code != "connection error" or attempt == 1:
                    raise
                time.sleep(0.25)
        if not isinstance(value, dict) or ELEMENT_KEY not in value:
            raise SmokeFailure(f"Invalid element response for {selector}")
        return str(value[ELEMENT_KEY])

    def visible(self, selector: str, using: str = "css selector") -> str | None:
        try:
            element = self.find(selector, using)
            return element if self.command("GET", f"/element/{element}/displayed") else None
        except DriverFailure as error:
            if error.code in ("no such element", "stale element reference"):
                return None
            raise

    def wait(self, condition: Callable[[], T], description: str, seconds: float = 40) -> T:
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            try:
                value = condition()
                if value:
                    return value
            except DriverFailure as error:
                if error.code not in ("no such element", "stale element reference"):
                    raise
            time.sleep(0.15)
        raise SmokeFailure(f"Timed out waiting for {description}")

    def element(self, selector: str, using: str = "css selector") -> str:
        return self.wait(lambda: self.visible(selector, using), f"visible {selector}")

    def interactive(self, selector: str, using: str = "css selector") -> str:
        def enabled() -> str | None:
            element = self.visible(selector, using)
            if element and self.command("GET", f"/element/{element}/enabled"):
                return element
            return None
        return self.wait(enabled, f"enabled {selector}")

    def click(self, selector: str, using: str = "css selector") -> None:
        element = self.interactive(selector, using)
        # WebKit's automatic scrolling can leave a partially visible control
        # at the viewport edge and click its old position. Center first; this
        # changes only the viewport and never invokes application commands.
        self.script("arguments[0].scrollIntoView({block:'center',inline:'nearest',behavior:'instant'});",
                    [{ELEMENT_KEY: element}])
        self.command("GET", f"/element/{element}/displayed")
        self.command("POST", f"/element/{element}/click", {})

    def click_button(self, text: str) -> None:
        self.click(f"//button[normalize-space(.)={json.dumps(text)}]", "xpath")

    def open_client(self, name: str) -> None:
        self.click("nav[aria-label='Main navigation'] button:nth-child(2)")
        self.click("//button[contains(concat(' ', normalize-space(@class), ' '), ' client-row ')]"
                   f"[.//strong[normalize-space(.)={json.dumps(name)}]]", "xpath")

    def fill(self, selector: str, value: str) -> None:
        element = self.interactive(selector)
        self.command("POST", f"/element/{element}/clear", {})
        self.command("POST", f"/element/{element}/value", {"text": value})

    def select(self, selector: str, value: str) -> None:
        # WebDriver clicks the HTML option rather than calling application code.
        select = self.interactive(selector)
        try:
            option = self.command("POST", f"/element/{select}/element", {
                "using": "xpath", "value": f"./option[normalize-space(.)={json.dumps(value, ensure_ascii=False)}]",
            })
        except DriverFailure as error:
            available = self.command("GET", f"/element/{select}/property/textContent")
            raise SmokeFailure(f"Could not select {value!r} in {selector}; choices={available!r}: {error}") from None
        if not isinstance(option, dict) or ELEMENT_KEY not in option:
            raise SmokeFailure("Native driver did not return the requested workflow option")
        self.command("POST", f"/element/{option[ELEMENT_KEY]}/click", {})

    def field_value(self, selector: str) -> str:
        element = self.find(selector)
        return str(self.command("GET", f"/element/{element}/property/value"))

    def wait_value(self, selector: str, value: str, description: str) -> None:
        self.wait(lambda: self.field_value(selector) == value, description)

    def text(self, selector: str = "body") -> str:
        element = self.find(selector)
        return str(self.command("GET", f"/element/{element}/text"))

    def wait_text(self, selector: str, value: str, description: str) -> None:
        self.wait(lambda: value in self.text(selector), description)

    def wait_content(self, selector: str, value: str, description: str) -> None:
        # WebKit's rendered text endpoint can reflect CSS clipping/ellipsis.
        # Identity checks must compare the complete stored value, not a label's
        # abbreviated on-screen rendering.
        def matches() -> bool:
            element = self.find(selector)
            return self.command("GET", f"/element/{element}/property/textContent") == value
        self.wait(matches, description)

    def screenshot(self, path: Path) -> None:
        encoded = self.command("GET", "/screenshot")
        if not isinstance(encoded, str):
            raise SmokeFailure("Native WebDriver did not return a screenshot")
        path.write_bytes(base64.b64decode(encoded, validate=True))


def executable(value: str, label: str) -> Path:
    selected = shutil.which(value) or value
    path = Path(selected).expanduser().resolve()
    if not path.is_file() or not os.access(path, os.X_OK):
        raise SmokeFailure(f"{label} is not an executable file: {path}")
    return path


def ensure_available(port: int) -> None:
    with socket.socket() as listener:
        # Match the native servers' reuse setting: recently closed connections
        # may remain in TIME_WAIT even though every test process has stopped.
        # An active listener still prevents this bind.
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            listener.bind(("127.0.0.1", port))
        except OSError:
            raise SmokeFailure(f"Loopback port {port} is already in use; choose another port") from None


def active_group_members(group: int) -> list[int]:
    """Find live descendants in the private Linux process group we created."""
    members = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        pid = int(entry.name)
        try:
            if os.getpgid(pid) != group:
                continue
            state = (entry / "stat").read_text().rsplit(")", 1)[1].split()[0]
            if state != "Z":
                members.append(pid)
        except (ProcessLookupError, FileNotFoundError, PermissionError):
            continue
    return members


def stop_driver(process: subprocess.Popen[bytes]) -> None:
    # A native driver or webview can outlive its intermediary. Signal and check
    # the private process group even when the intermediary has already exited.
    for stop_signal in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, stop_signal)
        except ProcessLookupError:
            pass
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            process.poll()  # reap our own child when it exits
            if not active_group_members(process.pid):
                return
            time.sleep(0.1)
    raise SmokeFailure("The native desktop test process group did not terminate")


def status(message: str) -> None:
    print(f"PASS {message}", flush=True)


def check_csv_imports(driver: WebDriver, artifacts: Path, data_root: Path) -> None:
    """Visible review workflow with independently known exact-cent results."""
    source = data_root / "external-csv"
    source.mkdir()
    checking = source / "checking.csv"
    checking.write_text("Date,Description,Amount,Category,Type\n"
        "2026-09-01,Synthetic wages,2000,Wages,income\n"
        "2026-09-02,Synthetic rent,-1200,Housing,expense\n"
        "2026-09-03,Synthetic fee,-10.01,Fees,fee\n"
        "2026-09-04,Synthetic refund,20.01,Housing,refund\n"
        "2026-09-05,Synthetic reserve transfer,-200,Savings,transfer\n", encoding="utf-8")
    savings = source / "savings.csv"
    savings.write_text("Date,Description,Amount,Category,Type\n"
        "2026-09-06,Synthetic incoming transfer,200,Savings,transfer\n", encoding="utf-8")

    def inspect(path: Path, account: str, opening: str, closing: str, existing: bool = False) -> None:
        driver.fill("#csv-path", str(path))
        driver.click_button("Inspect CSV")
        driver.wait_text(".csv-preview", path.name, "captured CSV source sample")
        if existing:
            driver.select("#csv-account", account)
        else:
            driver.fill("#csv-account-label", account)
            if account == "Synthetic savings":
                driver.select("#csv-account-kind", "Savings")
        for key, heading in [("date", "1: Date"), ("description", "2: Description"),
                             ("amount", "3: Amount"), ("category", "4: Category"), ("kind", "5: Type")]:
            driver.select(f"#csv-map-{key}", heading)
        for key, value in [("periodStart", "2026-09-01"), ("periodEnd", "2026-09-30"),
                           ("openingBalance", opening), ("closingBalance", closing)]:
            driver.fill(f"#csv-{key}", value)
        retention = "//label[contains(.,'Authorize retaining the complete original CSV')]/input"
        driver.click(retention, "xpath")
        driver.wait(lambda: driver.command("GET", f"/element/{driver.find(retention, 'xpath')}/property/checked") is True,
                    "explicit source retention checkbox selected")
        driver.click_button("Import into review queue")

    inspect(checking, "Synthetic checking", "1000", "1610")
    driver.wait_text(".imports-progress", "Approved 0", "no automatic transaction approvals")
    for index in range(1, 4):
        driver.click(f".imports-table tbody tr:nth-child({index}) input[type=checkbox]")
    driver.click_button("Approve selected validated rows (3)")
    driver.wait_text(".imports-totals", "$1,210.01", "independent expense and fee total")
    driver.click_button("Review checking.csv record 5")
    driver.select("#review-status", "Approved")
    driver.fill("#review-reason", "Verified synthetic merchant refund; offset spending, not income")
    driver.script("document.querySelector('#review-ack').scrollIntoView({block: 'center', behavior: 'instant'})")
    driver.click("#review-ack")
    driver.wait(lambda: driver.command("GET", f"/element/{driver.find('#review-ack')}/property/checked") is True,
                "individual source acknowledgment selected")
    driver.click_button("Save transaction decision")
    driver.wait_text(".imports-totals", "$1,190.00", "refund offsets spending exactly")
    driver.click_button("Review checking.csv record 4")
    driver.select("#review-status", "Rejected")
    driver.fill("#review-reason", "Synthetic fee excluded after individual review")
    driver.click_button("Save transaction decision")
    driver.wait_text(".imports-totals", "$1,179.99", "rejection withdraws earlier fee approval")
    status("CSV import remains unapproved until explicit decisions; refund and fee reversal match independent exact totals")
    inspect(savings, "Synthetic savings", "300", "500")
    driver.wait_text(".imports-progress", "Approved 3", "new source remains unapproved")
    driver.click(".transfer-review > summary")
    driver.select("#transfer-left", "Synthetic checking · 2026-09-05 · -$200.00 · Synthetic reserve transfer")
    driver.select("#transfer-right", "Synthetic savings · 2026-09-06 · $200.00 · Synthetic incoming transfer")
    driver.fill("#transfer-reason", "Verified equal opposite synthetic movements between owned accounts")
    driver.click_button("Approve paired internal transfer")
    driver.wait_text(".imports-progress", "Approved 5 · Rejected 1", "both transfer legs approved atomically")
    for value in ["$2,000.00", "$1,200.00", "$20.01", "$1,179.99", "$820.01", "$620.01", "$200.00"]:
        driver.wait_text(".imports-totals", value, "independently checked approved totals and account activity")
    driver.click(".imports-sources > summary")
    driver.wait_text(".imports-sources", "complete source retained with authorization", "source retention authorization")
    driver.wait_text(".imports-sources", "Approved difference after every row is decided: $10.01", "rejected fee remains visible in statement discrepancy")
    driver.click_button("Review checking.csv record 4")
    driver.click(".review-history > summary")
    driver.wait_text(".review-history", "Approved → Rejected", "immutable reversal history")
    driver.script("document.querySelector('.transaction-review').scrollIntoView({block: 'center'})")
    driver.screenshot(artifacts / "native-csv-review.png")
    inspect(checking, "Synthetic checking", "1000", "1610", existing=True)
    driver.wait_text(".imports-panel [role=alert]", "already", "same-account exact-source duplicate rejected")
    driver.click_button("Cancel CSV preview")
    driver.wait_value("#csv-path", "", "cancel clears transient source path")
    status("CSV provenance, explicit bulk/individual decisions, refund, reversal, transfer pairing, duplicate rejection and encrypted retention")


def check_pdf_imports(driver: WebDriver, artifacts: Path, data_root: Path) -> None:
    source = data_root / "external-pdf"
    source.mkdir()
    path = source / "statement.pdf"
    path.write_bytes(build_pdf([["Synthetic checking statement", "2026-10-01 Fictional payroll 1000.01", "2026-10-02 Fictional groceries -25.11"]]))
    damaged = source / "damaged.pdf"
    damaged.write_bytes(b"%PDF-1.4\ndamaged")
    driver.open_client("Alex Fictional")
    driver.select("#statement-format", "Selectable-text PDF")
    driver.fill("#csv-path", str(damaged))
    driver.click_button("Inspect PDF")
    driver.wait_text(".imports-panel [role=alert]", "could not be extracted safely", "damaged PDF rejected without an import")
    driver.fill("#csv-path", str(path))
    driver.click_button("Inspect PDF")
    driver.wait_text(".pdf-line-selection", "Fictional payroll 1000.01", "real PDF parser process returns selectable source text")
    driver.wait_text(".pdf-line-selection", "Selected PDF lines: 0 / 500", "no automatic PDF line selection")
    for date in ["2026-10-01", "2026-10-02"]:
        driver.click(f"//label[contains(@class,'pdf-source-line')][contains(.,'{date}')]/input", "xpath")
    driver.fill("#csv-account-label", "Synthetic PDF checking")
    driver.fill("#csv-openingBalance", "0")
    driver.fill("#csv-closingBalance", "974.90")
    driver.click("//label[contains(.,'Authorize retaining the complete original PDF')]/input", "xpath")
    driver.click_button("Import into review queue")
    driver.wait_text(".imports-panel", "Every PDF record needs individual verification.", "selected PDF rows staged for clarification")
    driver.wait_text(".imports-progress", "Needs Clarification 2", "all PDF rows require individual review")
    driver.click_button("Review statement.pdf record 1")
    driver.wait_text(".transaction-review", "page 1, extracted line", "page/line provenance displayed")
    driver.select("#review-kind", "Income")
    driver.fill("#review-category", "Wages")
    driver.select("#review-status", "Approved")
    driver.fill("#review-reason", "Individually verified synthetic PDF payroll, USD currency, amount and date")
    driver.click("#review-ack")
    driver.wait(lambda: driver.command("GET", f"/element/{driver.find('#review-ack')}/property/checked") is True,
                "PDF individual acknowledgment selected")
    driver.click_button("Save transaction decision")
    driver.wait_text(".imports-totals", "$3,000.01", "only approved PDF income joins prior approved CSV income")
    driver.wait_text(".imports-totals", "$1,820.02", "pending PDF expense stays excluded from exact combined account activity")
    driver.wait_text(".imports-progress", "Needs Clarification 1", "unapproved PDF expense remains unresolved")
    driver.click_button("Review statement.pdf record 1")
    driver.click(".transaction-review > details > summary")
    driver.wait_text(".transaction-review", "2026-10-01 Fictional payroll 1000.01", "original PDF text remains immutable")
    driver.script("document.querySelector('.transaction-review').scrollIntoView({block:'center'})")
    driver.screenshot(artifacts / "native-pdf-review.png")
    status("bounded separate PDF parser, damaged-file rejection, explicit source-line selection, page provenance, individual approval, and pending-data exclusion")


def run_checks(driver: WebDriver, binary: Path, password: str,
               wrong_password: str, artifacts: Path, restart_driver: Callable[[], None], data_root: Path) -> None:
    organization = "Smoke Test Counseling — Fictional"
    counselor = "Morgan Example"
    first_name = "Alex"
    first_last = "Fictional"
    second_name = "Robin"
    second_last = "Synthetic"
    first_goal = "Build a fictional emergency savings plan"
    edited_goal = "Save for a fictional household reserve"
    note = "Synthetic private note for Alex only. No real client information."

    driver.open(binary)
    driver.element("#organization")
    if driver.observe_script("return Boolean(window.__TAURI_INTERNALS__)") is not True:
        raise SmokeFailure("This session is not running in the native Tauri desktop runtime")
    location = urllib.parse.urlsplit(str(driver.command("GET", "/url")))
    embedded = ((location.scheme in ("http", "https") and location.hostname == "tauri.localhost")
                or (location.scheme == "tauri" and location.hostname == "localhost"))
    if not embedded:
        raise SmokeFailure("The native application is not serving its compiled frontend assets")
    status("actual native Tauri runtime launched")

    driver.fill("#organization", organization)
    driver.fill("#counselor", counselor)
    driver.fill("#password", password)
    driver.fill("#confirm-password", password)
    driver.wait_value("#organization", organization, "exact organization input before setup")
    driver.wait_value("#counselor", counselor, "exact counselor input before setup")
    driver.wait_value("#password", password, "complete password input before setup")
    driver.wait_value("#confirm-password", password, "complete confirmation input before setup")
    driver.click_button("Create workspace")
    driver.wait_content(".workspace-label strong", organization, "organization profile")
    driver.wait_content(".counselor strong", counselor, "counselor profile")
    status("organization and counselor setup")

    def create_client(given: str, family: str, goal: str) -> None:
        driver.click(".page-heading button")
        driver.fill("#first-name", given)
        driver.fill("#last-name", family)
        driver.fill("#email", f"{given.lower()}@example.invalid")
        driver.fill("#phone", "555-0100")
        driver.fill("#household-size", "2")
        driver.fill("#goals", goal)
        driver.click_button("Create client")
        driver.wait_text(".editor-panel h2", f"{given} {family}", "saved client profile")

    create_client(first_name, first_last, first_goal)
    driver.fill("#new-note", note)
    driver.click_button("Save session note")
    driver.wait_text(".notes-history", note, "saved counseling note")
    driver.wait_text(".session-note .note-meta strong", counselor, "saved note author")
    driver.wait_text(".budget-panel", "No saved budget yet", "first client's empty budget")

    def budget_line(index: int, button: str, label: str, amount: str, frequency: str = "Monthly", kind: str | None = None) -> None:
        driver.click_button(button)
        driver.fill(f"#budget-label-{index}", label)
        driver.fill(f"#budget-amount-{index}", amount)
        driver.select(f"#budget-frequency-{index}", frequency)
        if kind:
            driver.select(f"#budget-kind-{index}", kind)

    budget_line(0, "Add income", "Alex synthetic wages", "1500", "Every two weeks")
    budget_line(1, "Add expense", "Synthetic rent", "1200")
    budget_line(2, "Add expense", "Synthetic groceries", "100", "Weekly")
    budget_line(3, "Add expense", "Synthetic recreation", "200", kind="Discretionary expense")
    budget_line(4, "Add allocation", "Synthetic savings", "300")
    budget_line(5, "Add allocation", "Synthetic debt payment", "400", kind="Debt payment allocation")
    driver.click_button("Save budget and calculate")
    driver.wait_text(".budget-panel", "Budget saved and calculated.", "saved budget")
    driver.wait_value("#budget-amount-0", "1500.00", "canonical stored income")
    for total in ["$3,250.00", "$1,633.33", "$1,833.33", "$1,416.67", "$1,116.67", "$716.67"]:
        driver.wait_text(".budget-totals", total, f"independently checked monthly plan total {total}")
    driver.script("document.querySelector('.budget-results').scrollIntoView({block: 'center'})")
    driver.screenshot(artifacts / "native-budget.png")
    status("exact client budget calculation, encrypted save, and allocations")

    driver.wait_text(".debt-panel", "No saved debt plan yet", "first client's empty debt plan")
    driver.fill("#debt-start-month", "2026-11")
    driver.fill("#debt-monthly-payment", "100")

    def debt_account(index: int, label: str, balance: str, apr: str, minimum: str) -> None:
        driver.click_button("Add debt")
        driver.fill(f"#debt-label-{index}", label)
        driver.fill(f"#debt-balance-{index}", balance)
        driver.fill(f"#debt-apr-{index}", apr)
        driver.fill(f"#debt-minimumPayment-{index}", minimum)
        driver.fill(f"#debt-monthlyFee-{index}", "0")
        driver.select(f"#debt-model-{index}", "Fixed monthly APR estimate")
        driver.click(f"#debt-verified-{index}")

    debt_account(0, "Alex synthetic small debt", "100", "0", "10")
    debt_account(1, "Alex synthetic high APR debt", "500", "24", "20")
    driver.click_button("Save debt plan and compare")
    driver.wait_text(".debt-panel", "Debt plan saved and compared.", "saved debt comparison")
    driver.wait_value("#debt-balance-0", "100.00", "canonical debt balance")
    for expected in ["$43.10", "$35.35", "$643.10", "$635.35", "2027-05"]:
        driver.wait_text(".debt-comparison", expected, f"independent debt comparison {expected}")
    driver.wait_text(".debt-month-table", "$80.00", "snowball first-month target payment")
    driver.select("#debt-strategy", "Avalanche")
    driver.wait_text(".debt-month-table", "$90.00", "avalanche first-month target payment")
    driver.wait_text(".debt-month-table", "$420.00", "avalanche first-month closing balance")
    driver.script("document.querySelector('.debt-results').scrollIntoView({block: 'start'})")
    driver.screenshot(artifacts / "native-debt-comparison.png")
    status("saved debt strategies and monthly account schedules match independent examples")

    driver.wait_text(".savings-panel", "No saved savings plan yet", "first client's empty savings plan")
    driver.click_button("Refresh budget and debt references")
    driver.wait_text(".savings-panel .debt-budget-reference", "$1,633.33", "latest necessary expense reference")
    driver.fill("#savings-start-month", "2026-11")
    driver.fill("#savings-target-months", "3")
    driver.fill("#savings-liquidSavings", "2600")
    driver.fill("#savings-restrictedSavings", "5000")
    driver.fill("#savings-obligationsReserved", "600")
    driver.fill("#savings-emergencyContribution", "200")
    driver.fill("#savings-priorities", "Alex synthetic reserve priority")
    driver.click_button("Add sinking fund")
    driver.fill("#savings-goal-label-0", "Alex synthetic insurance reserve")
    driver.fill("#savings-goal-date-0", "2027-02-28")
    driver.fill("#savings-goal-targetAmount-0", "1200")
    driver.fill("#savings-goal-reservedAmount-0", "1000")
    driver.fill("#savings-goal-monthlyContribution-0", "50")
    driver.click(".savings-panel details summary")
    for index, amount in enumerate(["100", "300", "1500"]):
        driver.click_button("Add alternative contribution")
        driver.fill(f"#savings-alternative-{index}", amount)
    driver.click_button("Save savings plan and calculate")
    driver.wait_text(".savings-panel", "Savings plan saved and calculated.", "saved savings plan")
    for value in ["$1,000.00", "0.61 months", "$4,899.99", "$3,899.99", "2028-06-30", "$766.67"]:
        driver.wait_text(".savings-results", value, f"independent savings result {value}")
    driver.wait_text("table[aria-label='Sinking fund forecasts']", "$50.00", "deadline-based contribution")
    driver.wait_text("table[aria-label='Emergency contribution scenarios']", "2030-01-31", "lower contribution scenario")
    driver.wait_text("table[aria-label='Emergency contribution scenarios']", "2027-11-30", "higher contribution scenario")
    driver.wait_text("table[aria-label='Emergency contribution scenarios']", "Exceeds entered funds", "overallocated scenario warning")
    driver.select("#savings-schedule-month", "2028-06")
    driver.wait_text(".savings-schedule", "$99.99", "capped final emergency contribution")
    driver.script("document.querySelector('.savings-results').scrollIntoView({block: 'start'})")
    driver.screenshot(artifacts / "native-savings.png")
    status("savings buckets, exact coverage/targets/deadlines, contribution scenarios, and allocation checks")

    driver.wait_text(".cashflow-panel", "No saved cash-flow plan yet", "first client's empty cash-flow plan")
    driver.fill("#cashflow-start", "2026-11-01")
    driver.fill("#cashflow-end", "2026-11-30")
    for index, name, amount in [(0, "Alex synthetic checking", "500"), (1, "Alex synthetic reserve account", "1000")]:
        driver.click_button("Add cash-flow account")
        driver.fill(f"#cashflow-account-label-{index}", name)
        driver.fill(f"#cashflow-account-balance-{index}", amount)
    for index, name, kind, amount, date, frequency in [
        (0, "Alex synthetic rent date", "Bill", "1200", "2026-11-03", "One time / irregular"),
        (1, "Alex synthetic dated pay", "Paycheck", "1500", "2026-11-06", "Every two weeks"),
        (2, "Alex synthetic account transfer", "Transfer between modeled accounts", "200", "2026-11-07", "One time / irregular"),
    ]:
        driver.click_button("Add dated activity")
        driver.fill(f"#cashflow-label-{index}", name)
        driver.select(f"#cashflow-kind-{index}", kind)
        driver.fill(f"#cashflow-amount-{index}", amount)
        driver.fill(f"#cashflow-date-{index}", date)
        driver.select(f"#cashflow-frequency-{index}", frequency)
        if index == 2:
            driver.select(f"#cashflow-destination-{index}", "Alex synthetic reserve account")
    driver.fill("#cashflow-notes", "Alex synthetic cash-flow priority")
    driver.click_button("Save cash-flow plan and calculate")
    driver.wait_text(".cashflow-panel", "Cash-flow plan saved and calculated.", "saved dated cash-flow plan")
    driver.click("button[aria-label='Review 2026-11-03']")
    driver.wait_text(".cashflow-day-details", "-$700.00", "account shortfall despite positive combined balance")
    driver.wait_text(".cashflow-day-details", "2026-11-06", "next entered payday")
    for value in ["$2,100.00", "$1,200.00", "$3,300.00"]:
        driver.wait_text(".cashflow-results .budget-totals", value, "independent closing balance")
    driver.script("document.querySelector('.cashflow-calendar').scrollIntoView({block: 'center'})")
    driver.screenshot(artifacts / "native-cashflow-calendar.png")
    driver.click_button("Chronological ledger")
    driver.wait_text(".cashflow-ledger", "-$700.00", "chronological running balance")
    driver.wait_text(".cashflow-ledger", "Alex synthetic account transfer", "paired transfer in ledger")
    driver.script("document.querySelector('.cashflow-ledger').scrollIntoView({block: 'center'})")
    driver.screenshot(artifacts / "native-cashflow-ledger.png")
    driver.fill("#cashflow-date-0", "2026-11-08")
    driver.click_button("Preview cash flow / refresh references")
    driver.wait_text(".cashflow-results", "Draft cash-flow preview", "unsaved adjusted payment preview")
    driver.click("button[aria-label='Review 2026-11-03']")
    if "shortfall" in driver.text(".cashflow-day-details"):
        raise SmokeFailure("Moving rent after payday did not remove the original shortfall in the preview")
    # Restore the draft to its saved date without writing. Restart must preserve Nov 3.
    driver.fill("#cashflow-date-0", "2026-11-03")
    driver.click_button("Preview cash flow / refresh references")
    driver.wait_text(".cashflow-results", "Saved cash-flow forecast", "unchanged saved-date preview")
    status("dated cash flow, per-account shortfall, paired transfer, calendar/ledger, and non-writing date preview")

    check_csv_imports(driver, artifacts, data_root)

    create_client(second_name, second_last, "Create a fictional spending plan")
    driver.wait_text(".notes-history", "No session notes yet.", "second client's empty note history")
    if note in driver.text(".notes-history"):
        raise SmokeFailure("The first client's note appeared in another client's record")
    driver.wait_text(".budget-panel", "No saved budget yet", "second client's empty budget")
    if driver.visible("#budget-label-0"):
        raise SmokeFailure("The first client's budget appeared in the second client's record")
    driver.wait_text(".debt-panel", "No saved debt plan yet", "second client's empty debt plan")
    if driver.visible("#debt-label-0"):
        raise SmokeFailure("The first client's debt plan appeared in the second client's record")
    driver.wait_text(".savings-panel", "No saved savings plan yet", "second client's empty savings plan")
    if driver.visible("#savings-goal-label-0"):
        raise SmokeFailure("The first client's sinking fund appeared in the second client's plan")
    driver.wait_text(".cashflow-panel", "No saved cash-flow plan yet", "second client's empty cash-flow plan")
    if driver.visible("#cashflow-account-label-0"):
        raise SmokeFailure("The first client's cash-flow account appeared for another client")
    driver.wait_text(".imports-panel", "No imported transactions yet", "isolated second client CSV queue")
    status("two clients with isolated notes, budgets, debts, savings, and cash flow")

    driver.open_client("Alex Fictional")
    driver.wait_text(".notes-history", note, "first client's note after selection")
    driver.select("#client-status", "Planning")
    driver.fill("#goals", edited_goal)
    driver.click_button("Save changes")
    driver.wait_value("#goals", edited_goal, "updated counseling goal")
    driver.wait_text(".editor-panel .status-tag", "Planning", "updated workflow status")
    driver.screenshot(artifacts / "native-desktop.png")
    status("client edits saved")

    driver.click_button("Lock workspace")
    driver.element("#password")
    private_values = [f"{first_name} {first_last}", f"{second_name} {second_last}",
                      first_goal, edited_goal, note, organization, counselor, "Alex synthetic wages", "Alex synthetic small debt", "Alex synthetic high APR debt", "Alex synthetic reserve priority", "Alex synthetic insurance reserve", "Alex synthetic checking", "Alex synthetic cash-flow priority", "Alex synthetic dated pay", "Synthetic reserve transfer", "Synthetic incoming transfer", "Synthetic checking", "checking.csv", "Fictional payroll", "Synthetic PDF checking"]

    def assert_locked() -> None:
        markup = str(driver.observe_script("return document.documentElement.outerHTML"))
        if any(value in markup for value in private_values):
            raise SmokeFailure("Private workspace data remained in the DOM after locking")
        if driver.observe_script("return document.querySelector('#password').value") != "":
            raise SmokeFailure("The locked screen retained a password value")

    assert_locked()
    driver.fill("#password", wrong_password)
    driver.click_button("Unlock workspace")
    driver.element("[role=alert]")
    if driver.visible(".editor-panel"):
        raise SmokeFailure("An incorrect password unlocked the workspace")
    assert_locked()
    driver.fill("#password", password)
    driver.click_button("Unlock workspace")
    driver.wait_content(".workspace-label strong", organization, "unlocked organization profile")
    driver.open_client("Alex Fictional")
    driver.wait_text(".notes-history", note, "note after unlocking")
    driver.wait_value("#budget-label-0", "Alex synthetic wages", "budget after unlocking")
    driver.wait_text(".budget-totals", "$716.67", "correct budget totals after unlocking")
    driver.wait_value("#debt-label-0", "Alex synthetic small debt", "debt plan after unlocking")
    driver.wait_text(".debt-comparison", "$35.35", "debt comparison after unlocking")
    driver.wait_value("#savings-priorities", "Alex synthetic reserve priority", "savings after unlocking")
    driver.wait_text(".savings-results", "2028-06-30", "savings forecast after unlocking")
    driver.wait_value("#cashflow-notes", "Alex synthetic cash-flow priority", "cash-flow plan after unlocking")
    driver.wait_value("#cashflow-date-0", "2026-11-03", "preview did not change saved payment date")
    status("manual lock clears private data; incorrect password rejected; correct unlock")

    # Verify the application is responsive before a deliberate process stop.
    # Restart the native driver too: WebKitGTK session deletion/reuse can race
    # its browser shutdown, independent of application storage correctness.
    if driver.observe_script("return Boolean(window.__TAURI_INTERNALS__)") is not True:
        raise SmokeFailure("The application stopped responding before persistence validation")
    restart_driver()
    driver.open(binary)
    driver.element("#password")
    assert_locked()
    driver.fill("#password", password)
    driver.click_button("Unlock workspace")
    driver.wait_content(".workspace-label strong", organization, "persisted organization profile")
    driver.wait_content(".counselor strong", counselor, "persisted counselor profile")
    driver.open_client("Alex Fictional")
    driver.wait_text(".notes-history", note, "persisted counseling note")
    driver.wait_value("#email", "alex@example.invalid", "persisted client contact information")
    driver.wait_value("#household-size", "2", "persisted household size")
    driver.wait_value("#goals", edited_goal, "persisted edited goal")
    driver.wait_text(".editor-panel .status-tag", "Planning", "persisted edited status")
    driver.wait_value("#budget-label-0", "Alex synthetic wages", "persisted budget source")
    driver.wait_value("#budget-amount-0", "1500.00", "persisted original biweekly amount")
    driver.wait_text(".budget-totals", "$716.67", "persisted calculated budget")
    driver.wait_value("#debt-label-0", "Alex synthetic small debt", "persisted debt account")
    driver.wait_text(".debt-comparison", "$35.35", "persisted exact debt comparison")
    driver.wait_value("#savings-goal-label-0", "Alex synthetic insurance reserve", "persisted sinking fund")
    driver.wait_text(".savings-results", "2028-06-30", "persisted emergency forecast")
    driver.wait_value("#cashflow-date-0", "2026-11-03", "persisted original payment date")
    driver.wait_text(".cashflow-results .budget-totals", "$3,300.00", "persisted cash-flow forecast")
    driver.wait_text(".imports-progress", "Approved 5 · Rejected 1", "persisted CSV decisions and paired transfers")
    driver.wait_text(".imports-totals", "$820.01", "persisted exact approved activity")
    driver.open_client("Robin Synthetic")
    driver.wait_text(".editor-panel", "Robin Synthetic", "persisted second client")
    driver.wait_text(".notes-history", "No session notes yet.", "persisted second client's empty note history")
    if note in driver.text(".notes-history"):
        raise SmokeFailure("Persisted notes crossed client boundaries after relaunch")
    driver.wait_text(".budget-panel", "No saved budget yet", "persisted second client's empty budget")
    if driver.visible("#budget-label-0"):
        raise SmokeFailure("Persisted budgets crossed client boundaries after relaunch")
    driver.wait_text(".debt-panel", "No saved debt plan yet", "persisted second client's empty debt plan")
    if driver.visible("#debt-label-0"):
        raise SmokeFailure("Persisted debts crossed client boundaries after relaunch")
    driver.wait_text(".savings-panel", "No saved savings plan yet", "persisted second client's empty savings plan")
    if driver.visible("#savings-goal-label-0"):
        raise SmokeFailure("Persisted savings crossed client boundaries after relaunch")
    driver.wait_text(".cashflow-panel", "No saved cash-flow plan yet", "persisted second client's empty cash flow")
    if driver.visible("#cashflow-account-label-0"):
        raise SmokeFailure("Persisted cash-flow plans crossed client boundaries")
    driver.wait_text(".imports-panel", "No imported transactions yet", "persisted CSV isolation")
    status("full app relaunch preserves profiles, clients, edits, and isolated notes/budgets/debts/savings/cash-flow plans")


    check_pdf_imports(driver, artifacts, data_root)

    # Exercise recovery only in this synthetic app-data directory. No native
    # commands are invoked directly; passwords and paths use visible controls.
    backup_folder=data_root / "external-backups"
    backup_folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button:nth-child(3)")
    driver.fill("#backup-folder", str(backup_folder))
    driver.fill("#backup-password", password)
    driver.click_button("Create encrypted backup")
    driver.wait_text(".backup-success", "Encrypted backup saved.", "encrypted backup created")
    driver.wait_value("#backup-password", "", "backup password cleared from interface")
    backup_path=Path(driver.text(".backup-file-path"))
    backup_bytes=backup_path.read_bytes()
    if not backup_bytes.startswith(b"EBCBACK1") or note.encode() in backup_bytes or password.encode() in backup_bytes:
        raise SmokeFailure("Backup format or encrypted-data boundary was not preserved")
    driver.screenshot(artifacts / "native-encrypted-backup.png")
    driver.open_client("Alex Fictional")
    after_note="Synthetic note added after backup, retained in the previous workspace only."
    private_values.append(after_note)
    driver.fill("#new-note", after_note)
    driver.click_button("Save session note")
    driver.wait_text(".notes-history", after_note, "post-backup historical note")
    driver.click_button("Lock workspace")
    driver.element("#password")
    originals=list((data_root / "data").rglob("workspace.db"))
    if len(originals)!=1:
        raise SmokeFailure("Unexpected original synthetic database count")
    original_path=originals[0]
    original_bytes=original_path.read_bytes()
    driver.click(".restore-panel > summary")
    driver.fill("#restore-file", str(backup_path))
    driver.fill("#restore-password", wrong_password)
    driver.click_button("Verify backup for recovery")
    driver.wait_text(".restore-panel [role=alert]", "could not be verified", "incorrect backup password rejected")
    driver.fill("#restore-password", password)
    driver.click_button("Verify backup for recovery")
    driver.wait_text(".restore-review", organization, "authenticated recovery review")
    driver.wait_text(".restore-review", counselor, "verified counselor identity")
    driver.screenshot(artifacts / "native-restore-review.png")
    # Stop the complete application and driver while a verified candidate awaits
    # confirmation. A fresh process must still open the original workspace.
    restart_driver()
    driver.open(binary)
    driver.element("#password")
    assert_locked()
    driver.fill("#password", password)
    driver.click_button("Unlock workspace")
    driver.open_client("Alex Fictional")
    driver.wait_text(".notes-history", after_note, "interrupted restore preserves the original workspace")
    driver.click_button("Lock workspace")
    driver.element("#password")
    driver.click(".restore-panel > summary")
    driver.fill("#restore-file", str(backup_path))
    driver.fill("#restore-password", password)
    driver.click_button("Verify backup for recovery")
    driver.wait_text(".restore-review", organization, "new verified recovery review")
    driver.fill("#restore-confirmation", "RESTORE")
    driver.click_button("Restore verified workspace")
    driver.wait_text(".auth-card", "Workspace restored. Unlock it", "explicit confirmed restore completed")
    assert_locked()
    if original_path.read_bytes()!=original_bytes:
        raise SmokeFailure("Restoring changed the retained original database")
    driver.fill("#password", password)
    driver.click_button("Unlock workspace")
    driver.open_client("Alex Fictional")
    driver.wait_text(".notes-history", note, "original backed-up note restored")
    if after_note in driver.text(".notes-history"):
        raise SmokeFailure("Restore unexpectedly merged newer records into the backup")
    driver.wait_text(".budget-totals", "$716.67", "restored budget")
    driver.wait_text(".savings-results", "2028-06-30", "restored savings plan")
    driver.wait_text(".cashflow-results .budget-totals", "$3,300.00", "restored cash-flow plan")
    driver.wait_text(".imports-progress", "Approved 6 · Rejected 1", "backup restores CSV/PDF decisions and reciprocal links")
    driver.wait_text(".imports-totals", "$1,820.02", "backup restores exact imported totals")
    driver.click_button("Review checking.csv record 4")
    driver.click(".review-history > summary")
    driver.wait_text(".review-history", "Approved → Rejected", "backup restores immutable review history")
    driver.click_button("Review statement.pdf record 1")
    driver.wait_text(".transaction-review", "page 1, extracted line", "restored PDF provenance")
    driver.click(".review-history > summary")
    driver.wait_text(".review-history", "Needs Clarification → Approved", "restored immutable PDF approval history")
    status("encrypted backup, wrong-password rejection, interrupted review, explicit restore, retained source, and restored plans")
    # Simulate missing metadata only after stopping every synthetic app process.
    # The error-screen recovery flow must work without opening the damaged vault.
    pointer=original_path.parent / "workspace-active.json"
    active=json.loads(pointer.read_text())["generation"]
    damaged_directory=original_path.parent / "recovery-generations" / active
    restart_driver()
    (damaged_directory / "workspace.json").unlink()
    damaged_bytes=(damaged_directory / "workspace.db").read_bytes()
    driver.open(binary)
    driver.wait_text(".standalone-panel", "Unable to open the workspace", "simulated missing metadata error")
    driver.click(".restore-panel > summary")
    driver.fill("#restore-file", str(backup_path))
    driver.fill("#restore-password", password)
    driver.click_button("Verify backup for recovery")
    driver.wait_text(".restore-review", organization, "backup reviewed from storage error screen")
    driver.fill("#restore-confirmation", "RESTORE")
    driver.click_button("Restore verified workspace")
    driver.wait_text(".auth-card", "Workspace restored. Unlock it", "damaged workspace recovered")
    if (damaged_directory / "workspace.db").read_bytes()!=damaged_bytes:
        raise SmokeFailure("Recovery changed the retained damaged database")
    driver.fill("#password", password)
    driver.click_button("Unlock workspace")
    driver.open_client("Alex Fictional")
    driver.wait_text(".notes-history", note, "restored records after simulated metadata loss")
    driver.wait_value("#cashflow-date-0", "2026-11-03", "restored original cash-flow dates after recovery")
    driver.wait_text(".imports-progress", "Approved 6 · Rejected 1", "storage-error recovery includes imported records")
    status("fresh-process activation and error-screen recovery after simulated metadata loss")


def main() -> int:
    repository = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default=str(repository / "src-tauri/target/debug/eisenbart-counseling-suite"))
    parser.add_argument("--driver", default="tauri-driver")
    parser.add_argument("--native-driver", help="Path to WebKitWebDriver for the installed WebKitGTK runtime")
    parser.add_argument("--port", type=int, default=4444)
    parser.add_argument("--native-port", type=int, default=4445)
    parser.add_argument("--artifacts", type=Path, help="Store screenshots and driver log outside the checkout")
    options = parser.parse_args()
    if options.port == options.native_port or not all(0 < p < 65536 for p in (options.port, options.native_port)):
        parser.error("driver ports must be distinct and within 1–65535")
    artifacts = (options.artifacts or Path(tempfile.mkdtemp(prefix="eisenbart-desktop-smoke-artifacts-"))).resolve()
    if artifacts == repository or repository in artifacts.parents:
        parser.error("--artifacts must be outside the repository")
    artifacts.mkdir(parents=True, exist_ok=True)
    password = "Smoke-" + secrets.token_urlsafe(24)
    wrong_password = "Wrong-" + secrets.token_urlsafe(24)
    driver = WebDriver(options.port, [password, wrong_password])
    process: subprocess.Popen[bytes] | None = None
    try:
        if not sys.platform.startswith("linux"):
            raise SmokeFailure("This smoke test requires Linux XDG storage isolation; use the documented Windows manual checks on Windows")
        binary = executable(options.binary, "Built desktop application")
        intermediary = executable(options.driver, "tauri-driver")
        native = executable(options.native_driver or "WebKitWebDriver", "Native WebDriver")
        ensure_available(options.port)
        ensure_available(options.native_port)
        if sys.platform.startswith("linux") and not os.environ.get("DISPLAY"):
            raise SmokeFailure("Linux native UI testing requires DISPLAY; start Xvfb or use a desktop display")
        with tempfile.TemporaryDirectory(prefix="eisenbart-desktop-smoke-data-") as data_directory:
            root = Path(data_directory)
            environment = os.environ.copy()
            for variable, directory in (("XDG_DATA_HOME", "data"), ("XDG_CONFIG_HOME", "config"),
                                        ("XDG_CACHE_HOME", "cache")):
                destination = root / directory
                destination.mkdir()
                environment[variable] = str(destination)
            with (artifacts / "driver.log").open("wb") as log:
                def launch_driver() -> None:
                    nonlocal process
                    process = subprocess.Popen([
                        str(intermediary), "--port", str(options.port),
                        "--native-port", str(options.native_port), "--native-driver", str(native),
                    ], env=environment, stdout=log, stderr=log, start_new_session=True)

                def ready() -> bool:
                    if process is None or process.poll() is not None:
                        raise SmokeFailure(f"tauri-driver exited; see {artifacts / 'driver.log'}")
                    try:
                        value = driver.request("GET", "/status", timeout=2)
                        return isinstance(value, dict) and bool(value.get("ready"))
                    except DriverFailure as error:
                        if error.code == "connection error":
                            return False
                        raise

                def restart_driver() -> None:
                    if process is None or process.poll() is not None:
                        raise SmokeFailure("The native driver exited unexpectedly before the restart checkpoint")
                    stop_driver(process)
                    driver.session_id = None
                    ensure_available(options.port)
                    ensure_available(options.native_port)
                    launch_driver()
                    driver.wait(ready, "fresh native WebDriver readiness", seconds=25)

                launch_driver()

                try:
                    driver.wait(ready, "native WebDriver readiness", seconds=25)
                    run_checks(driver, binary, password, wrong_password, artifacts, restart_driver, root)
                except (SmokeFailure, OSError, ValueError):
                    try:
                        if driver.visible(".imports-panel [role=alert]"):
                            print("CSV diagnostic: " + driver.safe(driver.text(".imports-panel [role=alert]")), flush=True)
                    except SmokeFailure:
                        pass
                    if driver.session_id:
                        try:
                            driver.screenshot(artifacts / "failure.png")
                        except (SmokeFailure, OSError, ValueError):
                            pass
                    raise
                finally:
                    try:
                        driver.close()
                    except SmokeFailure:
                        pass
                    stop_driver(process)
        print(f"PASS native desktop smoke complete. Artifacts: {artifacts}", flush=True)
        return 0
    except (SmokeFailure, OSError, ValueError) as error:
        print(f"FAIL {driver.safe(str(error))}", file=sys.stderr, flush=True)
        print(f"Diagnostics: {artifacts}", file=sys.stderr, flush=True)
        return 1
    finally:
        try:
            driver.close()
        except SmokeFailure:
            pass
        if process is not None:
            stop_driver(process)


if __name__ == "__main__":
    sys.exit(main())
