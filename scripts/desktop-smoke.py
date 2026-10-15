#!/usr/bin/env python3
"""Exercise the actual Tauri desktop application with W3C WebDriver.

Uses only Python's standard library. Never connects to a development web server
or invokes application commands directly. All changes use the visible UI.
"""

from __future__ import annotations

import argparse
import base64
import datetime as dt
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
from zoneinfo import ZoneInfo
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
    def __init__(self, port: int, protected_values: list[str], transport: str = "direct"):
        self.base = f"http://127.0.0.1:{port}"
        # Local WebDriver traffic must stay on loopback, independent of outbound
        # HTTP proxy configuration. No external connections use this opener.
        self.http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        self.transport = transport
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
                **({"webkitgtk:browserOptions": {"binary": str(binary), "args": []}}
               if self.transport == "direct" else {"tauri:options": {"application": str(binary)}}),
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

    def observe_script(self, script: str, args: list[object] | None = None) -> object:
        # Call only for the explicit DOM/runtime observations below. A closed
        # native-driver connection may be retried once; mutations use script().
        for attempt in range(2):
            try:
                return self.script(script, args)
            except DriverFailure as error:
                if error.code != "connection error" or attempt == 1:
                    raise
                time.sleep(0.25)
        raise SmokeFailure("Read-only script observation did not complete")

    def scroll_element(self, element: str, block: str = "center") -> None:
        # Narrow viewport movement only: no arbitrary script, focus, typing,
        # clicks, application commands or financial writes can enter this retry.
        if block not in ("center", "start"):
            raise ValueError("Unsupported viewport alignment")
        for attempt in range(2):
            try:
                self.script("arguments[0].scrollIntoView({block:arguments[1],inline:'nearest',behavior:'instant'});",
                            [{ELEMENT_KEY: element}, block])
                return
            except DriverFailure as error:
                if error.code != "connection error" or attempt == 1:
                    raise
                time.sleep(0.25)

    def scroll_to(self, selector: str, block: str = "center") -> None:
        self.scroll_element(self.element(selector), block)

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
                # WebKit's enabled endpoint can miss inherited fieldset state.
                # Read the browser's effective HTML disabled state before any
                # focus, typing or click. Waiting never replays a mutation.
                effective = self.observe_script(
                    "return !arguments[0].matches(':disabled') && !arguments[0].closest('[inert]');",
                    [{ELEMENT_KEY: element}])
                if effective is True:
                    return element
            return None
        return self.wait(enabled, f"enabled {selector}")

    def click(self, selector: str, using: str = "css selector") -> None:
        element = self.interactive(selector, using)
        # WebKit's automatic scrolling can leave a partially visible control
        # at the viewport edge and click its old position. Center first; this
        # changes only the viewport and never invokes application commands.
        self.scroll_element(element)
        self.command("GET", f"/element/{element}/displayed")
        try:
            self.command("POST", f"/element/{element}/click", {})
        except DriverFailure as error:
            # Add intent to diagnostics; never replay an uncertain click.
            raise SmokeFailure(f"Native click failed for {selector}: {error}") from None

    def click_button(self, text: str) -> None:
        self.click(f"//button[normalize-space(.)={json.dumps(text)}]", "xpath")

    def open_client(self, name: str) -> None:
        self.click("nav[aria-label='Main navigation'] button:nth-child(2)")
        self.click("//button[contains(concat(' ', normalize-space(@class), ' '), ' client-row ')]"
                   f"[.//strong[normalize-space(.)={json.dumps(name)}]]", "xpath")

    def fill(self, selector: str, value: str) -> None:
        element = self.interactive(selector)
        # Native clear/focus can fail at a viewport edge after review panels
        # refresh. Move only the viewport, then recheck enabled state. Neither
        # clear nor typing is retried, and no application mutation is scripted.
        self.scroll_element(element)
        element = self.interactive(selector)
        try:
            if value == "":
                # WebKit rejects an empty send-keys text parameter. Use a real
                # select-all/delete gesture so controlled fields also receive
                # an input event; never replace their value through JavaScript.
                self.command("POST", f"/element/{element}/value", {"text": "\ue009a\ue000\ue003"})
                self.wait_value(selector, "", "keyboard-cleared native field")
            else:
                self.command("POST", f"/element/{element}/clear", {})
                self.command("POST", f"/element/{element}/value", {"text": value})
        except DriverFailure as error:
            raise SmokeFailure(f"Native field {selector}: {error}") from error

    def fill_pdf_destination(self, selector: str, destination: Path) -> None:
        # Use real keyboard events so React observes replacement of a retained
        # controlled text field, rather than relying on the clear endpoint.
        # Send one select-all/delete gesture, observe the empty value, then type
        # once and verify the complete destination before exporting. Never retry
        # typing or an export, and never set a DOM value through JavaScript.
        element = self.interactive(selector)
        self.scroll_element(element)
        element = self.interactive(selector)
        self.command("POST", f"/element/{element}/value", {"text": "\ue009a\ue000\ue003"})
        self.wait_value(selector, "", "keyboard-cleared PDF destination")
        self.command("POST", f"/element/{element}/value", {"text": str(destination)})
        self.wait_value(selector, str(destination), "complete PDF destination before export")

    def fill_native_date(self, selector: str, iso: str) -> None:
        # This Linux WebKit date widget types month/day/year, while its DOM
        # value is ISO. Sending an ISO string types the wrong year. Use the
        # observed widget format, then read back the complete ISO value before
        # any save. Neither typing nor a save is retried.
        year, month, day = iso.split("-")
        self.fill(selector, f"{month}/{day}/{year}")
        self.wait_value(selector, iso, "complete native date before saving")

    def select(self, selector: str, value: str, using: str = "css selector") -> None:
        # WebDriver clicks the HTML option rather than calling application code.
        select = self.interactive(selector, using)
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
    driver.scroll_to("#review-ack", "center")
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
    driver.scroll_to(".transaction-review", "center")
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
    driver.scroll_to(".transaction-review", "center")
    driver.screenshot(artifacts / "native-pdf-review.png")
    status("bounded separate PDF parser, damaged-file rejection, explicit source-line selection, page provenance, individual approval, and pending-data exclusion")



def check_recurring_imports(driver: WebDriver, artifacts: Path, data_root: Path) -> None:
    source=data_root / "external-service.csv"
    source.write_text("Date,Description,Amount,Category,Type\n2026-01-31,Fictional service,-10.01,Services,expense\n2026-02-28,Fictional service,-12.03,Services,expense\n2026-03-31,Fictional service,-10.01,Services,expense\n",encoding="utf-8")
    driver.fill("#csv-path",str(source))
    driver.click_button("Inspect CSV")
    driver.wait_text(".csv-preview",source.name,"fictional recurring source")
    driver.fill("#csv-account-label","Fictional service checking")
    for key,heading in [("date","1: Date"),("description","2: Description"),("amount","3: Amount"),("category","4: Category"),("kind","5: Type")]:
        driver.select(f"#csv-map-{key}",heading)
    driver.click_button("Import into review queue")
    driver.wait_text(".imports-progress","Pending 3","recurring source is never auto-approved")
    driver.wait_text(".recurring-review","No approved payment groups yet.","pending rows excluded from groups")
    for index in range(1,4):
        driver.click(f".imports-table tbody tr:nth-child({index}) input[type=checkbox]")
    driver.click_button("Approve selected validated rows (3)")
    driver.wait_text(".recurring-review","total $32.05","independent exact payment total")
    driver.wait_text(".recurring-review","Suggested: Monthly","calendar month-end advisory")
    driver.click_button("Review payments for Fictional service")
    driver.wait_text(".recurring-decision","2026-01-31 through 2026-03-31","visible payment evidence dates")
    driver.fill("#recurring-label","Fictional household service")
    driver.select("#recurring-frequency","Monthly")
    driver.click("#recurring-subscription")
    driver.fill("#recurring-reason","Verified three synthetic statements and fictional subscription contract")
    driver.click_button("Save payment pattern decision")
    driver.wait_text(".recurring-review","Counselor-classified subscription","explicit subscription classification")
    driver.wait_text(".imports-totals","Spending before refunds","financial totals still present")
    driver.wait_text(".imports-totals","$32.05","confirmation does not change transaction totals")
    driver.click_button("Review external-service.csv record 2")
    driver.select("#review-status","Rejected")
    driver.fill("#review-reason","Withdraw source approval for recurring evidence test")
    driver.click_button("Save transaction decision")
    driver.wait_text(".recurring-review","Needs review","source reversal invalidates confirmation")
    driver.wait_text(".imports-totals","$22.04","withdrawn payment excluded from official total")
    driver.click_button("Review external-service.csv record 2")
    driver.select("#review-status","Approved")
    driver.fill("#review-reason","Verified source again after correction")
    driver.click_button("Save transaction decision")
    driver.wait_text(".recurring-review","Needs review","reapproval does not silently revive prior confirmation")
    driver.click_button("Review payments for Fictional service")
    driver.wait_text(".recurring-decision","This decision needs renewed verification.","changed evidence explanation")
    driver.fill("#recurring-reason","Reconfirmed all three source decisions after reversal")
    driver.click_button("Save payment pattern decision")
    driver.wait_text(".recurring-review","Counselor-classified subscription","explicit reconfirmation")
    # A newly approved charge then withdrawn must not revive an older decision,
    # even when all remaining facts match its original evidence exactly.
    later=data_root / "external-later.csv"
    later.write_text("Date,Description,Amount,Category,Type\n2026-04-30,Fictional service,-10.01,Services,expense\n",encoding="utf-8")
    driver.fill("#csv-path",str(later))
    driver.click_button("Inspect CSV")
    driver.wait_text(".csv-preview",later.name,"later synthetic service charge")
    driver.select("#csv-account","Fictional service checking")
    for key,heading in [("date","1: Date"),("description","2: Description"),("amount","3: Amount"),("category","4: Category"),("kind","5: Type")]:
        driver.select(f"#csv-map-{key}",heading)
    driver.click_button("Import into review queue")
    driver.wait_text(".imports-progress","Pending 1","later payment awaits review")
    driver.click(".imports-table tbody tr:nth-child(4) input[type=checkbox]")
    driver.click_button("Approve selected validated rows (1)")
    driver.wait_text(".recurring-review","total $42.06","later approved payment joins observed total")
    driver.wait_text(".recurring-review","Needs review","additional approved evidence invalidates confirmation")
    driver.click_button("Review external-later.csv record 2")
    driver.select("#review-status","Rejected")
    driver.fill("#review-reason","Withdraw newly approved charge after checking the fictional account")
    driver.click_button("Save transaction decision")
    driver.wait_text(".recurring-review","total $32.05","remaining evidence matches original facts")
    driver.wait_text(".recurring-review","Needs review","return to old facts does not revive confirmation")
    save_counselor_evidence(driver, artifacts)
    driver.click_button("Review payments for Fictional service")
    driver.fill("#recurring-reason","Reconfirmed remaining sources after a withdrawn new charge")
    driver.click_button("Save payment pattern decision")
    driver.wait_text(".recurring-review","Counselor-classified subscription","explicit third confirmation")
    driver.click(".recurring-history > summary")
    driver.wait_text(".recurring-history","Payment pattern decision history (3)","append-only recurring decision history")
    driver.wait_text(".recurring-history","Reconfirmed all three source decisions after reversal","captured reason history")
    driver.scroll_to(".recurring-review", "start")
    driver.screenshot(artifacts / "native-recurring-review.png")
    status("merchant evidence, month-end suggestions, explicit subscription decisions, exact totals, reversal invalidation, and append-only history")


def assert_recurring_restored(driver: WebDriver) -> None:
    driver.open_client("Casey Pattern")
    driver.wait_text(".recurring-review","Counselor-classified subscription","persisted recurring confirmation")
    driver.click(".recurring-history > summary")
    driver.wait_text(".recurring-history","Payment pattern decision history (3)","persisted recurring history")
    driver.wait_text(".recurring-history","Reconfirmed remaining sources after a withdrawn new charge","persisted recurring evidence-bound reason")
    driver.wait_text(".recurring-history","This earlier decision requires renewed verification.","persisted permanent evidence invalidation")
    driver.wait_text(".imports-totals","$32.05","persisted recurring client exact totals")



def choose_balance_statement(driver: WebDriver) -> None:
    driver.select("#balance-document","running.csv — Fictional balance checking")


def check_statement_balances(driver: WebDriver, artifacts: Path, data_root: Path) -> None:
    source=data_root / "running.csv"
    source.write_text("Date,Description,Amount,Category,Type,Balance\n2026-09-01,Fictional wages,500.01,Wages,income,1500.01\n2026-09-02,Fictional rent,-300.02,Housing,expense,1199.99\n2026-09-03,Fictional fee,-0.01,Fees,fee,1199.98\n",encoding="utf-8")
    driver.fill("#csv-path",str(source))
    driver.click_button("Inspect CSV")
    driver.wait_text(".csv-preview",source.name,"synthetic running-balance source")
    driver.fill("#csv-account-label","Fictional balance checking")
    for key,heading in [("date","1: Date"),("description","2: Description"),("amount","3: Amount"),("category","4: Category"),("kind","5: Type"),("balance","6: Balance")]:
        driver.select(f"#csv-map-{key}",heading)
    driver.select("#csv-balance-order","Oldest first — after each transaction")
    for key,value in [("openingBalance","1000.00"),("closingBalance","1199.98"),("periodStart","2026-09-01"),("periodEnd","2026-09-30")]:
        driver.fill(f"#csv-{key}",value)
    driver.click_button("Import into review queue")
    driver.wait_text(".imports-progress","Pending 3","balanced source stays unapproved")
    choose_balance_statement(driver)
    driver.wait_text(".balance-progress","Source: 3 matched · 0 differences. Approved: 0 matched","source references do not approve financial movements")
    driver.select("#balance-order","Newest first — after each transaction")
    driver.fill("#balance-reason","Check conflicting declared order against source dates")
    driver.click_button("Save statement balance setting")
    driver.wait_text(".imports-panel","Statement balance setting saved; checks updated.","wrong order recorded explicitly")
    choose_balance_statement(driver)
    driver.wait_text(".statement-balance-details","Original date sequence: Conflict","wrong source order is never silently sorted")
    driver.select("#balance-order","Oldest first — after each transaction")
    driver.fill("#balance-reason","Verified original source is oldest first with after-transaction balances")
    driver.click_button("Save statement balance setting")
    driver.wait_text(".imports-panel","Statement balance setting saved; checks updated.","correct order saved without transaction approval")
    for index in range(1,4):
        driver.click(f".imports-table tbody tr:nth-child({index}) input[type=checkbox]")
    driver.click_button("Approve selected validated rows (3)")
    driver.wait_text(".imports-totals","$199.98","independent approved source net")
    initial_balance_facts(driver)
    view_financial_dashboard(driver,"2026-09","2026-09")
    driver.wait_text(".assessment-totals","Operating cash flow$199.98", "initial exact approved assessment")
    driver.wait_text(".classified-totals","Recorded necessary spending before refunds$300.02","verified necessary expense")
    driver.wait_text(".classified-totals","Recorded discretionary spending before refunds$0.01","verified discretionary fee")
    driver.wait_text(".verified-balance-totals","Estimated net worth at balance date$525.05","independent dated asset/debt arithmetic")
    driver.wait_text(".verified-ratios","2.00 months","conservative accessible savings coverage")
    driver.wait_text(".verified-ratios","5.00%","explicit take-home payment ratio")
    driver.wait_text("table[aria-label='Verified debt balance chart']","Fictional loan","local verified debt chart")
    save_counselor_assessment(driver,"Taylor original assessment")
    driver.wait_text(".saved-assessment-view .assessment-totals","Operating cash flow$199.98","independent captured assessment amount")
    driver.click(".saved-assessments > summary")
    choose_balance_statement(driver)
    driver.wait_text(".balance-progress","Source: 3 matched · 0 differences. Approved: 3 matched · 0 differences.","exact original and approved balance steps")
    driver.click_button("Review balance source record 3")
    driver.fill("#review-amount","-300.01")
    driver.fill("#review-reason","Verified corrected fictional transaction; retain original figures")
    driver.click("#review-ack")
    driver.click_button("Save transaction decision")
    driver.wait_text(".imports-totals","$199.99","one-cent corrected approved net")
    driver.wait_text(".assessment-totals","Operating cash flow$199.99", "dashboard automatically recalculates corrected approval")
    driver.wait_text(".classified-totals","Unclassified approved spending$300.01","corrected source invalidates its classification")
    driver.wait_text(".financial-facts-editor","Needs re-verification: prior Necessary expense","stale source decision disclosed")
    driver.click_button("Reverify prior classification for Fictional rent (3)")
    save_financial_facts(driver,"Reverified necessary rent after its one-cent source correction")
    driver.wait(lambda: driver.command("GET", f"/element/{driver.find("//label[contains(.,'Classification for Fictional rent')]/select",'xpath')}/property/value") == "necessary", "explicit re-verification select value")
    driver.wait_text(".classified-totals","Recorded necessary spending before refunds$300.01","explicit re-verification rebinds classification")
    choose_balance_statement(driver)
    driver.wait_text(".balance-progress","Source: 3 matched · 0 differences. Approved: 1 matched · 2 differences.","source arithmetic remains intact while corrected approved steps diverge")
    driver.wait_text(".balance-check-table","Difference $0.01","independent corrected balance difference")
    driver.click_button("Review balance source record 3")
    driver.select("#review-status","Rejected")
    driver.fill("#review-reason","Withdraw corrected payment from approved movement")
    driver.click_button("Save transaction decision")
    driver.wait_text(".imports-totals","$500.00","rejected expense excluded from approved totals")
    driver.wait_text(".assessment-totals","Operating cash flow$500.00", "dashboard automatically excludes reversed expense")
    driver.wait_text(".classified-totals","Recorded necessary spending before refunds$0.00","rejected necessity excluded")
    driver.wait_text(".verified-fact-metrics","Classifications needing re-verification across this client: 1","rejection leaves old classification inactive")
    driver.wait_text(".verified-ratios","Accessible savings coverage — estimateUnavailable","zero necessary denominator never invents coverage")
    save_counselor_assessment(driver,"Taylor post-withdrawal assessment")
    driver.wait_text(".saved-assessment-view .assessment-totals","Operating cash flow$500.00","new assessment excludes the withdrawn approval")
    open_original_counselor_assessment(driver)
    driver.scroll_to(".saved-assessment-view", "start")
    driver.screenshot(artifacts / "native-saved-counselor-assessment.png")
    export_original_assessment_pdf(driver, artifacts, "before-restart")
    driver.click(".saved-assessments > summary")
    choose_balance_statement(driver)
    driver.wait_text(".balance-progress","Source: 3 matched · 0 differences. Approved: 1 matched · 1 differences.","rejection preserves source checks and updates approved comparisons")
    driver.wait_text(".balance-check-table","Excluded","rejected balance is not an approved anchor")
    driver.wait_text(".balance-check-table","Difference $300.02","independent rejected-row discrepancy")
    driver.click(".statement-balance-history > summary")
    driver.wait_text(".statement-balance-history","Statement balance setting history (2)","immutable source-order decisions")
    driver.scroll_to(".balance-check-table", "center")
    driver.screenshot(artifacts / "native-statement-balances.png")
    status("explicit source order, exact row balances, wrong-order conflicts, separate approved data, one-cent corrections, rejected exclusion, and setting history")


def save_counselor_assessment(driver: WebDriver, title: str) -> None:
    if driver.command("GET", f"/element/{driver.find('.saved-assessments')}/property/open") is not True:
        driver.click(".saved-assessments > summary")
    driver.wait(lambda: driver.visible("#saved-assessment") is not None, "scoped assessment directory")
    driver.fill("#record-from","2026-09")
    driver.fill("#record-through","2026-09")
    driver.fill("#record-title",title)
    driver.fill("#record-findings","Fictional findings based on approved source decisions; missing coverage remains disclosed.")
    driver.fill("#record-recommendations","Verify remaining sources before changing the fictional household plan.")
    driver.fill("#record-followUp","Review the fictional records at the next counseling appointment.")
    driver.click("#record-ack")
    driver.click_button("Save assessment record")
    driver.wait_text(".saved-assessments","Assessment record saved with its supporting evidence.","immutable counselor assessment saved")
    driver.wait_text(".saved-assessment-view h3",title,"saved assessment title")
    driver.wait_value("#record-title","","successful save clears draft")
    driver.wait_text(".saved-assessment-status","Recorded context matches", "new assessment source context")


def open_original_counselor_assessment(driver: WebDriver) -> None:
    if driver.command("GET", f"/element/{driver.find('.saved-assessments')}/property/open") is not True:
        driver.click(".saved-assessments > summary")
    driver.select("#saved-assessment","Taylor original assessment — 2026-09-01 to 2026-09-30 — Needs review")
    driver.wait_text(".saved-assessment-view h3","Taylor original assessment","persisted original counselor assessment")
    driver.wait_text(".saved-assessment-status","Needs review","changed source does not rewrite historical assessment")
    driver.wait_text(".saved-assessment-view .assessment-totals","Operating cash flow$199.98","original approved figures remain immutable")
    driver.wait_text(".saved-assessment-view .verified-balance-totals","Estimated net worth at balance date$525.05","original captured balance inventory")
    driver.wait_text(".saved-assessment-view","Verify remaining sources before changing the fictional household plan.","preserved counselor recommendation")


def export_counselor_evidence_pdf(driver: WebDriver, artifacts: Path, label: str) -> None:
    destination = artifacts / f"counselor-evidence-{label}.pdf"
    if destination.exists():
        raise SmokeFailure("Counselor evidence PDF destination must be new")
    driver.fill_pdf_destination("#assessment-pdf-path", destination)
    driver.click_button("Export saved assessment PDF")
    driver.wait_text(".saved-assessments > .success-message", str(destination), "exact counselor evidence export completion")
    text = subprocess.run(["pdftotext", "-layout", str(destination), "-"], check=True, capture_output=True, text=True).stdout
    text = " ".join(text.split())
    for expected in ["Casey captured counseling evidence", "Selected counseling session notes", "Casey fictional discussion USD 9999.99", "Historical total / smallest / largest: USD 32.05 / USD 10.01 / USD 12.03", "Fictional household service", "Needs review", "Reconfirmed all three source decisions after reversal", "Original reviewed payment", "Operating cash flow: USD -32.05", "Selected / available notes at capture: 1 / 1", "NEEDS REVIEW"]:
        if expected not in text:
            raise SmokeFailure(f"Counselor evidence PDF missing independently expected text: {expected}")
    if "Later Casey context excluded" in text or "Income receipts: USD 9999.99" in text:
        raise SmokeFailure("Later narrative replaced immutable evidence or financial amounts")
    status(f"counselor PDF preserves selected notes, original approval references, reviewed payment status and exact independent totals ({label})")


def save_counselor_evidence(driver: WebDriver, artifacts: Path) -> None:
    driver.fill("#new-note", "Casey fictional discussion USD 9999.99 is narrative only; verify future service obligations.")
    driver.click_button("Save session note")
    driver.wait_text(".notes-history", "Casey fictional discussion USD 9999.99", "saved counselor context before report")
    driver.click(".saved-assessments > summary")
    driver.interactive("#saved-assessment")
    driver.fill("#record-from", "2026-01")
    driver.fill("#record-through", "2026-03")
    driver.fill("#record-title", "Casey captured counseling evidence")
    driver.fill("#record-findings", "Fictional counseling context; payment confirmation needs renewed source review.")
    driver.click("#record-include-evidence")
    driver.click_button("Review report evidence")
    driver.wait_text(".report-evidence-context", "1 saved notes", "scoped saved notes before selection")
    driver.click(".report-evidence-selection .check-label input")
    driver.click("#record-ack")
    driver.click_button("Save assessment record")
    driver.wait_text(".saved-assessment-view h3", "Casey captured counseling evidence", "expanded counselor record saved")
    driver.wait_text(".captured-report-evidence", "Casey fictional discussion USD 9999.99", "immutable selected narrative")
    driver.wait_text(".captured-payment-group", "Fictional service · Needs review", "historical review stays invalidated after identical evidence returns")
    driver.wait_text(".captured-payment-group", "Total $32.05", "exact historical approved service total")
    driver.wait_text(".saved-assessment-view .assessment-totals", "Operating cash flow-$32.05", "narrative money does not enter approved totals")
    driver.fill("#new-note", "Later Casey context excluded from the original report.")
    driver.click_button("Save session note")
    driver.wait_text(".notes-history", "Later Casey context excluded", "additional immutable context")
    driver.wait_text(".saved-assessment-status", "Needs review", "new counseling note flags original report")
    if "Later Casey context excluded" in driver.text(".captured-report-evidence"):
        raise SmokeFailure("New note rewrote captured report context")
    export_counselor_evidence_pdf(driver, artifacts, "before-restart")
    driver.scroll_to(".captured-report-evidence", "start")
    driver.screenshot(artifacts / "native-counselor-report-evidence.png")
    driver.click(".saved-assessments > summary")


def assert_counselor_evidence_restored(driver: WebDriver, artifacts: Path, label: str) -> None:
    driver.open_client("Casey Pattern")
    if driver.command("GET", f"/element/{driver.element('.saved-assessments')}/property/open") is not True:
        driver.click(".saved-assessments > summary")
    driver.select("#saved-assessment", "Casey captured counseling evidence — 2026-01-01 to 2026-03-31 — Needs review")
    driver.wait_text(".captured-report-evidence", "Casey fictional discussion USD 9999.99", "restored immutable counseling context")
    driver.wait_text(".captured-payment-group", "Fictional service · Needs review", "restored historical decision applicability")
    driver.wait_text(".saved-assessment-status", "Needs review", "retained later-note context warning")
    driver.wait_text(".saved-assessment-view .assessment-totals", "Operating cash flow-$32.05", "restored approved figures remain independent")
    export_counselor_evidence_pdf(driver, artifacts, label)
    driver.click(".saved-assessments > summary")


def check_combined_scenarios(driver: WebDriver, artifacts: Path, label: str) -> None:
    driver.click(".combined-scenarios > details > summary")
    if driver.visible("#scenario-label"):
        raise SmokeFailure("A temporary combined proposal unexpectedly persisted")
    driver.click_button("Reload saved scenario baseline")
    driver.wait_value("#scenario-debt", "100.00", "saved debt baseline for combined comparison")
    driver.click(".scenario-budget-changes > summary")
    driver.fill("#scenario-budget-0", "1600")
    driver.fill("#scenario-budget-3", "100")
    driver.fill("#scenario-debt", "200")
    driver.fill("#scenario-emergency", "300")
    driver.select("#scenario-proposedStrategy", "Snowball")
    driver.click(".scenario-cash-changes > summary")
    driver.fill("#scenario-cash-date-0", "2026-11-08")
    driver.wait_value("#scenario-cash-date-0", "2026-11-08", "explicit ISO proposal date")
    driver.fill("#scenario-label", "Alex reviewed combined proposal")
    driver.fill("#scenario-assumptions", "Temporary synthetic proposal; retain saved plans")
    driver.click_button("Compare temporary proposal")
    driver.wait_text(".combined-scenario-results", "$983.34", "independent combined proposal allocation buffer")
    table=".combined-scenario-results table"
    for value in ["$3,250.00", "$3,466.67", "$35.35", "$23.01", "2027-02", "2027-11-30", "$3,300.00"]:
        driver.wait_text(table,value,"exact side-by-side combined financial outcome")
    driver.wait_value("#budget-amount-0", "1500.00", "combined preview does not write saved income")
    driver.wait_value("#debt-monthly-payment", "100.00", "combined preview does not write saved debt")
    driver.wait_value("#savings-emergencyContribution", "200.00", "combined preview does not write saved savings")
    driver.wait_value("#cashflow-date-0", "2026-11-03", "combined preview does not write saved dated schedules")
    driver.scroll_to(".combined-scenario-results table", "start")
    driver.screenshot(artifacts / f"native-combined-scenarios-{label}.png")
    if label == "initial":
        driver.click_button("Save reviewed combined scenario")
        driver.wait_text(".saved-scenario-history article", "$983.34", "immutable combined scenario recorded")
        driver.wait_text(".saved-scenario-status", "Saved baseline unchanged", "current baseline at scenario save")
    check_scenario_commitment_controls(driver, artifacts, label)
    driver.fill("#scenario-debt", "12.345")
    driver.click_button("Compare temporary proposal")
    driver.wait_text(".combined-scenarios [role=alert]", "two decimal", "invalid proposal precision is rejected")
    driver.wait_value("#scenario-debt", "12.345", "failed proposal retained for correction")
    driver.click_button("Reload saved scenario baseline")
    driver.wait_value("#scenario-debt", "100.00", "read-only reload restores original baseline")
    driver.click(".combined-scenarios > details > summary")
    status("combined exact what-if comparison, separate dated edits, unchanged saved models, rejected assumptions and temporary input reset")


def check_scenario_commitment_controls(driver: WebDriver, artifacts: Path, label: str) -> None:
    driver.click(".scenario-extra-payments > summary")
    driver.click_button("Add proposed extra payment")
    driver.fill("#scenario-extra-month-0", "2026-11")
    driver.fill("#scenario-extra-amount-0", "300")
    driver.click(".scenario-goal-contributions > summary")
    driver.fill("#scenario-goal-0", "100")
    driver.fill("#scenario-label", "Alex temporary income proposal")
    driver.click_button("Compare temporary proposal")
    for value in ["$833.34", "$12.20", "2026-12", "2026-12-31", "$500.00", "$3,300.00"]:
        driver.wait_text(".combined-scenario-results table", value, "exact extra-payment and savings-goal comparison")
    driver.wait_value("#savings-goal-monthlyContribution-0", "50.00", "goal proposal leaves the live contribution unchanged")
    driver.wait_value("#debt-monthly-payment", "100.00", "extra payment never changes the live base payment")
    driver.scroll_to(".combined-scenario-results table", "start")
    driver.screenshot(artifacts / f"native-scenario-controls-{label}.png")
    if label == "initial":
        driver.fill("#scenario-extra-amount-0", "12.345")
        driver.click_button("Compare temporary proposal")
        driver.wait_text(".combined-scenarios [role=alert]", "two decimal", "invalid extra payment precision rejected")
        driver.wait_value("#scenario-extra-amount-0", "12.345", "failed extra payment retained")
        driver.fill("#scenario-extra-amount-0", "300")
        driver.click_button("Compare temporary proposal")
        driver.wait_text(".combined-scenario-results table", "$833.34", "corrected proposal explicitly recomputed")
    check_recurring_expense_controls(driver, artifacts, label)
    if label == "initial":
        driver.click_button("Save reviewed combined scenario")
        driver.wait_text(".saved-scenario-history article", "$758.34", "integrated recurring-expense scenario recorded")
    status(f"extra debt payment and savings-goal controls preserve independent funding, milestones and live plans ({label})")


def check_recurring_expense_controls(driver: WebDriver, artifacts: Path, label: str) -> None:
    driver.click(".scenario-new-expenses > summary")
    driver.click_button("Add proposed recurring expense")
    driver.fill("#scenario-expense-label-0", "Alex proposed child care")
    driver.fill("#scenario-expense-amount-0", "75")
    driver.click_button("Compare temporary proposal")
    for value in ["$758.34", "$5,124.99", "2027-12-31", "$3,300.00"]:
        driver.wait_text(".combined-scenario-results table", value, "recurring budget expense recalculates shared funding and emergency target without inventing dates")
    driver.click("#scenario-expense-dated-0")
    driver.select("#scenario-expense-account-0", "Alex synthetic checking")
    driver.fill("#scenario-expense-first-0", "2026-11-15")
    driver.fill("#scenario-expense-until-0", "2026-11-30")
    if label == "initial":
        driver.fill("#scenario-expense-first-0", "2026-02-30")
        driver.click_button("Compare temporary proposal")
        driver.wait_text(".combined-scenarios [role=alert]", "calendar date", "invalid new expense date rejected")
        driver.wait_value("#scenario-expense-first-0", "2026-02-30", "rejected expense date retained")
        driver.fill("#scenario-expense-first-0", "2026-11-15")
    driver.click_button("Compare temporary proposal")
    for value in ["$758.34", "$3,225.00", "$12.20", "2026-12-31"]:
        driver.wait_text(".combined-scenario-results table", value, "explicit new expense bill and combined controls preserve exact independent outcomes")
    driver.wait_value("#budget-amount-0", "1500.00", "new expense does not change the live budget")
    if driver.visible("#budget-amount-6"):
        raise SmokeFailure("New proposal expense unexpectedly entered the live budget")
    driver.scroll_to(".combined-scenario-results table", "start")
    driver.screenshot(artifacts / f"native-recurring-expenses-{label}.png")
    status(f"new recurring expense updates shared funding and only explicitly selected dated bills ({label})")
    check_temporary_income_controls(driver, artifacts, label)


def check_temporary_income_controls(driver: WebDriver, artifacts: Path, label: str) -> None:
    driver.click(".scenario-income-periods > summary")
    driver.click_button("Add temporary income period")
    driver.select("#scenario-income-line-0", "Alex synthetic wages · saved 1500.00 · biweekly")
    driver.fill("#scenario-income-amount-0", "800")
    driver.fill("#scenario-income-from-0", "2026-11")
    driver.fill("#scenario-income-through-0", "2026-11")
    driver.click_button("Compare temporary proposal")
    for value in ["-$975.00", "$1,733.33", "2026-12", "Normal / recovery", "$858.34"]:
        driver.wait_text(".income-period-results", value, "independent temporary income funding and normal recovery")
    driver.wait_text(".combined-scenario-results table", "$3,225.00", "monthly-only income reduction does not change dated receipts")
    driver.select("#scenario-income-paycheck-0", "Alex synthetic dated pay · 1500.00 · biweekly")
    if label == "initial":
        driver.fill("#scenario-income-through-0", "2026-10")
        driver.click_button("Compare temporary proposal")
        driver.wait_text(".combined-scenarios [role=alert]", "ordered", "reversed temporary period rejected")
        driver.wait_value("#scenario-income-through-0", "2026-10", "failed temporary period retained")
        driver.fill("#scenario-income-through-0", "2026-11")
    driver.click_button("Compare temporary proposal")
    driver.wait_text(".combined-scenario-results table", "$1,825.00", "selected November receipts change by exactly 1400.00")
    driver.wait_text(".income-period-results", "-$975.00", "monthly funding deficit remains explicit")
    driver.wait_value("#budget-amount-0", "1500.00", "temporary income never changes the saved normal income")
    driver.scroll_to(".income-period-results", "start")
    driver.screenshot(artifacts / f"native-temporary-income-{label}.png")
    status(f"temporary income periods, deficits, normal recovery and explicitly selected dated receipts preserve live plans ({label})")


def export_scenario_controls_pdf(driver: WebDriver, artifacts: Path, label: str) -> None:
    destination = artifacts / f"temporary-income-{label}.pdf"
    if destination.exists():
        raise SmokeFailure("Extended scenario PDF destination must be new")
    driver.fill_pdf_destination("#assessment-pdf-path", destination)
    driver.click_button("Export saved assessment PDF")
    driver.wait_text(".saved-assessments > .success-message", str(destination), "extended scenario PDF completion")
    text = subprocess.run(["pdftotext", "-layout", str(destination), "-"], check=True, capture_output=True, text=True).stdout
    text = " ".join(text.split())
    for value in ["Alex temporary income proposal", "Additional proposed debt payment: 2026-11 / USD 300.00", "Proposed goal monthly contribution: Alex synthetic insurance reserve / USD 100.00", "Baseline USD 766.67 / proposed USD 758.34", "Projected total interest: USD 12.20", "Projected payoff month: 2026-12", "2026-12-31", "New proposed recurring expense: Alex proposed child care / Necessary / USD 75.00 per Monthly / estimated", "Explicit expense payment dates: Alex synthetic checking / first 2026-11-15", "Baseline USD 3300.00 / proposed USD 1825.00", "Primary emergency completion: 2027-12-31", "Temporary income funding and recovery", "2026-11 / Reduced", "2026-12 / Normal / recovery", "USD 3466.67 / USD 1733.33", "USD 758.34 / USD -975.00", "USD 858.34 / USD 858.34", "Operating cash flow: USD 1820.02"]:
        if value not in text:
            raise SmokeFailure(f"Extended scenario PDF missing {value}")
    status(f"temporary income PDF preserves period funding, recovery, explicit receipts and separate approved activity ({label})")


def save_scenario_controls_report(driver: WebDriver, artifacts: Path) -> None:
    driver.click(".saved-assessments > summary")
    driver.wait(lambda: driver.visible("#saved-assessment") is not None, "extended report directory")
    driver.fill("#record-from", "2026-09")
    driver.fill("#record-through", "2026-10")
    driver.fill("#record-title", "Alex temporary income report")
    driver.fill("#record-findings", "Review extra payment funding and goal commitments; live plans stay unchanged.")
    driver.click("#record-include-saved-scenario")
    driver.click_button("Review saved combined scenarios")
    driver.select("#record-saved-scenario", "Alex temporary income proposal — Baseline unchanged")
    driver.wait_text(".report-saved-scenario", "$758.34", "explicit extended scenario selection")
    driver.click("#record-ack")
    driver.click_button("Save assessment record")
    driver.wait_text(".saved-assessment-view h3", "Alex temporary income report", "extended scenario report recorded")
    driver.wait_text(".selected-combined-scenario", "$758.34", "extended scenario linked by server")
    export_scenario_controls_pdf(driver, artifacts, "initial")
    driver.click(".saved-assessments > summary")


def assert_scenario_controls_restored(driver: WebDriver, artifacts: Path, label: str) -> None:
    driver.click(".combined-scenarios > details > summary")
    driver.click_button("Reload saved scenario history")
    driver.select("#saved-scenario", "Alex temporary income proposal — Needs review")
    driver.wait_text(".saved-scenario-history article", "$758.34", "original extended outcome survives recovery")
    driver.wait_text(".saved-scenario-history article", "2026-12-31", "original goal milestone survives recovery")
    driver.wait_value("#savings-goal-monthlyContribution-0", "50.00", "reopened goal proposal never applies live changes")
    driver.click(".combined-scenarios > details > summary")
    driver.click(".saved-assessments > summary")
    driver.select("#saved-assessment", "Alex temporary income report — 2026-09-01 to 2026-10-31 — Needs review")
    driver.wait_text(".selected-combined-scenario", "$758.34", "immutable extended report restored")
    driver.wait_text(".selected-combined-scenario .income-period-results", "-$975.00", "original reduced-income deficit restored")
    export_scenario_controls_pdf(driver, artifacts, label)
    driver.click(".saved-assessments > summary")
    status(f"temporary income scenario and report retain original monthly funding across recovery ({label})")


def export_saved_combined_pdf(driver: WebDriver, artifacts: Path, label: str) -> None:
    destination = artifacts / f"saved-combined-{label}.pdf"
    if destination.exists():
        raise SmokeFailure("Saved combined PDF destination must be new")
    driver.fill_pdf_destination("#assessment-pdf-path", destination)
    driver.click_button("Export saved assessment PDF")
    driver.wait_text(".saved-assessments > .success-message", str(destination), "selected combined scenario PDF completion")
    text = subprocess.run(["pdftotext", "-layout", str(destination), "-"], check=True, capture_output=True, text=True).stdout
    text = " ".join(text.split())
    for value in ["Selected saved combined scenario / hypothetical", "Alex reviewed combined proposal", "Baseline USD 3250.00 / proposed USD 3466.67", "Baseline USD 766.67 / proposed USD 983.34", "Projected total interest: USD 23.01", "Primary emergency completion: 2027-11-30", "Operating cash flow: USD 1820.02"]:
        if value not in text:
            raise SmokeFailure(f"Selected scenario PDF missing {value}")
    status(f"saved combined report PDF preserves baseline, proposed milestones and separate approved activity ({label})")


def save_combined_report(driver: WebDriver, artifacts: Path) -> None:
    driver.click(".saved-assessments > summary")
    driver.wait(lambda: driver.visible("#saved-assessment") is not None, "combined report directory")
    driver.fill("#record-from", "2026-09")
    driver.fill("#record-through", "2026-10")
    driver.fill("#record-title", "Alex selected combined report")
    driver.fill("#record-findings", "Review hypothetical proposals; live plans remain unchanged.")
    driver.click("#record-include-saved-scenario")
    driver.click_button("Review saved combined scenarios")
    driver.select("#record-saved-scenario", "Alex reviewed combined proposal — Baseline unchanged")
    driver.wait_text(".report-saved-scenario", "$983.34", "explicit reviewed saved proposal selection")
    driver.click("#record-ack")
    driver.click_button("Save assessment record")
    driver.wait_text(".saved-assessment-view h3", "Alex selected combined report", "selected saved scenario report recorded")
    driver.wait_text(".selected-combined-scenario", "$983.34", "immutable comparison attached by server")
    driver.wait_text(".saved-assessment-view .scenario-budget-totals", "Income$3,466.67", "report uses selected proposal")
    export_saved_combined_pdf(driver, artifacts, "initial")
    driver.click(".saved-assessments > summary")


def assert_saved_combined_restored(driver: WebDriver, artifacts: Path, label: str) -> None:
    driver.open_client("Alex Fictional")
    driver.click(".combined-scenarios > details > summary")
    driver.click_button("Reload saved scenario history")
    driver.select("#saved-scenario", "Alex reviewed combined proposal — Needs review")
    driver.wait_text(".saved-scenario-status", "historical baseline", "later versions flag immutable saved comparison")
    driver.wait_text(".saved-scenario-history article", "$983.34", "original combined outcome after recovery")
    driver.wait_value("#budget-amount-0", "1500.00", "reopened proposal never applies live income")
    driver.click(".combined-scenarios > details > summary")
    driver.click(".saved-assessments > summary")
    driver.select("#saved-assessment", "Alex selected combined report — 2026-09-01 to 2026-10-31 — Needs review")
    driver.wait_text(".selected-combined-scenario", "$983.34", "linked comparison restored and verified")
    driver.wait_text(".saved-assessment-view .scenario-budget-totals", "Income$3,466.67", "original proposed model set retained")
    export_saved_combined_pdf(driver, artifacts, label)
    driver.click(".saved-assessments > summary")
    assert_scenario_controls_restored(driver, artifacts, label)
    status(f"saved scenarios and report relationships persist without rewriting plans ({label})")


def check_budget_methods(driver: WebDriver, artifacts: Path, label: str) -> None:
    if driver.command("GET", f"/element/{driver.element('.budget-methods')}/property/open") is not True:
        driver.click(".budget-methods > summary")
    driver.wait_value("#method-minimum", "", "framework assumptions do not carry across restart/client changes")
    driver.click_button("Compare saved budget")
    driver.wait_text(".method-zero", "Unassigned / deficit$716.67", "zero-based framework preserves original unassigned amount")
    driver.wait_text("table[aria-label='Optional budget percentage comparison']", "Unavailable", "unknown required minimums never become zero")
    driver.fill("#method-minimum", "100")
    driver.click_button("Compare saved budget")
    for expected in ["$1,733.33", "$1,625.00", "$108.33", "-$775.00", "$600.00", "$1,950.00", "-$216.67", "-$50.00"]:
        driver.wait_text("table[aria-label='Optional budget percentage comparison']", expected, "independently expected reference comparison " + expected)
    driver.wait_text(".method-zero", "Spending and allocations assigned$2,533.33", "minimum reclassification does not duplicate payments")
    driver.fill("#method-needsPercent", "61")
    driver.click_button("Compare saved budget")
    driver.wait_text(".budget-methods [role=alert]", "total exactly 100.00%", "invalid percentages rejected without changing saved plan")
    driver.wait_value("#method-needsPercent", "61", "failed assumptions retained")
    driver.fill("#method-needsPercent", "60")
    driver.click_button("Compare saved budget")
    driver.wait_text(".method-zero", "Unassigned / deficit$716.67", "saved plan unaffected by comparison failure")
    driver.scroll_to(".budget-method-results", "start")
    driver.screenshot(artifacts / f"native-budget-methods-{label}.png")
    driver.click(".budget-methods > summary")
    status(f"optional budget frameworks, exact targets, minimum reclassification, unknowns and failed comparison leave saved finances unchanged ({label})")


def export_original_assessment_pdf(driver: WebDriver, artifacts: Path, label: str) -> None:
    destination = artifacts / f"saved-assessment-{label}.pdf"
    if destination.exists():
        raise SmokeFailure("PDF smoke destination must be new; never overwrite an earlier export")
    driver.fill_pdf_destination("#assessment-pdf-path", destination)
    driver.click_button("Export saved assessment PDF")
    driver.wait_text(".saved-assessments > .success-message", str(destination), "visible native PDF export completion for selected destination")
    original = destination.read_bytes()
    if not original.startswith(b"%PDF-"):
        raise SmokeFailure("Native export did not create a PDF")
    text = subprocess.run(["pdftotext", "-layout", str(destination), "-"], check=True, capture_output=True, text=True).stdout
    text = " ".join(text.split())
    for expected in ["Later decisions do not rewrite them", "Taylor original assessment", "Taylor Balance", "Smoke Test Counseling", "NEEDS REVIEW", "Operating cash flow: USD 199.98", "Estimated net worth at balance date: USD 525.05", "Verify remaining sources", "Captured transaction evidence", "INTERNAL / INCOMPLETE"]:
        if expected not in text:
            raise SmokeFailure(f"Native PDF missing independently expected text: {expected}")
    if "This live view is recalculated" in text:
        raise SmokeFailure("Live-dashboard wording appeared in a saved historical PDF")
    if "Operating cash flow: USD 500.00" in text:
        raise SmokeFailure("Live figures replaced the immutable historical PDF")
    driver.click_button("Export saved assessment PDF")
    driver.wait_text(".saved-assessments .alert.error", "already exists", "PDF overwrite safely rejected")
    if destination.read_bytes() != original:
        raise SmokeFailure("Rejected overwrite damaged PDF")
    status(f"local printable PDF export preserved historical evidence and figures ({label})")


def export_client_action_pdf(driver: WebDriver, artifacts: Path, label: str, needs_review: bool = True) -> None:
    destination = artifacts / f"client-action-plan-{label}.pdf"
    if destination.exists():
        raise SmokeFailure("Client PDF smoke destination must be new")
    driver.fill_pdf_destination("#client-plan-pdf-path", destination)
    driver.click_button("Export client action plan PDF")
    driver.wait_text(".client-action-plans > .success-message", str(destination), "client PDF export completion for selected destination")
    text = subprocess.run(["pdftotext", "-layout", str(destination), "-"], check=True, capture_output=True, text=True).stdout
    text = " ".join(text.split())
    for expected in ["Your financial action plan", "Alex agreed client plan", "America/Chicago", "Agreed debt allocation: USD 100.00", "Agreed primary savings allocation: USD 250.00", "Unallocated / buffer: USD 1066.67", "Operating cash flow: USD 1820.02", "Strategy: Avalanche", "Projected total interest: USD 35.35", "Projected emergency completion: 2028-06-30", "Incomplete", "Follow-up arrangements"]:
        if expected.lower() not in text.lower():
            raise SmokeFailure(f"Client action plan PDF missing expected text: {expected}")
    if "Selected counseling session notes" in text or "Casey fictional discussion" in text:
        raise SmokeFailure("Internal counselor note evidence entered a client PDF")
    for private in ["Selected intake context", "Alex fictional employer", "Two fictional adults and one dependent", "Entered intake context only"]:
        if private in text:
            raise SmokeFailure(f"Internal intake context entered a client PDF: {private}")
    if needs_review and "NEEDS REVIEW" not in text:
        raise SmokeFailure("Historical client plan omitted its review flag")
    if text.index("Verify missing statements") > text.index("Confirm lender payment dates"):
        raise SmokeFailure("Client plan priority order changed")
    status(f"client PDF preserves agreed allocations, priorities, conditional milestones and historical basis ({label})")


def assert_client_action_restored(driver: WebDriver, artifacts: Path, label: str) -> None:
    if driver.command("GET", f"/element/{driver.element('.client-action-plans')}/property/open") is not True:
        driver.click(".client-action-plans > summary")
    driver.interactive("#saved-action-plan")
    option = driver.observe_script("return Array.from(document.querySelector('#saved-action-plan').options).find(x=>x.textContent.includes('Alex agreed client plan'))?.textContent")
    if not option or "Needs review" not in option:
        raise SmokeFailure("Restored client plan directory omitted historical review flag")
    driver.select("#saved-action-plan", option)
    driver.wait_text(".saved-client-action-plan h3", "Alex agreed client plan", "restored client plan")
    driver.wait_text(".action-plan-status", "Needs review", "client agreement inherits changed supporting context")
    driver.wait_text(".saved-client-action-plan .action-allocation-totals", "Unallocated / buffer$1,066.67", "immutable agreed model payments")
    driver.wait_text(".saved-client-action-plan", "Verify missing statements", "persisted priorities")
    export_client_action_pdf(driver, artifacts, label)


def save_client_action_plan(driver: WebDriver, artifacts: Path, export_label: str = "current") -> None:
    driver.click(".client-action-plans > summary")
    driver.interactive("#saved-action-plan")
    driver.click_button("Review client plan basis")
    driver.wait_text(".action-plan-basis", "Unallocated / buffer$1,066.67", "authoritative monthly agreement from selected models")
    driver.fill("#action-plan-title", "Alex agreed client plan")
    driver.fill("#action-budget-recommendations", "Protect fictional bills; agree on selected model contributions and verify actual lender dates.")
    for i, step in enumerate(["Verify missing statements", "Confirm lender payment dates", "Arrange primary savings contributions"]):
        driver.fill(f"#action-step-{i}", step)
    driver.select("#action-owner-1", "Counselor")
    driver.fill("#action-follow-up", "Arrange a fictional appointment after verifying the missing records.")
    driver.fill("#action-limitations", "Entered estimates; statement coverage remains incomplete and forecasts are conditional.")
    driver.click("#action-plan-ack")
    driver.click_button("Record client action plan")
    driver.wait_text(".saved-client-action-plan h3", "Alex agreed client plan", "recorded client plan")
    driver.wait_text(".action-plan-status", "Recorded basis matches", "current recorded client agreement")
    export_client_action_pdf(driver, artifacts, export_label, False)
    driver.interactive("#client-plan-pdf-path")
    driver.scroll_to(".saved-client-action-plan", "start")
    driver.screenshot(artifacts / "native-client-action-plan.png")


def export_captured_planning_pdf(driver: WebDriver, artifacts: Path, label: str) -> None:
    destination = artifacts / f"saved-planning-{label}.pdf"
    if destination.exists():
        raise SmokeFailure("Planning PDF smoke destination must be new")
    driver.fill_pdf_destination("#assessment-pdf-path", destination)
    driver.click_button("Export saved assessment PDF")
    driver.wait_text(".saved-assessments > .success-message", str(destination), "native planning scenario PDF completion for selected destination")
    text = subprocess.run(["pdftotext","-layout",str(destination),"-"],check=True,capture_output=True,text=True).stdout
    text = " ".join(text.split())
    for expected in ["Alex saved planning scenario","Captured planning scenario / hypothetical","Monthly income: USD 3250.00","Unallocated: USD 716.67","Selected strategy: Avalanche","Projected total interest: USD 35.35","Emergency target: USD 4899.99","Primary emergency completion: 2028-06-30","Projected combined closing balance: USD 3300.00","Operating cash flow: USD 1820.02","NEEDS REVIEW"]:
        if expected not in text:
            raise SmokeFailure(f"Captured planning PDF missing expected text: {expected}")
    if "Selected counseling session notes" not in text or "Selected / available notes at capture: 1 / 1" not in text or "Casey fictional discussion" in text:
        raise SmokeFailure("Captured planning report lost its selected note or mixed clients")
    for expected in ["Selected intake context", "Alex fictional employer", "Two fictional adults and one dependent", "not verified financial facts"]:
        if expected not in text:
            raise SmokeFailure(f"Selected intake missing from internal PDF: {expected}")
    for excluded in ["Later omitted fictional address", "Unrelated fictional appointment outcome", "Alex changed employer"]:
        if excluded in text:
            raise SmokeFailure(f"Unselected or later intake leaked into report: {excluded}")
    if "Monthly income: USD 3466.67" in text:
        raise SmokeFailure("Later plan income replaced captured PDF assumptions")
    status(f"native planning PDF preserves original assumptions and independently expected milestones ({label})")


def assert_captured_planning_restored(driver: WebDriver, artifacts: Path, label: str) -> None:
    driver.open_client("Alex Fictional")
    if driver.command("GET",f"/element/{driver.element('.saved-assessments')}/property/open") is not True:
        driver.click(".saved-assessments > summary")
    driver.select("#saved-assessment","Alex saved planning scenario — 2026-09-01 to 2026-10-31 — Needs review")
    driver.wait_text(".saved-assessment-view h3","Alex saved planning scenario","restored scenario record")
    driver.wait_text(".saved-assessment-status","Needs review","reverted active plans never reactivate an old captured version")
    driver.wait_text(".saved-assessment-view .scenario-budget-totals","Income$3,250.00","captured original normalized income")
    driver.wait_text(".saved-assessment-view .scenario-budget-totals","Unallocated$716.67","captured original allocations")
    driver.wait_text(".saved-assessment-view .scenario-debt-summary","Selected strategy: Avalanche","preserved explicit strategy")
    driver.wait_text(".saved-assessment-view .scenario-debt-outcome","Total interest $35.35","captured independent avalanche outcome")
    driver.wait_text(".saved-assessment-view .scenario-savings-outcome","2028-06-30","captured savings milestone")
    driver.wait_text(".saved-assessment-view .scenario-cashflow-outcome","$3,300.00","separate captured per-account forecast")
    driver.wait_text(".saved-assessment-view .assessment-totals","Operating cash flow$1,820.02","approved history remains separate from entered planning")
    driver.wait_text(".saved-assessment-view .report-intake-context", "Alex fictional employer", "captured original employment remains historical")
    driver.wait_text(".saved-assessment-view .report-intake-context", "Two fictional adults and one dependent", "selected household survives recovery")
    export_captured_planning_pdf(driver,artifacts,label)
    assert_client_action_restored(driver, artifacts, label)
    driver.click(".saved-assessments > summary")


def save_captured_planning_assessment(driver: WebDriver, artifacts: Path) -> None:
    driver.open_client("Alex Fictional")
    check_combined_scenarios(driver, artifacts, "initial")
    save_combined_report(driver, artifacts)
    save_scenario_controls_report(driver, artifacts)
    driver.click(".saved-assessments > summary")
    driver.wait(lambda:driver.visible("#saved-assessment") is not None,"saved assessment directory before scenario")
    driver.fill("#record-from","2026-09")
    driver.fill("#record-through","2026-10")
    driver.fill("#record-title","Alex saved planning scenario")
    driver.fill("#record-recommendations","Protect fictional bills while reviewing the selected debt and savings plan.")
    driver.click("#record-include-planning")
    driver.click_button("Review current saved plans")
    driver.wait_text(".scenario-selection .scenario-budget-totals","Income$3,250.00","reviewed saved planning baseline")
    driver.fill("#record-scenario-label","Alex primary saved plan")
    driver.fill("#record-scenario-assumptions","Entered hypothetical plans; verify lender dates and all financial coverage before commitments.")
    driver.select("#record-debt-strategy","Avalanche")
    driver.click("#record-include-evidence")
    driver.click_button("Review report evidence")
    driver.wait_text(".report-evidence-context", "1 saved notes", "Alex current counselor context stays scoped")
    driver.click(".report-evidence-selection .check-label input")
    driver.click("#record-include-intake")
    driver.click_button("Review saved intake")
    driver.interactive("#record-intake-household")
    driver.click("#record-intake-household")
    driver.click("#record-intake-employment")
    driver.wait_text(".report-intake-selection", "Alex fictional employer", "explicitly reviewed saved employment context")
    driver.click("#record-ack")
    driver.click_button("Save assessment record")
    driver.wait_text(".saved-assessment-view h3","Alex saved planning scenario","saved scenario-backed assessment")
    driver.wait_text(".saved-assessment-status","Recorded context matches","new captured scenario context")
    # The report status can render before its post-save directory refresh has
    # finished. That refresh temporarily protects the adjacent intake editor.
    driver.interactive("#saved-assessment")
    if driver.command("GET", f"/element/{driver.element('.client-care')}/property/open") is not True:
        driver.click(".client-care > summary")
    driver.click("#intake-address")
    driver.wait(lambda:driver.observe_script("return document.activeElement?.id==='intake-address'"),"native intake field focused after report refresh")
    driver.fill("#intake-address", "Later omitted fictional address")
    driver.fill("#follow-notes-0", "Unrelated fictional appointment outcome")
    driver.wait_value("#intake-address", "Later omitted fictional address", "omitted address replacement")
    driver.wait_value("#follow-notes-0", "Unrelated fictional appointment outcome", "unrelated outcome replacement")
    driver.click_button("Save intake and follow-ups")
    driver.wait_text(".client-care .success-message", "saved locally", "unselected care changes saved")
    driver.interactive("#record-title")
    driver.wait_text(".saved-assessment-status", "Recorded context matches", "unselected care changes do not invalidate report")
    driver.click(".client-care > summary")
    save_client_action_plan(driver, artifacts)
    driver.click(".client-care > summary")
    driver.fill("#intake-employer", "Alex changed employer")
    driver.wait_value("#intake-employer", "Alex changed employer", "selected employer replacement")
    driver.click_button("Save intake and follow-ups")
    driver.wait_text(".saved-assessment-status", "Needs review", "selected intake change flags report")
    driver.interactive("#intake-employer")
    driver.fill("#intake-employer", "Alex fictional employer")
    driver.wait_value("#intake-employer", "Alex fictional employer", "selected employer restored")
    driver.click_button("Save intake and follow-ups")
    driver.interactive("#record-title")
    driver.wait_text(".saved-assessment-status", "Needs review", "selected intake reversion never reactivates historical report")
    driver.scroll_to(".saved-assessment-view .report-intake-context", "start")
    driver.command("POST", "/execute/async", {"script":"const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));", "args":[]})
    driver.screenshot(artifacts / "native-selected-report-intake.png")
    driver.click(".client-care > summary")
    status("selected report intake: unselected address/outcome ignored; selected employer change and reversion permanently flagged")
    driver.fill("#budget-amount-0","1600")
    driver.click_button("Save budget and calculate")
    driver.wait_text(".budget-panel .budget-totals","$3,466.67","independent later income change")
    driver.wait_text(".saved-assessment-status","Needs review","plan revision invalidates recorded context automatically")
    driver.wait_text(".saved-assessment-view .scenario-budget-totals","Income$3,250.00","original captured income after active plan change")
    driver.fill("#budget-amount-0","1500")
    driver.click_button("Save budget and calculate")
    driver.wait_text(".budget-panel .budget-totals","$716.67","active plan reverted to original inputs")
    driver.wait_text(".saved-assessment-status","Needs review","reverting values does not restore captured plan version")
    export_captured_planning_pdf(driver,artifacts,"before-restart")
    driver.wait_text(".action-plan-status", "Needs review", "active plan edits flag recorded client agreement")
    export_client_action_pdf(driver, artifacts, "before-restart")
    driver.interactive(".assessment-export .button.primary")
    driver.scroll_to(".saved-assessment-view .captured-planning", "start")
    driver.screenshot(artifacts / "native-captured-planning-scenario.png")
    driver.click(".saved-assessments > summary")
    status("captured scenario selection, debt/savings/cash-flow assumptions, historical isolation and permanent plan-version review flags")


def view_financial_dashboard(driver: WebDriver, start: str, end: str) -> None:
    driver.fill("#assessment-from", start)
    driver.fill("#assessment-through", end)
    driver.click_button("View financial dashboard")
    driver.wait_text(".assessment-basis", f"{start}-01 through", "explicit approved assessment period")
    driver.wait_text(".assessment-basis", "Approved records only · Incomplete", "honest dashboard basis")


def assert_alex_dashboard(driver: WebDriver) -> None:
    view_financial_dashboard(driver,"2026-09","2026-10")
    driver.wait_text(".assessment-totals","Approved income receipts$3,000.01", "approved CSV and PDF income assessment")
    driver.wait_text(".assessment-totals","Operating cash flow$1,820.02", "approved net spending after refunds")
    driver.wait_text(".assessment-review","Needs Clarification 1", "pending PDF expense excluded")
    driver.wait_text("table[aria-label='Monthly approved income and spending']","2026-09", "local monthly comparison chart")
    driver.wait_text("table[aria-label='Monthly approved income and spending']","$2,000.00", "exact September receipts")
    driver.wait_text("table[aria-label='Monthly approved income and spending']","$1,000.01", "exact October approved payroll")
    driver.wait_text("table[aria-label='Monthly operating cash flow']","$820.01", "September operating cashflow")
    driver.wait_text("table[aria-label='Approved spending categories']","Housing", "approved category chart")
    view_financial_dashboard(driver,"2026-10","2026-10")
    driver.wait_text(".assessment-totals","Approved income receipts$1,000.01", "assessment dates exclude September")
    driver.wait_text(".assessment-totals","Gross spending$0.00", "unapproved October spending stays absent")
    driver.wait_text(".financial-dashboard","No approved spending in this period.", "no invented category chart")
    view_financial_dashboard(driver,"2026-09","2026-10")


def set_fact_classification(driver: WebDriver, description: str, choice: str) -> None:
    label = f"//label[contains(.,{json.dumps('Classification for '+description,ensure_ascii=False)})]/select"
    driver.select(label,choice,"xpath")


def save_financial_facts(driver: WebDriver, reason: str) -> None:
    driver.fill("#facts-reason",reason)
    driver.click("#facts-ack")
    driver.click_button("Save verified financial facts")
    driver.wait_text(".imports-panel","Verified financial facts saved; dashboard updated.","explicit verified fact save")
    driver.wait_value("#facts-reason","","successful fact save clears reason")


def initial_balance_facts(driver: WebDriver) -> None:
    driver.click(".financial-facts-editor > summary")
    set_fact_classification(driver,"Fictional wages","Verified take-home pay")
    set_fact_classification(driver,"Fictional rent","Necessary expense")
    set_fact_classification(driver,"Fictional fee","Discretionary expense")
    driver.fill("#facts-as-of","2026-09-30")
    rows=[("Fictional checking","Cash / checking","100.01",None),
          ("Fictional emergency","Accessible, unreserved savings","600.05",None),
          ("Fictional reserve","Reserved / restricted savings","25.02",None),
          ("Fictional loan","Debt owed","200.03","25.01")]
    for i,(label,kind,amount,payment) in enumerate(rows):
        driver.click_button("Add verified balance")
        driver.fill(f"#fact-balance-label-{i}",label)
        driver.select(f"#fact-balance-kind-{i}",kind)
        driver.fill(f"#fact-balance-amount-{i}",amount)
        if payment is not None: driver.fill(f"#fact-balance-payment-{i}",payment)
        driver.fill(f"#fact-balance-evidence-{i}","Independently verified fictional balance and required payment at September 30; separate from running.csv arithmetic")
    driver.click("#facts-inventory")
    save_financial_facts(driver,"Verified dated fictional inventory and three source classifications")


def assert_statement_balances_restored(driver: WebDriver, artifacts: Path, label: str) -> None:
    driver.open_client("Taylor Balance")
    driver.wait_text(".imports-progress","Approved 2 · Rejected 1","persisted balance decisions")
    choose_balance_statement(driver)
    driver.wait_text(".balance-progress","Source: 3 matched · 0 differences. Approved: 1 matched · 1 differences.","persisted source and approved comparisons")
    driver.wait_text(".balance-check-table","Difference $300.02","persisted rejection discrepancy")
    driver.wait_text(".imports-totals","$500.00","persisted exact approved balance client net")
    driver.click(".statement-balance-history > summary")
    driver.wait_text(".statement-balance-history","Statement balance setting history (2)","persisted balance setting history")
    driver.wait_text(".statement-balance-history","Verified original source is oldest first","persisted source-order review reason")
    view_financial_dashboard(driver,"2026-09","2026-09")
    driver.wait_text(".assessment-totals","Operating cash flow$500.00", "recomputed dashboard from persisted approved decisions")
    driver.wait_text(".verified-balance-totals","Estimated net worth at balance date$525.05","persisted encrypted balance facts")
    driver.wait_text(".classified-totals","Classified take-home pay receipts$500.01","persisted verified take-home classification")
    driver.wait_text(".verified-fact-metrics","Classifications needing re-verification across this client: 1","persisted source-bound invalidation")
    driver.click(".financial-facts-editor > summary")
    driver.wait_text(".financial-facts-history","Financial fact history (2)","persisted immutable financial facts history")
    driver.click(".financial-facts-editor > summary")
    open_original_counselor_assessment(driver)
    export_original_assessment_pdf(driver, artifacts, label)
    driver.click(".saved-assessments > summary")


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
    driver.scroll_to(".budget-results", "center")
    driver.screenshot(artifacts / "native-budget.png")
    status("exact client budget calculation, encrypted save, and allocations")
    check_budget_methods(driver, artifacts, "initial")

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
    driver.scroll_to(".debt-results", "start")
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
    driver.scroll_to(".savings-results", "start")
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
    driver.scroll_to(".cashflow-calendar", "center")
    driver.screenshot(artifacts / "native-cashflow-calendar.png")
    driver.click_button("Chronological ledger")
    driver.wait_text(".cashflow-ledger", "-$700.00", "chronological running balance")
    driver.wait_text(".cashflow-ledger", "Alex synthetic account transfer", "paired transfer in ledger")
    driver.scroll_to(".cashflow-ledger", "center")
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
    check_client_care(driver, artifacts, "initial", initial=True)

    driver.open_client("Alex Fictional")
    driver.wait_text(".notes-history", note, "first client's note after selection")
    driver.select("#client-status", "Planning")
    driver.fill("#goals", edited_goal)
    driver.click_button("Save changes")
    driver.wait_value("#goals", edited_goal, "updated counseling goal")
    driver.wait_text(".editor-panel .status-tag", "Planning", "updated workflow status")
    driver.screenshot(artifacts / "native-desktop.png")
    status("client edits saved")
    check_workspace_pages(driver, artifacts)
    check_document_collection(driver, artifacts, "initial", initial=True)

    driver.click_button("Lock workspace")
    driver.element("#password")
    private_values = [f"{first_name} {first_last}", f"{second_name} {second_last}",
                      first_goal, edited_goal, note, organization, counselor, "Alex fictional employer", "Reviewed the synthetic documents", "Alex synthetic wages", "Alex synthetic small debt", "Alex synthetic high APR debt", "Alex synthetic reserve priority", "Alex synthetic insurance reserve", "Alex synthetic checking", "Alex synthetic cash-flow priority", "Alex synthetic dated pay", "Synthetic reserve transfer", "Synthetic incoming transfer", "Synthetic checking", "checking.csv", "Fictional September checking statement", "Fictional optional pay stub", "Fictional receipt recorded; financial approval remains separate.", "Fictional payroll", "Synthetic PDF checking"]

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
    create_client("Casey","Pattern","Review fictional recurring payments")
    check_recurring_imports(driver,artifacts,data_root)
    create_client("Taylor","Balance","Verify fictional statement running balances")
    check_statement_balances(driver,artifacts,data_root)
    save_captured_planning_assessment(driver,artifacts)
    restart_driver()
    driver.open(binary)
    driver.fill("#password",password)
    driver.click_button("Unlock workspace")
    assert_recurring_restored(driver)
    check_client_care(driver, artifacts, "restart")
    check_document_collection(driver, artifacts, "restart")
    assert_statement_balances_restored(driver, artifacts, "restart")
    assert_captured_planning_restored(driver, artifacts, "restart")
    check_budget_methods(driver, artifacts, "restart")
    check_combined_scenarios(driver, artifacts, "restart")
    assert_saved_combined_restored(driver, artifacts, "restart")
    assert_counselor_evidence_restored(driver, artifacts, "restart")
    driver.open_client("Alex Fictional")
    assert_alex_dashboard(driver)
    driver.scroll_to(".financial-dashboard", "start")
    driver.screenshot(artifacts / "native-financial-dashboard.png")
    driver.wait_text(".recurring-review","Synthetic rent","Alex has his own payment groups")
    if "Fictional household service" in driver.text(".recurring-review"):
        raise SmokeFailure("Recurring decisions crossed client boundaries")

    # Exercise recovery only in this synthetic app-data directory. No native
    # commands are invoked directly; passwords and paths use visible controls.
    backup_folder=data_root / "external-backups"
    backup_folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']")
    driver.click("nav[aria-label='Settings sections'] button:nth-child(3)")
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
    assert_recurring_restored(driver)
    check_client_care(driver, artifacts, "encrypted-restore")
    check_document_collection(driver, artifacts, "encrypted-restore")
    assert_statement_balances_restored(driver, artifacts, "encrypted-restore")
    assert_captured_planning_restored(driver, artifacts, "encrypted-restore")
    check_budget_methods(driver, artifacts, "encrypted-restore")
    check_combined_scenarios(driver, artifacts, "encrypted-restore")
    assert_saved_combined_restored(driver, artifacts, "encrypted-restore")
    assert_counselor_evidence_restored(driver, artifacts, "encrypted-restore")
    driver.open_client("Alex Fictional")
    assert_alex_dashboard(driver)
    driver.open_client("Alex Fictional")
    status("encrypted backup, wrong-password rejection, interrupted review, explicit restore, retained source, restored plans and recurring decisions")
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
    assert_recurring_restored(driver)
    check_client_care(driver, artifacts, "error-screen-restore")
    check_document_collection(driver, artifacts, "error-screen-restore")
    assert_statement_balances_restored(driver, artifacts, "error-screen-restore")
    assert_captured_planning_restored(driver, artifacts, "error-screen-restore")
    check_budget_methods(driver, artifacts, "error-screen-restore")
    check_combined_scenarios(driver, artifacts, "error-screen-restore")
    assert_saved_combined_restored(driver, artifacts, "error-screen-restore")
    assert_counselor_evidence_restored(driver, artifacts, "error-screen-restore")
    driver.open_client("Alex Fictional")
    assert_alex_dashboard(driver)
    status("fresh-process activation and error-screen recovery after simulated metadata loss, including recurring decisions")
    check_firm_branding(driver, artifacts, data_root, password, binary, restart_driver)
    check_completeness(driver, artifacts)
    check_saved_coverage_reviews(driver, artifacts, data_root, password, binary, restart_driver)
    check_final_report_approval(driver, artifacts, data_root, password, binary, restart_driver)
    check_card_statements(driver, artifacts, data_root, password, binary, restart_driver)
    check_integrated_card_review(driver, artifacts, data_root, password, binary, restart_driver)
    check_card_completeness(driver, artifacts, data_root, password, binary, restart_driver)
    check_card_final_approval(driver, artifacts, data_root, password, binary, restart_driver)


def check_card_final_approval(driver: WebDriver, artifacts: Path, data_root: Path,
                              password: str, binary: Path,
                              restart_driver: Callable[[], None]) -> None:
    title='Fictional approved bank/card assessment'
    def financial(area: str, select_client: bool = False) -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Financial Review']")
        if select_client:driver.select('#workspace-client','Jamie Complete')
        driver.interactive('#workspace-client')
        # Client selection mounts the current section and loads its data before
        # navigation is permitted. Wait for its actual controls, not just the
        # always-visible tab button; never replay a possibly ignored click.
        current=driver.observe_script("return document.querySelector('.section-navigation button[aria-current=page]')?.textContent")
        control={'Working tools':'#assessment-from','Card completeness':'#card-facts-from','Bank & card activity':'#card-link-action'}.get(current)
        if control:driver.interactive(control)
        driver.click_button(area)
        driver.wait(lambda:driver.observe_script("return document.querySelector('.section-navigation button[aria-current=page]')?.textContent") == area,'selected financial review section')
    financial('Working tools',True)
    driver.click('.financial-facts-editor > summary');driver.interactive('#facts-as-of')
    index=driver.observe_script("return Array.from(document.querySelectorAll('input[id^=fact-balance-label-]')).findIndex(e=>e.value==='Fictional mapped card debt')")
    if not isinstance(index,int) or index<0:raise SmokeFailure('Original card debt missing from household facts')
    driver.fill(f'#fact-balance-amount-{index}','1037.05');driver.fill(f'#fact-balance-payment-{index}','50.03')
    if not driver.observe_script("return document.querySelector('#facts-inventory').checked"):driver.click('#facts-inventory')
    save_financial_facts(driver,'Reviewed original fictional closing debt and known required monthly payment; inventory recorded once.')
    financial('Bank & card activity');driver.interactive('#card-link-action');driver.select('#card-link-action','Match repayment')
    source=driver.observe_script("return Array.from(document.querySelector('#card-link-statement').options).find(o=>o.textContent.startsWith('card-review.csv'))?.textContent")
    if not source:raise SmokeFailure('Missing original card source for fresh repayment review')
    driver.select('#card-link-statement',source)
    row=driver.observe_script("return Array.from(document.querySelector('#card-link-row').options).find(o=>o.textContent.startsWith('4: Fictional card payment'))?.textContent")
    bank=driver.observe_script("return Array.from(document.querySelector('#card-link-bank').options).find(o=>o.textContent.includes('Fictional matched card repayment'))?.textContent")
    if not row or not bank:raise SmokeFailure('Current repayment evidence missing')
    driver.select('#card-link-row',row);driver.select('#card-link-bank',bank)
    driver.fill('#card-link-reason','Explicit renewed review of current approvals, equal USD 50.03 and in-period dates; no split or duplicate.')
    driver.click('#card-link-ack');driver.click_button('Save reviewed card change')
    driver.wait_value('#card-link-reason','','renewed repayment review saved')
    driver.wait_text('.card-repayment-matches','Current approval bindings','stale evidence renewed only by explicit review')
    financial('Card completeness');driver.interactive('#card-facts-from');driver.click_button('Review latest card facts context (keep draft)')
    driver.wait_text('.card-completeness > .success-message','Latest context loaded','fresh exact facts and repayment context')
    driver.click('#card-facts-inventory-ack');driver.click('#card-facts-ack');driver.click_button('Save card completeness evidence')
    driver.wait_text('.card-completeness > .success-message','version 4','fresh complete card evidence saved')
    financial('Completeness & final validation');driver.click('.saved-coverage-reviews > summary');driver.interactive('#coverage-review-from')
    driver.click_button('Review latest context (keep draft)');driver.wait_text('.saved-coverage-reviews','Latest saved context loaded','bank account inventory refreshed explicitly')
    count=driver.observe_script("return document.querySelectorAll('input[id^=coverage-label-]').length")
    if count!=2:raise SmokeFailure('Complete card final example requires both known bank accounts')
    for i in range(count):
        driver.fill(f'#coverage-start-{i}','2026-09-01');driver.fill(f'#coverage-end-{i}','2026-09-30')
    sources=driver.observe_script("return Array.from(document.querySelectorAll('.saved-coverage-reviews input[aria-label^=\"Use \"]')).filter(e=>!e.checked).map(e=>e.getAttribute('aria-label'))")
    for label in sources:driver.click(f'input[aria-label={json.dumps(label)}]')
    driver.fill('#coverage-inventory-notes','Both fictional checking accounts and separately reviewed card inventory inspected; no other household assets, debts or obligations omitted.')
    driver.fill('#coverage-conclusion','Original retained reconciled sources, full expected bank periods and card evidence reviewed for separate final approval.')
    driver.click('#coverage-inventory-reviewed');driver.click_button('Save completeness review')
    driver.wait_text('.saved-coverage-reviews','version 3','current two-account completeness saved')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']");driver.click('.saved-assessments > summary');driver.interactive('#record-title')
    for selector,value in [('#record-from','2026-09'),('#record-through','2026-09'),('#record-title',title)]:driver.fill(selector,value)
    driver.click('#record-include-completeness');driver.click_button('Review saved completeness context');driver.wait_text('.coverage-review-selection','Both fictional checking accounts','matching bank inventory loaded')
    driver.click('#record-include-cards');driver.click_button('Review bank/card supplement');driver.wait_text('.bank-card-totals','Provisional combined observed spendingUSD 1287.10','unchanged original bank/card subtotal')
    driver.click('#record-include-card-facts');driver.click_button('Review saved card completeness');driver.wait_text('.captured-card-completeness','Card completeness review version 4','current card evidence loaded')
    driver.click('#record-ack');driver.click_button('Save assessment record');driver.wait_text('.saved-assessments > .success-message','Assessment record saved','complete format-nine candidate saved')
    driver.wait(lambda:driver.observe_script("const s=document.querySelector('#saved-assessment');return s?.value&&s.selectedOptions[0]?.textContent.startsWith('Fictional approved bank/card assessment')?s.value:false;"),'new final parent selected after directory refresh')
    def open_approval() -> None:
        driver.interactive('#workspace-client')
        driver.interactive('.saved-assessments > summary')
        if not driver.observe_script("return document.querySelector('.saved-assessments').open"):driver.click('.saved-assessments > summary')
        driver.interactive('#saved-assessment')
        driver.click_button('Final report checklist');driver.interactive('#checklist-report')
        choice=driver.observe_script("return Array.from(document.querySelector('#checklist-report').options).find(o=>o.textContent.startsWith('Fictional approved bank/card assessment'))?.textContent")
        if not choice:raise SmokeFailure('Card final parent is missing')
        driver.select('#checklist-report',choice);driver.click_button('Review final-report readiness')
    open_approval();driver.wait_text('.final-readiness-status','Required checks passed','complete bank/card evidence passes v2 gates')
    for expected in ['Combined recorded spendingUSD 1287.10','Income less combined spendingUSD 712.91','Bank cash movementUSD 749.96','Existing household debt (counted once)USD 1037.05']:
        driver.wait_text('.card-final-figures',expected,'independent official combined figures')
    accounts=driver.observe_script("return Array.from(document.querySelectorAll('textarea[id^=final-account-]')).map(e=>e.id)")
    if len(accounts)!=3:raise SmokeFailure('Final approval omitted bank/card scope decisions')
    for account in accounts:driver.fill('#'+account,'Original full-period account/card evidence and applicability inspected.')
    driver.fill('#final-planning-notes','Historical bank/card assessment only; future scenario recommendations are not applicable or captured.')
    driver.fill('#final-professional-notes','Fictional counselor inspected original sources, bank/card completeness, duplicates, exclusions, exact figures and original household inventory.')
    notes='Reviewed fictional card originals, necessary/discretionary roles, current matched repayments, required payment and debt mapping without counting debt twice.'
    driver.fill('#final-card-notes',notes)
    for key in ['confirmClient','confirmSources','confirmInventory','confirmAssumptions']:driver.click(f'#final-{key}')
    driver.click_button('Approve final counselor report')
    driver.wait_text('.final-report-approval > [role=alert]','Confirm the reviewed card sources','separate card confirmation cannot be omitted')
    driver.wait_value('#final-card-notes',notes,'failed approval retains card review notes')
    driver.click('#final-confirm-cards');driver.click_button('Approve final counselor report')
    driver.wait_text('.approved-final-report','counselor-final-report-v2','explicit v2 approval preserved')
    driver.wait_text('.final-report-current-status','supporting context matches','new combined approval has current context')
    identity=driver.text('.approved-final-report .field-help');figures=driver.text('.approved-final-report .card-final-figures')
    def export(label: str) -> None:
        destination=artifacts / f'approved-card-{label}.pdf'
        driver.fill_pdf_destination('#final-pdf-path',destination);driver.click_button('Export approved final report PDF')
        driver.wait_text('.final-report-approval > .success-message',str(destination),'card final PDF exported')
        text=' '.join(subprocess.run(['pdftotext','-layout',str(destination),'-'],check=True,capture_output=True,text=True).stdout.split())
        for expected in ['FINAL COUNSELOR REPORT','counselor-final-report-v2','Combined recorded spending: USD 1287.10','Income less combined spending: USD 712.91','Bank cash movement: USD 749.96','Existing household debt / counted once: USD 1037.05',notes]:
            if expected not in text:raise SmokeFailure(f'Final card PDF omitted {expected}')
        if label!='current' and 'CURRENT CONTEXT NEEDS REVIEW' not in text:raise SmokeFailure('Historical card PDF omitted review flag')
    export('current');driver.scroll_to('.approved-final-report','start');driver.screenshot(artifacts / 'native-approved-card-final.png')
    financial('Card completeness');driver.interactive('#card-facts-conclusion');driver.fill('#card-facts-conclusion','Later fictional card follow-up reviewed; original final approval remains historical.')
    driver.click('#card-facts-ack');driver.click_button('Save card completeness evidence');driver.wait_text('.card-completeness > .success-message','version 5','later review is appended')
    restart_driver();driver.open(binary);driver.fill('#password',password);driver.click_button('Unlock workspace')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']");driver.select('#workspace-client','Jamie Complete');open_approval()
    driver.wait_text('.final-report-current-status','Historical approval','later card review flags preserved original approval')
    if driver.text('.approved-final-report .field-help')!=identity or driver.text('.approved-final-report .card-final-figures')!=figures:raise SmokeFailure('Later review or restart rewrote approved identity/figures')
    export('historical')
    backup_folder=data_root / 'card-final-backups';backup_folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']");driver.click_button('Backup & recovery');driver.fill('#backup-folder',str(backup_folder));driver.fill('#backup-password',password);driver.click_button('Create encrypted backup');driver.wait_text('.backup-success','Encrypted backup saved.','card final encrypted backup')
    backup_path=Path(driver.text('.backup-file-path'));driver.click_button('Lock workspace');driver.click('.restore-panel > summary');driver.fill('#restore-file',str(backup_path));driver.fill('#restore-password',password);driver.click_button('Verify backup for recovery');driver.wait_text('.restore-review','Fictional Oak Counseling','v1/v2 approval and exact parent evidence verified');driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace');driver.wait_text('.auth-card','Workspace restored. Unlock it','card final recovery activated');driver.fill('#password',password);driver.click_button('Unlock workspace')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']");driver.select('#workspace-client','Jamie Complete');open_approval();driver.wait_text('.final-report-current-status','Historical approval','original combined approval restored')
    if driver.text('.approved-final-report .field-help')!=identity or driver.text('.approved-final-report .card-final-figures')!=figures:raise SmokeFailure('Encrypted recovery rewrote original card approval')
    export('restored');driver.scroll_to('.approved-final-report','start');driver.screenshot(artifacts / 'native-card-final-restored.png')
    driver.select('#workspace-client','Alex Fictional');driver.interactive('#checklist-report')
    if title in driver.text('#checklist-report') or notes in driver.text('body'):raise SmokeFailure('Card approval or private notes crossed client context')
    status('card-inclusive v2 exact spending/cash/debt figures, explicit renewed repayment evidence and confirmation, immutable approval/PDF, later-context flags, restart, v1/v2 encrypted recovery and client isolation')


def check_card_completeness(driver: WebDriver, artifacts: Path, data_root: Path,
                            password: str, binary: Path,
                            restart_driver: Callable[[], None]) -> None:
    def facts_page(select_client: bool = False) -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Financial Review']")
        if select_client: driver.select('#workspace-client','Jamie Complete')
        driver.click_button('Working tools')
        driver.click('.financial-facts-editor > summary')
        driver.interactive('#facts-as-of')
    def card_page(select_client: bool = False) -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Financial Review']")
        if select_client: driver.select('#workspace-client','Jamie Complete')
        driver.click_button('Card completeness')
        driver.interactive('#card-facts-from')
    def saved_reference() -> None:
        if not driver.observe_script("return document.querySelector('.card-facts-saved-reference')?.open"):
            driver.click('.card-facts-saved-reference > summary')
    def save_review(number: int) -> None:
        driver.click('#card-facts-ack')
        driver.click_button('Save card completeness evidence')
        driver.wait_text('.card-completeness > .success-message',f'Card completeness saved as version {number}.','reviewed card evidence persisted')
        saved_reference()
    facts_page(True)
    count=driver.observe_script("return document.querySelectorAll('.facts-balance-row').length")
    if not isinstance(count,int) or count<1: raise SmokeFailure('Original verified bank balances missing')
    driver.fill('#facts-as-of','2026-09-30')
    for index,label,kind,amount in [(count,'Fictional mapped card debt','Debt owed','1037.05'),(count+1,'Fictional repayment checking','Cash / checking','49.97')]:
        driver.click_button('Add verified balance')
        driver.fill(f'#fact-balance-label-{index}',label)
        driver.select(f'#fact-balance-kind-{index}',kind)
        driver.fill(f'#fact-balance-amount-{index}',amount)
        driver.fill(f'#fact-balance-evidence-{index}','Reviewed fictional original statement at September 30; card debt recorded once in household inventory.')
    driver.click('#facts-inventory')
    save_financial_facts(driver,'Explicit fictional card debt and second checking balance added to existing full inventory.')
    view_financial_dashboard(driver,'2026-09','2026-09')
    driver.wait_text('.verified-balance-totals','Recorded outstanding debt$1,037.05','existing household debt before linking card evidence')
    card_page()
    for selector,value in [('#card-facts-from','2026-09'),('#card-facts-through','2026-09'),('#card-facts-inventory','Reviewed the fictional household card inventory including its archived durable card.'),('#card-facts-missing','Fictional role and closing-balance review is incomplete.'),('#card-facts-conclusion','Internal card evidence review; final approval and other household evidence remain separate.')]:driver.fill(selector,value)
    driver.click('#card-facts-inventory-ack')
    driver.select('#card-facts-applicable-0','Applicable')
    save_review(1)
    driver.wait_text('.captured-card-completeness','4 unresolved rows','missing source-bound roles are counted')
    driver.wait_text('.card-debt-comparisons','Difference Unknown','unmapped debt is not verified zero')
    for selector,value in [('#card-facts-start-0','2026-09-01'),('#card-facts-end-0','2026-09-30'),('#card-facts-notes-0','Archived workflow account was applicable throughout this historical month.'),('#card-facts-missing','')]:driver.fill(selector,value)
    driver.click("input[aria-label^='Review card-review.csv for ']")
    closing=driver.observe_script("return Array.from(document.querySelector('#card-facts-closing-0').options).find(o=>o.textContent.startsWith('card-review.csv'))?.textContent")
    debt=driver.observe_script("return Array.from(document.querySelector('#card-facts-balance-0').options).find(o=>o.textContent.startsWith('Fictional mapped card debt'))?.textContent")
    if not closing or not debt:raise SmokeFailure('Missing explicit card-source or verified debt choice')
    driver.select('#card-facts-closing-0',closing);driver.select('#card-facts-balance-0',debt)
    roles=driver.observe_script("return Array.from(document.querySelectorAll('select[id^=card-role-]')).map(e=>({id:e.id,label:e.labels[0].textContent}))")
    if len(roles)!=4:raise SmokeFailure('Card payments became eligible spending roles or approved charges disappeared')
    for role in roles:driver.select('#'+role['id'],'Discretionary' if 'row 2' in role['label'] else 'Necessary')
    save_review(2)
    driver.wait_text('.captured-card-completeness','Necessary card spendingUSD -12.93','independent refund/fee/interest role total')
    driver.wait_text('.captured-card-completeness','Discretionary card spendingUSD 100.01','independent purchase role total')
    driver.wait_text('.card-debt-comparisons','Existing verified debt USD 1037.05 · Difference USD 0.00','existing debt is compared without adding it again')
    driver.wait_text('.captured-card-completeness','Recorded card evidence checks satisfied','recorded evidence passes its bounded checks')
    driver.scroll_to('.captured-card-completeness','start');driver.screenshot(artifacts / 'native-card-completeness.png')
    # Mapping a card cannot change the original household balance facts.
    driver.click_button('Working tools')
    view_financial_dashboard(driver,'2026-09','2026-09')
    driver.wait_text('.verified-balance-totals','Recorded outstanding debt$1,037.05','card mapping does not duplicate household debt')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.click('.saved-assessments > summary');driver.interactive('#record-title')
    for selector,value in [('#record-from','2026-09'),('#record-through','2026-09'),('#record-title','Fictional card completeness assessment')]:driver.fill(selector,value)
    driver.click('#record-include-cards');driver.click_button('Review bank/card supplement')
    driver.wait_text('.bank-card-totals','Card net spendingUSD 87.08','reviewed unchanged bank/card supplement')
    driver.click('#record-include-card-facts');driver.click_button('Review saved card completeness')
    driver.wait_text('.captured-card-completeness','Card completeness review version 2','reviewed exact saved card completeness version')
    driver.click('#record-ack');driver.click_button('Save assessment record')
    driver.wait_text('.saved-assessments > .success-message','Assessment record saved','format-nine historical evidence recorded')
    driver.wait_text('.saved-assessment-view .captured-card-completeness','Discretionary card spendingUSD 100.01','saved classified card amounts')
    original=driver.text('.saved-assessment-view .captured-card-completeness')
    destination=artifacts / 'native-card-completeness-assessment.pdf'
    driver.fill_pdf_destination('#assessment-pdf-path',destination);driver.click_button('Export saved assessment PDF')
    driver.wait_text('.saved-assessments > .success-message',str(destination),'native card completeness PDF exported')
    text=' '.join(subprocess.run(['pdftotext','-layout',str(destination),'-'],check=True,capture_output=True,text=True).stdout.split())
    for expected in ['Captured card completeness evidence','-12.93','100.01','1037.05','never added']:
        if expected not in text:raise SmokeFailure(f'Card completeness PDF omitted {expected}')
    facts_page()
    driver.fill(f'#fact-balance-amount-{count}','1037.06')
    save_financial_facts(driver,'Synthetic one-cent verified debt correction; historical card evidence must remain unchanged.')
    card_page()
    driver.wait_text('.card-completeness','original review is preserved','later verified balance flags old review')
    driver.click_button('Review latest card facts context (keep draft)')
    driver.wait_text('.card-completeness > .success-message','Latest context loaded','explicit current facts refresh')
    driver.click('#card-facts-inventory-ack')
    save_review(3)
    driver.wait_text('.card-debt-comparisons','Difference USD -0.01','one-cent debt discrepancy remains unresolved')
    driver.wait_text('.captured-card-completeness','differs from its mapped verified debt balance','difference blocks completeness findings')
    driver.select('#card-facts-version','Version 2 — 2026-09 through 2026-09')
    driver.wait_text('.captured-card-completeness','Difference USD 0.00','original mapped amount remains in immutable earlier version')
    if driver.visible('#card-facts-ack') is not None:raise SmokeFailure('Historical card review remained editable')
    restart_driver();driver.open(binary);driver.fill('#password',password);driver.click_button('Unlock workspace')
    card_page(True);saved_reference()
    driver.wait_text('.card-debt-comparisons','Difference USD -0.01','latest debt review persists across process restart')
    backup_folder=data_root / 'card-completeness-backups';backup_folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']");driver.click_button('Backup & recovery')
    driver.fill('#backup-folder',str(backup_folder));driver.fill('#backup-password',password);driver.click_button('Create encrypted backup')
    driver.wait_text('.backup-success','Encrypted backup saved.','card completeness encrypted backup')
    backup_path=Path(driver.text('.backup-file-path'))
    driver.click_button('Lock workspace');driver.click('.restore-panel > summary');driver.fill('#restore-file',str(backup_path));driver.fill('#restore-password',password)
    driver.click_button('Verify backup for recovery');driver.wait_text('.restore-review','Fictional Oak Counseling','card completeness source/history/report proofs verified')
    driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace');driver.wait_text('.auth-card','Workspace restored. Unlock it','card completeness recovery activated')
    driver.fill('#password',password);driver.click_button('Unlock workspace')
    card_page(True);saved_reference();driver.wait_text('.card-debt-comparisons','Difference USD -0.01','latest card evidence restored')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']");driver.click('.saved-assessments > summary');driver.interactive('#saved-assessment')
    choice=driver.observe_script("return Array.from(document.querySelector('#saved-assessment').options).find(o=>o.textContent.startsWith('Fictional card completeness assessment'))?.textContent")
    if not choice:raise SmokeFailure('Historical card completeness report missing after restore')
    driver.select('#saved-assessment',choice);driver.wait_text('.saved-assessment-view .captured-card-completeness','Difference USD 0.00','historical report retains original debt mapping after recovery')
    if driver.text('.saved-assessment-view .captured-card-completeness')!=original:raise SmokeFailure('Later balances or recovery rewrote original card completeness report')
    driver.scroll_to('.saved-assessment-view .captured-card-completeness','start');driver.screenshot(artifacts / 'native-card-completeness-historical.png')
    driver.click_button('Final report checklist');driver.interactive('#checklist-report')
    choice=driver.observe_script("return Array.from(document.querySelector('#checklist-report').options).find(o=>o.textContent.startsWith('Fictional card completeness assessment'))?.textContent")
    if not choice:raise SmokeFailure('Captured card report missing from approval checklist')
    driver.select('#checklist-report',choice);driver.click_button('Review final-report readiness');driver.wait_text('.final-readiness-status','Final approval blocked','card evidence does not bypass final policy')
    card_page();driver.select('#workspace-client','Alex Fictional');driver.wait_text('#card-facts-version','Current editable review (0)','card completeness client isolation')
    driver.wait_value('#card-facts-inventory','','private inventory notes cleared on client change')
    if 'Fictional mapped card debt' in driver.text('.card-completeness'):raise SmokeFailure('Private card debt evidence crossed clients')
    status('card inventory/applicability, exact source-bound roles, no duplicate debt, one-cent discrepancy, read-only history, format-nine PDF/report, blocked final approval, restart, encrypted recovery and isolation')


def check_integrated_card_review(driver: WebDriver, artifacts: Path, data_root: Path,
                                 password: str, binary: Path,
                                 restart_driver: Callable[[], None]) -> None:
    def card_page() -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Documents']")
        driver.select('#workspace-client','Jamie Complete')
        driver.click_button('Credit-card statements')
        driver.interactive("//button[normalize-space(.)='Review card row 2']",'xpath')
    def review_page() -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Financial Review']")
        driver.click_button('Bank & card activity')
        driver.interactive('#card-link-action')
    def confirm_change(reason: str) -> None:
        driver.fill('#card-link-reason',reason)
        driver.click('#card-link-ack')
        driver.click_button('Save reviewed card change')
        driver.wait_text('.card-financial-review > .success-message','Card review change saved','reviewed integration change persisted')
        driver.wait_value('#card-link-reason','','integration draft cleared')
    def choose_source() -> None:
        driver.interactive('#card-link-statement')
        text=driver.observe_script("return Array.from(document.querySelector('#card-link-statement').options).find(o=>o.textContent.startsWith('card-review.csv'))?.textContent")
        if not text: raise SmokeFailure('Card source unavailable for explicit assignment')
        driver.select('#card-link-statement',text)
    card_page()
    # Explicitly restore the deliberately rejected test purchase to its verified source amount.
    driver.click_button('Review card row 2')
    driver.fill('#card-row-amount','100.01')
    driver.select('#card-row-status','Approved')
    driver.fill('#card-row-reason','Verified original fictional purchase for integrated spending example; prior corrections remain historical.')
    driver.click('#card-row-ack')
    driver.click_button('Save card transaction decision')
    driver.wait_text('.card-reconciliation','Available statement arithmetic reconciles','card source reconciled before integration')
    source=data_root / 'bank-card-repayment.csv'
    source.write_text('Date,Description,Amount,Category,Type\n2026-09-04,Fictional matched card repayment,-50.03,Card repayment,balanceMovement\n',encoding='utf-8')
    driver.click_button('Working tools')
    driver.fill('#csv-path',str(source))
    driver.click_button('Inspect CSV')
    driver.wait_text('.csv-preview',source.name,'synthetic repayment source')
    driver.fill('#csv-account-label','Card repayment checking')
    for key,heading in [('date','1: Date'),('description','2: Description'),('amount','3: Amount'),('category','4: Category'),('kind','5: Type')]:
        driver.select(f'#csv-map-{key}',heading)
    for key,value in [('periodStart','2026-09-01'),('periodEnd','2026-09-30'),('openingBalance','100.00'),('closingBalance','49.97')]:
        driver.fill(f'#csv-{key}',value)
    driver.click("//label[contains(.,'Authorize retaining the complete original CSV')]/input",'xpath')
    driver.click_button('Import into review queue')
    driver.wait_text('.imports-panel > .success-message','CSV imported','repayment source persisted without approval')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Transactions']")
    driver.wait_text('.imports-progress','Pending 1','repayment remains pending until review')
    driver.click_button('Review bank-card-repayment.csv record 2')
    driver.select('#review-status','Approved')
    driver.fill('#review-reason','Verified fictional card repayment is a balance movement, not another expense or income.')
    driver.click('#review-ack')
    driver.click_button('Save transaction decision')
    driver.wait_text('.imports-progress','Approved 3','repayment approval saved')
    review_page()
    driver.fill('#card-link-label','Fictional durable household card')
    confirm_change('Verified fictional account identity; display names alone do not match sources.')
    driver.select('#card-link-action','Assign or unassign card statement')
    choose_source()
    driver.select('#card-link-account','Fictional durable household card')
    confirm_change('Assigned the inspected original card statement to its durable account.')
    for selector in ['#card-review-from','#card-review-through']: driver.fill(selector,'2026-09')
    driver.click_button('Calculate bank and card review')
    driver.wait_text('.bank-card-totals','unmatched','unmatched repayment blocks combined spending')
    driver.wait_text('.bank-card-totals','Unavailable','unknown combined spending is not zero')
    driver.select('#card-link-action','Match repayment')
    choose_source()
    driver.interactive('#card-link-row')
    row=driver.observe_script("return Array.from(document.querySelector('#card-link-row').options).find(o=>o.textContent.startsWith('4: Fictional card payment'))?.textContent")
    bank=driver.observe_script("return Array.from(document.querySelector('#card-link-bank').options).find(o=>o.textContent.includes('Fictional matched card repayment'))?.textContent")
    if not row or not bank: raise SmokeFailure('Missing approved repayment choices')
    driver.select('#card-link-row',row)
    driver.select('#card-link-bank',bank)
    confirm_change('Verified equal USD 50.03 negative amounts within seven days, with no split payment or duplicate link.')
    driver.click_button('Calculate bank and card review')
    for expected in ['Bank cash movementUSD 749.96','Card net spendingUSD 87.08','Provisional combined observed spendingUSD 1287.10','Card payments (excluded from spending)USD 50.03','Matched in-period repaymentsUSD 50.03']:
        driver.wait_text('.bank-card-totals',expected,'independent bank/card cash versus spending example')
    driver.scroll_to('.bank-card-totals','start')
    driver.screenshot(artifacts / 'native-bank-card-review.png')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.click('.saved-assessments > summary')
    driver.interactive('#record-title')
    for selector,value in [('#record-from','2026-09'),('#record-through','2026-09'),('#record-title','Fictional integrated bank/card assessment')]:driver.fill(selector,value)
    driver.click('#record-include-cards')
    driver.click_button('Review bank/card supplement')
    driver.wait_text('.bank-card-totals','Provisional combined observed spendingUSD 1287.10','explicit report card review')
    driver.click('#record-ack')
    driver.click_button('Save assessment record')
    driver.wait_text('.saved-assessments > .success-message','Assessment record saved','format-eight report persisted')
    driver.wait_text('.captured-card-review','Provisional combined observed spendingUSD 1287.10','captured reviewed card figures')
    original=driver.text('.captured-card-review')
    driver.click('.captured-card-review details > summary')
    driver.wait_text('.captured-card-review','Fictional durable household card','captured durable card identity')
    destination=artifacts / 'native-bank-card-assessment.pdf'
    driver.fill_pdf_destination('#assessment-pdf-path',destination)
    driver.click_button('Export saved assessment PDF')
    driver.wait(lambda:destination.is_file(),'integrated internal PDF')
    text=' '.join(subprocess.run(['pdftotext','-layout',str(destination),'-'],check=True,capture_output=True,text=True).stdout.split())
    for expected in ['Captured bank/card review supplement','1287.10','749.96','50.03','Card liability change']:
        if expected not in text: raise SmokeFailure(f'Integrated native PDF omitted {expected}')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Transactions']")
    driver.click_button('Review bank-card-repayment.csv record 2')
    driver.fill('#review-reason','Fresh explicit bank approval must invalidate the original repayment binding even with unchanged amount.')
    driver.click('#review-ack')
    driver.click_button('Save transaction decision')
    driver.wait_text('.imports-progress','Approved 3','fresh bank approval saved')
    review_page()
    driver.wait_text('.card-repayment-matches','Needs review — approval changed','immutable repayment evidence detects newer approval')
    for selector in ['#card-review-from','#card-review-through']:driver.fill(selector,'2026-09')
    driver.click_button('Calculate bank and card review')
    driver.wait_text('.bank-card-totals','Unavailable','stale repayment does not silently revive')
    driver.select('#card-link-action','Rename, archive or reopen card account')
    driver.select('#card-link-account','Fictional durable household card')
    driver.fill('#card-link-label','Renamed fictional household card')
    driver.click('#card-link-archived')
    confirm_change('Archived and renamed the workflow card; retained its durable identity, sources and stale repayment evidence.')
    restart_driver()
    driver.open(binary)
    driver.fill('#password',password)
    driver.click_button('Unlock workspace')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Financial Review']")
    driver.select('#workspace-client','Jamie Complete')
    driver.click_button('Bank & card activity')
    driver.wait_text('.card-financial-review','Renamed fictional household card — Archived','card identity persists across process restart')
    driver.wait_text('.card-repayment-matches','Needs review — approval changed','stale matches persist across restart')
    # Verify the full new schema and original format-eight report before activating encrypted recovery.
    backup_folder=data_root / 'integrated-card-backups'
    backup_folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']")
    driver.click_button('Backup & recovery')
    driver.fill('#backup-folder',str(backup_folder));driver.fill('#backup-password',password)
    driver.click_button('Create encrypted backup')
    driver.wait_text('.backup-success','Encrypted backup saved.','integrated encrypted backup')
    backup_path=Path(driver.text('.backup-file-path'))
    driver.click_button('Lock workspace');driver.click('.restore-panel > summary')
    driver.fill('#restore-file',str(backup_path));driver.fill('#restore-password',password)
    driver.click_button('Verify backup for recovery')
    driver.wait_text('.restore-review','Fictional Oak Counseling','historical card sources, links and report validated')
    driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace')
    driver.wait_text('.auth-card','Workspace restored. Unlock it','integrated backup activated')
    driver.fill('#password',password);driver.click_button('Unlock workspace')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.select('#workspace-client','Jamie Complete')
    driver.click('.saved-assessments > summary')
    driver.interactive('#saved-assessment')
    choice=driver.observe_script("return Array.from(document.querySelector('#saved-assessment').options).find(o=>o.textContent.startsWith('Fictional integrated bank/card assessment'))?.textContent")
    if not choice: raise SmokeFailure('Captured integrated report missing after recovery')
    driver.select('#saved-assessment',choice)
    driver.wait_text('.captured-card-review','Provisional combined observed spendingUSD 1287.10','original report retains figures after later context and recovery')
    if driver.text('.captured-card-review') != original: raise SmokeFailure('Later changes or recovery rewrote original integrated report')
    driver.scroll_to('.captured-card-review','start');driver.screenshot(artifacts / 'native-bank-card-historical.png')
    review_page()
    driver.wait_text('.card-financial-review','Renamed fictional household card — Archived','recovered current account history')
    driver.select('#workspace-client','Alex Fictional')
    driver.wait_text('.card-financial-review','No durable card accounts recorded','card account and repayment client isolation')
    if 'Renamed fictional household card' in driver.text('.card-financial-review'):raise SmokeFailure('Card identity crossed clients')
    status('durable card identity, explicit source/repayment links, independent cash versus spending totals, format-eight report/PDF, permanent stale binding, restart, encrypted recovery and isolation')


def check_card_statements(driver: WebDriver, artifacts: Path, data_root: Path,
                          password: str, binary: Path,
                          restart_driver: Callable[[], None]) -> None:
    source=data_root / 'card-review.csv'
    source.write_text('Date,Description,Amount,Category\n2026-09-01,Fictional card groceries,100.01,Food\n2026-09-02,Fictional card refund,-20.02,Food\n2026-09-03,Fictional card payment,-50.03,Card payment\n2026-09-04,Fictional card fee,5.04,Fees\n2026-09-05,Fictional card interest,2.05,Interest\n',encoding='utf-8')
    def page(select_client: bool = False) -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Documents']")
        if select_client:
            driver.select('#workspace-client','Jamie Complete')
        driver.click_button('Credit-card statements')
        driver.wait_text('.card-statements','Credit-card statements','working card review area')
        driver.interactive("//button[normalize-space(.)='Review latest card data (keep draft)']",'xpath')
    page()
    driver.wait_text('.card-statements','No card statements recorded','empty card ledger')
    driver.click('.card-import > summary')
    driver.fill('#card-csv-path',str(source))
    driver.click_button('Inspect card CSV')
    driver.wait_text('.card-preview',source.name,'card source sample')
    for key,heading in [('date','1: Date'),('description','2: Description'),('amount','3: Amount'),('category','4: Category')]:
        driver.select(f'#card-map-{key}',heading)
    for selector,value in [('#card-account','Fictional household card'),('#card-period-start','2026-09-01'),('#card-period-end','2026-09-30'),('#card-opening','1000.00'),('#card-closing','1037.05')]:
        driver.fill(selector,value)
    driver.click('#card-retain')
    driver.click_button('Save card statement for individual review')
    driver.wait_text('.card-progress','Pending 5','no automatic card approvals')
    driver.wait_text('.card-reconciliation','Original source difference USD 0.00','exact whole-source liability reconciliation')
    driver.wait_text('.card-reconciliation','Approved closing Unknown','undecided card amounts never become zero')
    def decide(number: int, kind: str, state: str, reason: str, amount: str | None = None) -> None:
        driver.click_button(f'Review card row {number}')
        driver.select('#card-row-kind',kind)
        driver.select('#card-row-status',state)
        if amount is not None:
            driver.fill('#card-row-amount',amount)
        driver.fill('#card-row-reason',reason)
        driver.click('#card-row-ack')
        driver.click_button('Save card transaction decision')
        driver.wait_text('.card-statements > .success-message','Card transaction decision saved','individual source-bound card decision')
        driver.wait(lambda: driver.visible('.card-transaction-review') is None,'card decision draft cleared after save')
    for number,kind in enumerate(['purchase','refund','payment','fee','interest'],2):
        decide(number,kind,'Approved','Inspected fictional original card row, correct debt sign and potential duplicates.')
    for expected in ['Net card spendingUSD 87.08','Approved card payments (not spending)USD 50.03','Approved change in debt owedUSD 37.05']:
        driver.wait_text('.card-totals',expected,'independently checked card spending and payments')
    driver.wait_text('.card-reconciliation','Approved closing USD 1037.05','independent debt closing amount')
    driver.wait_text('.card-reconciliation','Available statement arithmetic reconciles','approved and source arithmetic remain separate')
    driver.scroll_to('.card-statement-view','start')
    driver.screenshot(artifacts / 'native-card-reconciled.png')
    decide(2,'purchase','Approved','Verified one-cent fictional correction; retain original source.','100.02')
    driver.wait_text('.card-reconciliation','Original source difference USD 0.00','correction preserves original arithmetic')
    driver.wait_text('.card-reconciliation','Approved difference USD 0.01','one-cent card correction disclosed')
    decide(2,'purchase','Rejected','Withdraw corrected fictional charge; keep both earlier decisions.')
    driver.wait_text('.card-totals','Net card spendingUSD -12.93','rejected purchase excluded without treating refund as income')
    driver.click_button('Review card row 2')
    driver.click('.card-review-history > summary')
    driver.wait_text('.card-review-history','USD 100.01','original card approval history')
    driver.wait_text('.card-review-history','USD 100.02','correction and rejection history')
    driver.scroll_to('.card-transaction-review','start')
    driver.screenshot(artifacts / 'native-card-correction-history.png')
    driver.click_button('Discard card decision draft')
    identity=driver.text('.card-statement-view .field-help')
    # Existing bank amounts remain exactly the original cash-based figures.
    period_end=dt.datetime.now(ZoneInfo('America/Chicago')).date().replace(day=1)-dt.timedelta(days=1)
    month=period_end.strftime('%Y-%m')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Financial Review']")
    view_financial_dashboard(driver,month,month)
    driver.wait_text('.assessment-totals','Operating cash flow$799.99','card activity does not silently alter bank dashboard totals')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.click_button('Final report checklist')
    driver.interactive('#checklist-report')
    choice=driver.observe_script("return Array.from(document.querySelector('#checklist-report').options).find(o=>o.textContent.startsWith('Fictional final-ready completed-period assessment'))?.textContent")
    if not choice:
        raise SmokeFailure('Original card-client bank report disappeared')
    driver.select('#checklist-report',choice)
    driver.click_button('Review final-report readiness')
    driver.wait_text('.approved-final-report','Credit-card evidence is stored separately','bank-only historical approval flags known card coverage')
    restart_driver()
    driver.open(binary)
    driver.fill('#password',password)
    driver.click_button('Unlock workspace')
    page(True)
    driver.wait_text('.card-totals','Net card spendingUSD -12.93','card review persists across process restart')
    if driver.text('.card-statement-view .field-help') != identity:
        raise SmokeFailure('Card source identity changed after restart')
    backup_folder=data_root / 'card-backups'
    backup_folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']")
    driver.click_button('Backup & recovery')
    driver.fill('#backup-folder',str(backup_folder))
    driver.fill('#backup-password',password)
    driver.click_button('Create encrypted backup')
    driver.wait_text('.backup-success','Encrypted backup saved.','card evidence encrypted backup created')
    backup_path=Path(driver.text('.backup-file-path'))
    driver.click_button('Lock workspace')
    driver.click('.restore-panel > summary')
    driver.fill('#restore-file',str(backup_path))
    driver.fill('#restore-password',password)
    driver.click_button('Verify backup for recovery')
    driver.wait_text('.restore-review','Fictional Oak Counseling','card history validated before recovery')
    driver.fill('#restore-confirmation','RESTORE')
    driver.click_button('Restore verified workspace')
    driver.wait_text('.auth-card','Workspace restored. Unlock it','card backup activated')
    driver.fill('#password',password)
    driver.click_button('Unlock workspace')
    page(True)
    driver.wait_text('.card-totals','Net card spendingUSD -12.93','card amounts restored without rewriting original source')
    if driver.text('.card-statement-view .field-help') != identity:
        raise SmokeFailure('Encrypted recovery changed card source identity')
    driver.click_button('Review card row 2')
    driver.click('.card-review-history > summary')
    driver.wait_text('.card-review-history','Withdraw corrected fictional charge','restored immutable card decision history')
    driver.wait_text('.card-review-history','USD 100.01','restored original card amount')
    driver.click_button('Discard card decision draft')
    driver.scroll_to('.card-statement-view','start')
    driver.screenshot(artifacts / 'native-card-restored.png')
    driver.select('#workspace-client','Alex Fictional')
    driver.wait_text('.card-statements','No card statements recorded','card source/client isolation')
    if 'Fictional household card' in driver.text('.card-statements'):
        raise SmokeFailure('Private card source crossed client boundaries')
    status('card signed CSV mapping, explicit decisions, exact debt/payment/spending reconciliation, correction/rejection history, bank-report guard, restart, encrypted recovery and client isolation')


def check_final_report_approval(driver: WebDriver, artifacts: Path, data_root: Path,
                                password: str, binary: Path,
                                restart_driver: Callable[[], None]) -> None:
    period_end=dt.datetime.now(ZoneInfo('America/Chicago')).date().replace(day=1)-dt.timedelta(days=1)
    period_start=period_end.replace(day=1)
    month=period_start.strftime('%Y-%m')
    # First prove that the existing incomplete household cannot be approved.
    driver.click_button('Final report checklist')
    driver.interactive('#checklist-report')
    choice=driver.observe_script("return Array.from(document.querySelector('#checklist-report').options).find(o=>o.textContent.startsWith('Fictional completeness-linked assessment'))?.textContent")
    if not choice:
        raise SmokeFailure('Missing original incomplete report for blocked approval check')
    driver.select('#checklist-report',choice)
    driver.click_button('Review final-report readiness')
    driver.wait_text('.final-readiness-status','Final approval blocked','missing evidence cannot be waived')
    if not driver.observe_script("return document.querySelector('#final-professional-notes').matches(':disabled')"):
        raise SmokeFailure('Missing evidence left final approval editable')
    driver.scroll_to('.final-report-approval','start')
    driver.screenshot(artifacts / 'native-final-approval-blocked.png')

    # Independent complete example, isolated from the deliberately incomplete cases.
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Clients']")
    driver.click('.page-heading button')
    driver.fill('#first-name','Jamie')
    driver.fill('#last-name','Complete')
    driver.fill('#household-size','1')
    driver.fill('#goals','Fictional historical completed-period assessment only; no scenario recommendation requested.')
    driver.click_button('Create client')
    driver.wait_text('.editor-panel h2','Jamie Complete','separate fictional final-report client')
    source=data_root / 'complete-final.csv'
    source.write_text(f'Date,Description,Amount,Category,Type\n{period_start},Complete fictional wages,2000.01,Wages,income\n{period_start+dt.timedelta(days=1)},Complete fictional rent,-1200.02,Housing,expense\n',encoding='utf-8')
    driver.fill('#csv-path',str(source))
    driver.click_button('Inspect CSV')
    driver.wait_text('.csv-preview',source.name,'complete example source')
    driver.fill('#csv-account-label','Complete fictional checking')
    for key,heading in [('date','1: Date'),('description','2: Description'),('amount','3: Amount'),('category','4: Category'),('kind','5: Type')]:
        driver.select(f'#csv-map-{key}',heading)
    for key,value in [('periodStart',str(period_start)),('periodEnd',str(period_end)),('openingBalance','1000.00'),('closingBalance','1799.99')]:
        driver.fill(f'#csv-{key}',value)
    driver.click("//label[contains(.,'Authorize retaining the complete original CSV')]/input",'xpath')
    driver.click_button('Import into review queue')
    driver.wait_text('.imports-progress','Pending 2','complete source not automatically approved')
    for index in (1,2):
        driver.click(f'.imports-table tbody tr:nth-child({index}) input[type=checkbox]')
    driver.click_button('Approve selected validated rows (2)')
    driver.wait_text('.imports-progress','Approved 2','individual source rows explicitly approved')
    driver.click('.financial-facts-editor > summary')
    set_fact_classification(driver,'Complete fictional wages','Verified take-home pay')
    set_fact_classification(driver,'Complete fictional rent','Necessary expense')
    driver.fill('#facts-as-of',str(period_end))
    driver.click_button('Add verified balance')
    driver.fill('#fact-balance-label-0','Complete fictional checking')
    driver.select('#fact-balance-kind-0','Cash / checking')
    driver.fill('#fact-balance-amount-0','1799.99')
    driver.fill('#fact-balance-evidence-0','Fictional complete household balance verified at period end; no debts or other balances.')
    driver.click('#facts-inventory')
    save_financial_facts(driver,'Synthetic complete inventory and independently checked classifications')

    def coverage_page() -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Financial Review']")
        driver.click_button('Completeness & final validation')
        driver.click('.saved-coverage-reviews > summary')
        driver.interactive('#coverage-review-from')
    coverage_page()
    driver.fill('#coverage-review-from',month)
    driver.fill('#coverage-review-through',month)
    driver.fill('#coverage-inventory-notes','Fictional sole household checking account reviewed; no other assets, debts or obligations.')
    driver.fill('#coverage-start-0',str(period_start))
    driver.fill('#coverage-end-0',str(period_end))
    driver.click(".saved-coverage-reviews input[aria-label^='Use ']")
    driver.fill('#coverage-conclusion','Synthetic complete reconciled period evidence; no missing items.')
    driver.click('#coverage-inventory-reviewed')
    driver.click_button('Save completeness review')
    driver.wait_text('.saved-coverage-reviews','Completeness review saved as version 1.','complete expected inventory recorded')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.click('.saved-assessments > summary')
    driver.interactive('#record-title')
    driver.fill('#record-from',month)
    driver.fill('#record-through',month)
    driver.fill('#record-title','Fictional final-ready completed-period assessment')
    driver.fill('#record-findings','Verified fictional income USD 2000.01, necessary spending USD 1200.02, net operating cash USD 799.99 and checking USD 1799.99.')
    driver.fill('#record-recommendations','Historical assessment only; discuss future objectives at the next appointment.')
    driver.click('#record-include-completeness')
    driver.click_button('Review saved completeness context')
    driver.wait_text('.coverage-review-selection','no other assets, debts or obligations','exact inventory reviewed before capture')
    driver.click('#record-ack')
    driver.click_button('Save assessment record')
    driver.wait_text('.saved-assessments','Assessment record saved with its supporting evidence.','complete immutable assessment captured')
    # The save notice can render before the refreshed directory contains the
    # selected record. Capture its real ID only after its option is selected;
    # an empty ID would select the checklist's placeholder, not an assessment.
    parent=driver.wait(lambda: driver.observe_script(
        "const select=document.querySelector('#saved-assessment');"
        "return select?.value && select.selectedOptions[0]?.textContent.startsWith('Fictional final-ready completed-period assessment') ? select.value : false;"),
        'newly saved final-report assessment selected in its refreshed directory')
    driver.click_button('Final report checklist')

    def open_approval() -> None:
        driver.interactive('#checklist-report')
        option=driver.observe_script("return Array.from(document.querySelector('#checklist-report').options).find(o=>o.value==="+json.dumps(parent)+")?.textContent")
        if not option:
            raise SmokeFailure('Final report parent disappeared from its client directory')
        driver.select('#checklist-report',option)
        driver.click_button('Review final-report readiness')
    open_approval()
    driver.wait_text('.final-readiness-status','Required checks passed','complete independent source passes every required gate')
    driver.fill('#final-account-0','Full completed-period scope and original checking evidence inspected.')
    planning='Historical assessment only; no scenario advice requested or captured. Fictional client agreed to these limits.'
    professional='Fictional counselor checked original sources, account scope, possible duplicates, exclusions, classifications, exact balances and recommendations.'
    driver.fill('#final-planning-notes',planning)
    driver.fill('#final-professional-notes',professional)
    for key in ['confirmClient','confirmSources','confirmInventory','confirmAssumptions']:
        driver.click(f'#final-{key}')
    if not driver.observe_script("return document.querySelector('#checklist-report').matches(':disabled')"):
        raise SmokeFailure('An unsaved approval allowed its selected assessment to change')
    driver.scroll_to('#final-professional-notes','start')
    driver.screenshot(artifacts / 'native-final-approval-confirmations.png')
    driver.click_button('Approve final counselor report')
    driver.wait_text('.approved-final-report',professional,'explicit counselor approval preserved')
    driver.wait_text('.final-report-current-status','supporting context matches','new approval context current')
    identity=driver.text('.approved-final-report .field-help')

    def export(label: str, historical: bool) -> None:
        destination=artifacts / f'approved-final-{label}.pdf'
        driver.fill_pdf_destination('#final-pdf-path',destination)
        driver.click_button('Export approved final report PDF')
        driver.wait_text('.final-report-approval > .success-message',str(destination),'approved final PDF written to a new local filename')
        text=subprocess.run(['pdftotext','-layout',str(destination),'-'],check=True,capture_output=True,text=True).stdout
        text=' '.join(text.split())
        for expected in ['FINAL COUNSELOR REPORT','COUNSELOR APPROVED','Jamie Complete','counselor-final-report-v1','Operating cash flow: USD 799.99','Estimated net worth at balance date: USD 1799.99',planning,professional]:
            if expected not in text:
                raise SmokeFailure(f'Final PDF omitted {expected!r}')
        if 'Final report validation / not eligible' in text:
            raise SmokeFailure('Final PDF incorrectly retained the internal-only eligibility heading')
        if historical and 'CURRENT CONTEXT NEEDS REVIEW' not in text:
            raise SmokeFailure('Historical final PDF omitted its current review flag')
    export('current',False)
    driver.scroll_to('.approved-final-report','start')
    driver.screenshot(artifacts / 'native-approved-final-report.png')
    coverage_page()
    driver.fill('#coverage-conclusion','Later fictional follow-up context; original approval must remain unchanged.')
    driver.click_button('Save completeness review')
    driver.wait_text('.saved-coverage-reviews','Completeness review saved as version 2.','later review retained without rewriting approval')
    restart_driver()
    driver.open(binary)
    driver.fill('#password',password)
    driver.click_button('Unlock workspace')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.select('#workspace-client','Jamie Complete')
    driver.click_button('Final report checklist')
    open_approval()
    driver.wait_text('.final-report-current-status','Historical approval','original approval survives restart with changed-context flag')
    if driver.text('.approved-final-report .field-help') != identity:
        raise SmokeFailure('Restart or source context change replaced original approval identity')
    driver.wait_text('.approved-final-report',professional,'restart retains original counselor notes')
    backup_folder=data_root / 'final-report-backups'
    backup_folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']")
    driver.click_button('Backup & recovery')
    driver.fill('#backup-folder',str(backup_folder))
    driver.fill('#backup-password',password)
    driver.click_button('Create encrypted backup')
    driver.wait_text('.backup-success','Encrypted backup saved.','final approval encrypted backup created')
    backup_path=Path(driver.text('.backup-file-path'))
    driver.click_button('Lock workspace')
    driver.click('.restore-panel > summary')
    driver.fill('#restore-file',str(backup_path))
    driver.fill('#restore-password',password)
    driver.click_button('Verify backup for recovery')
    driver.wait_text('.restore-review','Fictional Oak Counseling','final approval backup validated')
    driver.fill('#restore-confirmation','RESTORE')
    driver.click_button('Restore verified workspace')
    driver.wait_text('.auth-card','Workspace restored. Unlock it','final approval backup restored')
    driver.fill('#password',password)
    driver.click_button('Unlock workspace')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.select('#workspace-client','Jamie Complete')
    driver.click_button('Final report checklist')
    open_approval()
    driver.wait_text('.final-report-current-status','Historical approval','encrypted recovery retains original approval and current review flags')
    if driver.text('.approved-final-report .field-help') != identity:
        raise SmokeFailure('Encrypted recovery replaced original approval ID or digest')
    export('restored-historical',True)
    driver.scroll_to('.approved-final-report','start')
    driver.screenshot(artifacts / 'native-final-report-restored.png')
    status('final report blocked evidence, complete exact-cent approval, explicit confirmations, immutable PDF, changed-context flags, restart and encrypted recovery')


def check_saved_coverage_reviews(driver: WebDriver, artifacts: Path, data_root: Path,
                                 password: str, binary: Path,
                                 restart_driver: Callable[[], None]) -> None:
    original='Fictional loan statement and repayment terms remain unprovided.'
    changed='Fictional lender terms requested again at the next appointment.'
    def review_page(select_client: bool = False) -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Financial Review']")
        if select_client:
            driver.select('#workspace-client','Alex Fictional')
        driver.click_button('Completeness & final validation')
        driver.click('.saved-coverage-reviews > summary')
        driver.interactive('#coverage-review-from')
    review_page()
    driver.fill('#coverage-review-from','2026-09')
    driver.fill('#coverage-review-through','2026-10')
    driver.fill('#coverage-inventory-notes','Fictional household checking, savings and missing loan inventory reviewed.')
    driver.fill('#coverage-missing-information',original)
    driver.fill('#coverage-conclusion','Collect the remaining evidence before final approval.')
    count=driver.observe_script("return document.querySelectorAll('input[id^=coverage-label-]').length")
    if not isinstance(count,int) or count < 2:
        raise SmokeFailure('The saved review omitted known checking/savings accounts')
    for i in range(count):
        driver.fill(f'#coverage-start-{i}','2026-09-01')
        driver.fill(f'#coverage-end-{i}','2026-10-31')
    driver.click(".saved-coverage-reviews input[aria-label^='Use ']")
    driver.click_button('Add missing account or obligation')
    driver.fill(f'#coverage-label-{count}','Fictional unprovided loan')
    driver.select(f'#coverage-kind-{count}','loan')
    driver.fill(f'#coverage-exception-{count}','Source statement not yet received; balance remains unknown.')
    driver.click('#coverage-inventory-reviewed')
    driver.click_button('Save completeness review')
    driver.wait_text('.saved-coverage-reviews','Completeness review saved as version 1.','scoped completeness context saved')
    driver.select('#coverage-review-version','Version 1 — 2026-09 through 2026-10')
    driver.wait_text('.saved-coverage-reviews','Historical completeness review version 1.','numbered review history is read-only')
    if not driver.observe_script("return document.querySelector('#coverage-inventory-notes').matches(':disabled')"):
        raise SmokeFailure('Numbered saved completeness history was editable')
    driver.select('#coverage-review-version','Current editable review (1)')
    driver.wait_value('#coverage-missing-information',original,'original completeness draft reopened')
    driver.scroll_to('.saved-coverage-reviews','start')
    driver.screenshot(artifacts / 'native-saved-completeness-review.png')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.click('.saved-assessments > summary')
    driver.interactive('#record-title')
    driver.fill('#record-from','2026-09')
    driver.fill('#record-through','2026-10')
    driver.fill('#record-title','Fictional completeness-linked assessment')
    driver.click('#record-include-completeness')
    driver.click_button('Review saved completeness context')
    driver.wait_text('.coverage-review-selection',original,'reviewed saved completeness evidence before report capture')
    driver.interactive('#record-ack')
    driver.click('#record-ack')
    driver.click_button('Save assessment record')
    driver.wait_text('.saved-assessments','Assessment record saved with its supporting evidence.','completeness version attached to immutable report')
    driver.wait_text('.saved-assessment-view .captured-coverage-review','Saved completeness review version 1','original review retained in recorded report')
    record_id=driver.observe_script("return document.querySelector('#saved-assessment').value")
    totals=driver.text('.saved-assessment-view .assessment-totals')
    review_page()
    driver.fill('#coverage-missing-information',changed)
    driver.click_button('Save completeness review')
    driver.wait_text('.saved-coverage-reviews','Completeness review saved as version 2.','amendment retained as a new review version')
    restart_driver()
    driver.open(binary)
    driver.fill('#password',password)
    driver.click_button('Unlock workspace')
    review_page(True)
    driver.wait_value('#coverage-missing-information',changed,'current review survives restart')
    driver.select('#coverage-review-version','Version 1 — 2026-09 through 2026-10')
    driver.wait_value('#coverage-missing-information',original,'original review history survives restart')
    driver.select('#coverage-review-version','Current editable review (2)')
    driver.wait_value('#coverage-missing-information',changed,'return to current completeness context')
    backup_folder=data_root / 'coverage-review-backups'
    backup_folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']")
    driver.click_button('Backup & recovery')
    driver.fill('#backup-folder',str(backup_folder))
    driver.fill('#backup-password',password)
    driver.click_button('Create encrypted backup')
    driver.wait_text('.backup-success','Encrypted backup saved.','completeness review backup created')
    backup_path=Path(driver.text('.backup-file-path'))
    driver.click_button('Lock workspace')
    driver.click('.restore-panel > summary')
    driver.fill('#restore-file',str(backup_path))
    driver.fill('#restore-password',password)
    driver.click_button('Verify backup for recovery')
    driver.wait_text('.restore-review','Fictional Oak Counseling','completeness review backup verified')
    driver.fill('#restore-confirmation','RESTORE')
    driver.click_button('Restore verified workspace')
    driver.wait_text('.auth-card','Workspace restored. Unlock it','completeness review backup activated')
    driver.fill('#password',password)
    driver.click_button('Unlock workspace')
    review_page(True)
    driver.wait_value('#coverage-missing-information',changed,'current completeness review restored')
    driver.select('#coverage-review-version','Version 1 — 2026-09 through 2026-10')
    driver.wait_value('#coverage-missing-information',original,'original review restored read-only')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.click('.saved-assessments > summary')
    driver.interactive('#saved-assessment')
    choice=driver.observe_script("return Array.from(document.querySelector('#saved-assessment').options).find(o=>o.value==="+json.dumps(record_id)+")?.textContent")
    if not choice or 'Needs review' not in choice:
        raise SmokeFailure('A newer completeness review did not flag its original report')
    driver.select('#saved-assessment',choice)
    driver.wait_text('.saved-assessment-view .captured-coverage-review',original,'report retains original completeness version after encrypted recovery')
    if changed in driver.text('.saved-assessment-view .captured-coverage-review'):
        raise SmokeFailure('Current completeness notes replaced original report context')
    if driver.text('.saved-assessment-view .assessment-totals') != totals:
        raise SmokeFailure('Coverage review edits changed the saved approved financial amounts')
    destination=artifacts / 'captured-completeness-restored.pdf'
    driver.fill_pdf_destination('#assessment-pdf-path',destination)
    driver.click_button('Export saved assessment PDF')
    driver.wait_text('.saved-assessments > .success-message',str(destination),'captured completeness PDF exported after recovery')
    text=subprocess.run(['pdftotext','-layout',str(destination),'-'],check=True,capture_output=True,text=True).stdout
    for expected in ['Captured counselor completeness review',original,'Fictional unprovided loan','NEEDS REVIEW']:
        if expected not in text:
            raise SmokeFailure(f'Captured completeness PDF omitted {expected!r}')
    if changed in text:
        raise SmokeFailure('Historical completeness PDF substituted current review notes')
    driver.scroll_to('.saved-assessment-view .captured-coverage-review','start')
    driver.screenshot(artifacts / 'native-captured-completeness-review.png')
    status('saved completeness create/amend/read-only history, expected accounts and source selection, report binding, restart, encrypted recovery and historical PDF')


def check_completeness(driver: WebDriver, artifacts: Path) -> None:
    # Run after restart/recovery and historical branding checks, through visible UI.
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Financial Review']")
    driver.click_button('Completeness & final validation')
    driver.interactive('#coverage-from')
    driver.fill('#coverage-from','2026-09')
    driver.fill('#coverage-through','2026-09')
    driver.click_button('Review financial completeness')
    driver.wait_text('.coverage-results','2026-09-01 through 2026-09-30','explicit completeness period')
    driver.wait_text('.coverage-results','Synthetic checking','original account identity in coverage')
    driver.wait_text('.coverage-results','Not eligible for finalization','unknown inventory blocks finalization')
    driver.wait_text('.coverage-results','of 30 calendar days','calendar-day coverage uses exact selected month')
    driver.scroll_to("section[aria-label='Financial completeness review']",'start')
    driver.screenshot(artifacts / 'native-financial-completeness.png')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.click_button('Final report checklist')
    driver.interactive('#checklist-report')
    choice=driver.observe_script("return Array.from(document.querySelector('#checklist-report').options).find(o=>o.textContent.startsWith('Fictional branded progress assessment'))?.textContent")
    if not choice or 'Needs review' not in choice:
        raise SmokeFailure('Saved-report validation did not expose the historical changed-context report')
    driver.select('#checklist-report',choice)
    driver.wait_text('.report-validation','coverage-review-v1','versioned derived report checklist')
    driver.wait_text('.report-validation','Current supporting context · Needs review','current supporting changes remain explicit')
    driver.wait_text('.report-validation','Financial completeness · Blocked','incomplete historical report cannot finalize')
    driver.wait_text('.report-validation .coverage-results','2026-09-01 through 2026-10-31','report checklist uses captured period rather than live coverage period')
    driver.scroll_to("section[aria-label='Saved report checklist']",'start')
    driver.screenshot(artifacts / 'native-report-validation.png')
    pdf_text=subprocess.run(['pdftotext','-layout',str(artifacts / 'firm-branding-restored.pdf'),'-'],check=True,capture_output=True,text=True).stdout
    for expected in ['Final report validation / not eligible for finalization','coverage-review-v1','Financial completeness / Blocked','Declared account coverage']:
        if expected not in pdf_text:
            raise SmokeFailure(f'Historical internal PDF omitted validation evidence {expected!r}')
    status('live account/source/date coverage and captured final-report checklist after restart and encrypted recovery, including PDF disclosures')


def check_firm_branding(driver: WebDriver, artifacts: Path, data_root: Path, password: str,
                        binary: Path, restart_driver: Callable[[], None]) -> None:
    import struct
    import zlib
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    logo=data_root / 'fictional-logo.png'
    logo.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',2,1,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(b'\x00\xb0\x8a\x42\x26\x3d\x48'))+chunk(b'IEND',b''))
    firm='Fictional Oak Counseling'
    def settings() -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']")
        driver.click_button('Branding')
        driver.interactive('#firm-name')
    settings()
    driver.fill('#firm-name',firm)
    driver.fill('#firm-address','123 Fictional Lane\nExample Town')
    driver.fill('#firm-email','office@example.invalid')
    driver.fill('#firm-phone','555-0100')
    driver.fill('#firm-website','https://example.invalid')
    driver.fill('#firm-disclosure','Fictional firm disclosure: verify the stated financial assumptions.')
    driver.fill('#firm-logo-path',str(logo))
    driver.click_button('Inspect logo')
    driver.wait_text('.firm-branding','Logo inspected locally','local bounded logo inspection')
    driver.element('.firm-brand-preview img')
    driver.click_button('Save firm branding')
    driver.wait_text('.firm-branding','Saved branding version 1','firm branding saved')
    driver.wait_content('.workspace-label strong',firm,'workspace uses saved firm name')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.select('#workspace-client','Alex Fictional')
    driver.click('.saved-assessments > summary')
    driver.wait_text('.report-branding-preview','Saved firm branding version 1','current branding reviewed for capture')
    driver.fill('#record-from','2026-09')
    driver.fill('#record-through','2026-10')
    driver.fill('#record-title','Fictional branded progress assessment')
    driver.click('#record-include-planning')
    driver.click_button('Review current saved plans')
    driver.wait_text('.scenario-selection .scenario-budget-totals','Income$3,250.00','saved assumptions reviewed for branded report')
    driver.fill('#record-scenario-label','Fictional branded saved plan')
    driver.fill('#record-scenario-assumptions','Entered plans remain separate from approved statement activity.')
    driver.select('#record-debt-strategy','Avalanche')
    driver.click('#record-ack')
    driver.click_button('Save assessment record')
    driver.wait_text('.saved-assessments','Assessment record saved with its supporting evidence.','branded assessment captured')
    driver.wait_text("section[aria-label='Captured firm branding']",'555-0100','captured original contact')
    record_id=driver.observe_script("return document.querySelector('#saved-assessment').value")
    if not record_id:
        raise SmokeFailure('The branded assessment ID was not shown in the saved-record selector')
    # The linked client plan inherits the same captured branding; its established
    # independently checked allocations and mandatory warnings remain enabled.
    save_client_action_plan(driver,artifacts,"branded-current")
    settings()
    driver.fill('#firm-phone','555-0199')
    driver.click_button('Save firm branding')
    driver.wait_text('.firm-branding','Saved branding version 2','later contact change saved')
    driver.scroll_to('.firm-branding','start')
    driver.command('POST','/execute/async',{'script':'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));','args':[]})
    driver.screenshot(artifacts / 'native-firm-branding.png')
    restart_driver()
    driver.open(binary)
    driver.fill('#password',password)
    driver.click_button('Unlock workspace')
    driver.wait_content('.workspace-label strong',firm,'firm display name survives restart')
    settings()
    driver.wait_value('#firm-phone','555-0199','latest firm contact survives restart')
    backup_folder=data_root / 'branding-backups'
    backup_folder.mkdir()
    driver.click_button('Backup & recovery')
    driver.fill('#backup-folder',str(backup_folder))
    driver.fill('#backup-password',password)
    driver.click_button('Create encrypted backup')
    driver.wait_text('.backup-success','Encrypted backup saved.','branding backup saved')
    backup_path=Path(driver.text('.backup-file-path'))
    driver.click_button('Lock workspace')
    if firm in driver.text('body'):
        raise SmokeFailure('Private firm branding remained visible while locked')
    driver.click('.restore-panel > summary')
    driver.fill('#restore-file',str(backup_path))
    driver.fill('#restore-password',password)
    driver.click_button('Verify backup for recovery')
    driver.wait_text('.restore-review',firm,'branding verified for recovery')
    driver.fill('#restore-confirmation','RESTORE')
    driver.click_button('Restore verified workspace')
    driver.wait_text('.auth-card','Workspace restored. Unlock it','branding backup activated')
    driver.fill('#password',password)
    driver.click_button('Unlock workspace')
    settings()
    driver.wait_value('#firm-phone','555-0199','current branding restored')
    driver.wait_text("section[aria-label='Firm branding history']",'Version 1: Fictional Oak Counseling','original branding version restored')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.select('#workspace-client','Alex Fictional')
    driver.click('.saved-assessments > summary')
    driver.interactive('#saved-assessment')
    choice=driver.observe_script("return Array.from(document.querySelector('#saved-assessment').options).find(o=>o.value==="+json.dumps(record_id)+")?.textContent")
    if not choice or 'Needs review' not in choice:
        raise SmokeFailure('The historical branded report did not flag its later branding change')
    driver.select('#saved-assessment',choice)
    driver.wait_text("section[aria-label='Captured firm branding']",'555-0100','report retains captured contact after recovery')
    if '555-0199' in driver.text("section[aria-label='Captured firm branding']"):
        raise SmokeFailure('Current contact details replaced the historical report branding')
    driver.element("section[aria-label='Captured firm branding'] img")
    destination=artifacts / 'firm-branding-restored.pdf'
    driver.fill_pdf_destination('#assessment-pdf-path',destination)
    driver.click_button('Export saved assessment PDF')
    driver.wait_text('.saved-assessments > .success-message',str(destination),'branded historical PDF exported')
    text=subprocess.run(['pdftotext','-layout',str(destination),'-'],check=True,capture_output=True,text=True).stdout
    for expected in [firm,'office@example.invalid','555-0100','Fictional firm disclosure','NEEDS REVIEW']:
        if expected not in text:
            raise SmokeFailure(f'Branded historical PDF omitted {expected!r}')
    if '555-0199' in text:
        raise SmokeFailure('The historical PDF used the current contact details')
    status('firm name/contact/logo/disclosure, reviewed report/client-plan capture, restart, encrypted recovery and historical PDF branding')


def check_document_collection(driver: WebDriver, artifacts: Path, label: str, initial: bool = False) -> None:
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Documents']")
    driver.click_button("Collection checklist")
    driver.element(".document-collection")
    if initial:
        driver.click_button("Add document request")
        driver.fill("#collection-request-title", "Fictional September checking statement")
        driver.select("#collection-account", "Synthetic checking")
        driver.fill_native_date("#collection-periodStart", "2026-09-01")
        driver.fill_native_date("#collection-periodEnd", "2026-09-30")
        driver.fill_native_date("#collection-dueDate", "2026-10-30")
        driver.fill("#collection-notes", "Original fictional collection request.")
        driver.click_button("Save document checklist")
        driver.wait_text(".collection-counts", "Saved version 1", "missing request saved")
        driver.click_button("Add document request")
        driver.fill("#collection-request-title", "Fictional optional pay stub")
        driver.select("#collection-kind", "Pay stub")
        driver.click_button("Save document checklist")
        driver.wait_text(".collection-counts", "Saved version 2", "second request saved")
        driver.click("button[aria-label='Open request Fictional optional pay stub']")
        driver.click("#collection-archived")
        driver.click_button("Save document checklist")
        driver.wait_text(".collection-counts", "Saved version 3", "request archive preserves history")
        driver.click("button[aria-label='Open request Fictional September checking statement']")
        driver.select("#collection-status", "Received (not financially approved)")
        driver.fill_native_date("#collection-receivedDate", "2026-10-14")
        driver.select("#collection-source", "checking.csv — Synthetic checking")
        driver.fill("#collection-notes", "Fictional receipt recorded; financial approval remains separate.")
        driver.click_button("Save document checklist")
        driver.wait_text(".collection-counts", "Saved version 4", "received request and source link saved")
    driver.wait_text(".collection-counts", "0 missing", f"saved missing count ({label})")
    driver.wait_text(".collection-counts", "1 received", f"saved receipt count ({label})")
    driver.wait_text(".collection-counts", "1 archived", f"saved archive count ({label})")
    driver.click("button[aria-label='Open request Fictional September checking statement']")
    driver.wait_value("#collection-receivedDate", "2026-10-14", f"original received date ({label})")
    driver.wait_value("#collection-notes", "Fictional receipt recorded; financial approval remains separate.", f"receipt notes persisted ({label})")
    chosen=driver.observe_script("const s=document.querySelector('#collection-source');return s.options[s.selectedIndex].textContent")
    if chosen != "checking.csv — Synthetic checking":
        raise SmokeFailure("Checklist lost its original scoped source link")
    version=driver.observe_script("return Array.from(document.querySelector('#collection-history').options).find(o=>o.value==='1')?.textContent")
    if not version:
        raise SmokeFailure("Checklist original version missing")
    driver.select("#collection-history", version)
    driver.wait_text(".document-collection", "Historical checklist version 1 of 4", f"original collection history ({label})")
    driver.click("button[aria-label='Open request Fictional September checking statement']")
    driver.wait_value("#collection-status", "missing", f"original status remains missing ({label})")
    driver.wait_value("#collection-notes", "Original fictional collection request.", f"original request notes unchanged ({label})")
    if driver.observe_script("return document.querySelector('#collection-request-title').matches(':disabled')") is not True:
        raise SmokeFailure("Historical checklist fields are editable")
    driver.select("#collection-history", "Current checklist")
    driver.wait_text(".collection-counts", "Saved version 4", "return to current checklist")
    driver.select("#collection-filter", "Archived")
    driver.wait_text(".collection-table", "Fictional optional pay stub", "archived request remains available")
    driver.select("#workspace-client", "Robin Synthetic")
    driver.wait_text(".document-collection", "No document requests yet", "other client has no collection requests")
    driver.select("#workspace-client", "Alex Fictional")
    driver.wait_text(".collection-counts", "1 received", "original client collection restored")
    driver.scroll_to(".document-collection", "start")
    driver.command("POST", "/execute/async", {"script":"const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));", "args":[]})
    driver.screenshot(artifacts / f"native-document-collection-{label}.png")
    check_statement_accounts(driver, artifacts, label, initial)
    driver.open_client("Alex Fictional")
    status(f"document collection create/edit/receipt/archive/history, scoped source and client isolation ({label})")


def check_statement_accounts(driver: WebDriver, artifacts: Path, label: str, initial: bool = False) -> None:
    driver.click_button("Statement accounts")
    driver.element(".statement-accounts")
    if initial:
        driver.click_button("Add statement account")
        driver.fill("#account-label", "Fictional standalone checking")
        driver.click_button("Save statement account")
        driver.wait_text(".statement-accounts", "Statement account saved locally.", "standalone account saved")
        driver.click("button[aria-label='Manage account Fictional standalone checking']")
        driver.fill("#account-label", "Fictional renamed checking")
        driver.click_button("Save statement account")
        driver.element("button[aria-label='Manage account Fictional renamed checking']")
        driver.click("button[aria-label='Manage account Fictional renamed checking']")
        driver.click("#account-archived")
        driver.click_button("Save statement account")
        driver.wait(lambda: driver.observe_script("return document.querySelector('#account-label')===null"), "account archived")
        driver.select("#accounts-filter", "Archived accounts")
        driver.click("button[aria-label='Manage account Fictional renamed checking']")
        driver.wait_text("section[aria-label='Account change history']", "Current saved version 3", "archive history saved")
        driver.click("#account-archived")
        driver.click_button("Save statement account")
        driver.wait(lambda: driver.observe_script("return document.querySelector('#account-label')===null"), "account reactivated")
        driver.select("#accounts-filter", "Active accounts")
    driver.click("button[aria-label='Manage account Fictional renamed checking']")
    driver.wait_value("#account-label", "Fictional renamed checking", f"account current name ({label})")
    driver.wait_text("section[aria-label='Account change history']", "Current saved version 4", f"account history persisted ({label})")
    driver.wait_text("section[aria-label='Account change history']", "Version 1: Fictional standalone checking", "original account label retained")
    driver.wait_text("section[aria-label='Account change history']", "Version 3: Fictional renamed checking · Archived", "archived version retained")
    if driver.observe_script("return document.querySelector('#account-archived').checked") is not False:
        raise SmokeFailure("Reactivated account unexpectedly remained archived")
    driver.scroll_to(".statement-accounts", "start")
    driver.command("POST", "/execute/async", {"script":"const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));", "args":[]})
    driver.screenshot(artifacts / f"native-statement-accounts-{label}.png")
    driver.click_button("Discard account edits")
    driver.select("#workspace-client", "Robin Synthetic")
    driver.wait_text(".statement-accounts", "No statement accounts yet", "other client account isolation")
    driver.select("#workspace-client", "Alex Fictional")
    driver.element("button[aria-label='Manage account Fictional renamed checking']")
    status(f"standalone statement account create/rename/archive/reactivate/history and isolation ({label})")


def check_workspace_pages(driver: WebDriver, artifacts: Path) -> None:
    count = driver.observe_script("return document.querySelectorAll('nav[aria-label=\"Main navigation\"] button').length")
    if count != 12:
        raise SmokeFailure("The original twelve workspace sections are not all reachable")
    for page, heading in [("Financial Review", "Client financial review"), ("Transactions", "Transaction review queue"), ("Documents", "Documents and statement accounts")]:
        driver.click(f"nav[aria-label='Main navigation'] button[aria-label='{page}']")
        driver.wait_text("#imports-title", heading, f"focused {page} page")
        driver.element(".imports-totals" if page != "Documents" else "#csv-path")
        if page != "Documents" and driver.visible("#csv-path"):
            raise SmokeFailure(f"Document import controls leaked into {page}")
    driver.click(".imports-sources > summary")
    driver.wait_text(".imports-sources", "checking.csv", "saved source inventory in Documents")
    driver.click("//nav[@aria-label='Documents sections']//button[starts-with(normalize-space(.), 'Scanned documents / OCR')]", "xpath")
    driver.wait_text(".planned-module-page", "mandatory verification", "future OCR area is explicitly planned")
    driver.click("nav[aria-label='Documents sections'] button:first-child")
    driver.wait_text("#imports-title", "Documents and statement accounts", "return from planned area")
    for page, selector, expected in [("Budget Planner", ".budget-totals", "$3,250.00"), ("Debt Planner", "#debt-label-0", "Alex synthetic small debt"), ("Savings Planner", "#savings-priorities", "Alex synthetic reserve priority"), ("Cash-Flow Calendar", "#cashflow-account-label-0", "Alex synthetic checking")]:
        driver.click(f"nav[aria-label='Main navigation'] button[aria-label='{page}']")
        if selector.startswith("#"):
            driver.wait_value(selector, expected, f"persisted {page} data")
        else:
            driver.wait_text(selector, expected, f"persisted {page} figures")
    driver.select("#workspace-client", "Robin Synthetic")
    driver.wait_text(".cashflow-panel", "No saved cash-flow plan yet", "client context switches to isolated empty plan")
    if driver.visible("#cashflow-account-label-0"):
        raise SmokeFailure("Prior client's cash-flow rows remained in a module page")
    driver.select("#workspace-client", "Alex Fictional")
    driver.wait_value("#cashflow-account-label-0", "Alex synthetic checking", "client context returns to saved plan")
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Financial Scenarios']")
    driver.wait_text(".combined-scenarios", "Compare combined financial scenarios", "scenario page available")
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
    driver.wait_text(".saved-assessments", "Saved counselor assessments", "report page available")
    if driver.visible("#csv-path") or driver.visible(".imports-table"):
        raise SmokeFailure("Reports page contains unrelated import/review controls")
    driver.scroll_to(".page-heading", "start")
    driver.command("POST", "/execute/async", {"script":"const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));", "args":[]})
    driver.screenshot(artifacts / "native-workspace-pages.png")
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']")
    driver.click("nav[aria-label='Settings sections'] button:nth-child(4)")
    driver.interactive("#firm-name")
    driver.wait_text(".firm-branding", "No branding changes yet", "branding page is a working saved form")
    driver.open_client("Alex Fictional")
    driver.interactive("#first-name")
    status("all twelve main pages, saved planner data, isolated client context, future areas and report/admin separation")


def check_client_care(driver: WebDriver, artifacts: Path, label: str, initial: bool = False) -> None:
    driver.open_client("Alex Fictional")
    if driver.command("GET", f"/element/{driver.element('.client-care')}/property/open") is not True:
        driver.click(".client-care > summary")
    driver.interactive("#intake-address")
    if initial:
        driver.fill("#intake-address", "123 Fictional Lane\nExample City")
        driver.fill("#intake-household", "Two fictional adults and one dependent")
        driver.select("#intake-employment", "Employed")
        driver.fill("#intake-employer", "Alex fictional employer")
        driver.fill("#intake-occupation", "Fictional office role")
        driver.fill("#intake-income-notes", "Entered intake context only; verify financial documents separately.")
        driver.click_button("Add follow-up")
        driver.select("#follow-kind-0", "Appointment")
        driver.fill("#follow-title-0", "Review fictional statements")
        driver.fill_native_date("#follow-date-0", "2100-02-03")
        driver.fill("#follow-time-0", "09:30")
        driver.wait_value("#follow-time-0", "09:30", "explicit appointment time")
        driver.click_button("Add follow-up")
        driver.fill("#follow-title-1", "Collect missing fictional documents")
        driver.fill_native_date("#follow-date-1", "1900-01-01")
        driver.click_button("Save intake and follow-ups")
        driver.wait_text(".client-care .success-message", "saved locally", "initial encrypted client intake save")
        driver.wait_text(".care-reminders", "1 overdue", "only open past task generates reminder")
        driver.click("nav[aria-label='Main navigation'] button:nth-child(1)")
        driver.wait_text(".overview-table", "Collect missing fictional documents", "dashboard includes the saved overdue task")
        driver.select("#follow-up-view", "Overdue")
        driver.select("#follow-up-kind", "Tasks")
        driver.click_button("Apply follow-up filters")
        driver.wait_text(".overview-basis", "1 matching item", "exact overdue task filter")
        driver.click(".overview-open")
        driver.wait_value("#follow-title-1", "Collect missing fictional documents", "dashboard opens the correct client task")
        driver.wait(lambda:driver.observe_script("return document.activeElement.id") == "follow-title-1", "selected overdue task receives keyboard focus")
        driver.select("#follow-status-1", "Completed")
        driver.fill("#follow-notes-1", "Reviewed the synthetic documents")
        driver.click_button("Add follow-up")
        driver.fill("#follow-title-2", "Cancelled fictional call")
        driver.fill_native_date("#follow-date-2", "1900-01-01")
        driver.select("#follow-status-2", "Cancelled")
        driver.click_button("Save intake and follow-ups")
        driver.wait_text(".care-reminders", "0 overdue", "completed and cancelled tasks stop reminders")
    driver.wait_value("#intake-employer", "Alex fictional employer", f"saved employment intake ({label})")
    driver.wait_value("#follow-time-0", "09:30", f"saved explicit appointment time ({label})")
    driver.wait_value("#follow-date-0", "2100-02-03", f"saved appointment date ({label})")
    driver.wait_value("#follow-status-1", "completed", f"retained task completion ({label})")
    driver.wait_value("#follow-notes-1", "Reviewed the synthetic documents", f"retained task outcome ({label})")
    driver.wait_value("#follow-status-2", "cancelled", f"retained cancellation ({label})")
    driver.wait_text(".care-reminders", "0 overdue", f"closed tasks remain out of reminders ({label})")
    driver.scroll_to(".care-reminders", "start")
    driver.screenshot(artifacts / f"native-client-intake-{label}.png")
    driver.scroll_to("#follow-title-0", "start")
    driver.screenshot(artifacts / f"native-client-follow-ups-{label}.png")
    driver.click(".client-care > summary")
    driver.open_client("Robin Synthetic")
    driver.click(".client-care > summary")
    driver.wait_value("#intake-employer", "", "other client has no employment intake")
    driver.wait_text(".client-care", "No appointments or follow-up tasks recorded.", "follow-ups remain isolated by client")
    driver.click(".client-care > summary")
    driver.open_client("Alex Fictional")
    status(f"optional client intake, explicit appointments, overdue/completed/cancelled tasks and client isolation ({label})")
    check_follow_up_overview(driver, artifacts, label)


def check_follow_up_overview(driver: WebDriver, artifacts: Path, label: str) -> None:
    driver.click("nav[aria-label='Main navigation'] button:nth-child(1)")
    driver.wait_text(".overview-table", "Review fictional statements", f"saved appointment on dashboard ({label})")
    expected = [["Overdue", "0"], ["Due today", "0"], ["Upcoming", "1"], ["Completed / cancelled", "2"]]
    driver.wait(lambda:driver.observe_script("return Array.from(document.querySelector('.overview-counts').children).map(x=>[x.querySelector('span').textContent,x.querySelector('strong').textContent])") == expected, "dashboard counts use only latest saved follow-up versions")
    driver.select("#follow-up-view", "Completed / cancelled")
    driver.select("#follow-up-kind", "Tasks")
    driver.click_button("Apply follow-up filters")
    driver.wait_text(".overview-basis", "2 matching items", "closed tasks remain separate from open reminders")
    driver.wait_text(".overview-table", "Collect missing fictional documents", "completed item remains reviewable")
    driver.wait_text(".overview-table", "Cancelled fictional call", "cancelled item remains reviewable")
    driver.fill("#follow-up-search", "Robin Synthetic")
    driver.wait_value("#follow-up-search", "Robin Synthetic", "explicit overview client search")
    driver.click_button("Apply follow-up filters")
    driver.wait_text(".overview-empty", "No follow-ups match these filters.", "other client's empty follow-up list stays separate")
    driver.fill("#follow-up-search", " ")
    driver.wait_value("#follow-up-search", " ", "explicitly cleared client search")
    driver.select("#follow-up-view", "Upcoming (after today)")
    driver.select("#follow-up-kind", "Appointments")
    driver.click_button("Apply follow-up filters")
    driver.wait_text(".overview-basis", "1 matching item", "upcoming appointment filter")
    driver.wait_text(".overview-table", "09:30", "explicit local appointment time is preserved")
    driver.scroll_to(".follow-up-overview", "start")
    # Wait for paint only; no DOM values, focus or application state are changed.
    driver.command("POST", "/execute/async", {"script": "const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));", "args": []})
    driver.screenshot(artifacts / f"native-follow-up-overview-{label}.png")
    driver.click(".overview-open")
    driver.wait_value("#follow-title-0", "Review fictional statements", "dashboard selection opens the exact saved appointment")
    driver.wait(lambda:driver.observe_script("return document.activeElement.id") == "follow-title-0", "selected appointment receives keyboard focus")
    status(f"practice overview counts, closed history, client/kind/date filters and exact client/task navigation ({label})")


def main() -> int:
    repository = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default=str(repository / "src-tauri/target/debug/eisenbart-counseling-suite"))
    parser.add_argument("--transport", choices=("direct", "tauri"), default="direct",
                        help="Linux native driver directly (default), or the Tauri forwarding layer for diagnosis")
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
    driver = WebDriver(options.port, [password, wrong_password], options.transport)
    process: subprocess.Popen[bytes] | None = None
    try:
        if not sys.platform.startswith("linux"):
            raise SmokeFailure("This smoke test requires Linux XDG storage isolation; use the documented Windows manual checks on Windows")
        binary = executable(options.binary, "Built desktop application")
        intermediary = executable(options.driver, "tauri-driver") if options.transport == "tauri" else None
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
            # Same native capability mapping and automation flags used by Tauri's
            # driver. Direct mode removes its pooled forwarding connection only.
            environment["TAURI_AUTOMATION"] = "true"
            environment["TAURI_WEBVIEW_AUTOMATION"] = "true"
            with (artifacts / "driver.log").open("wb") as log:
                def launch_driver() -> None:
                    nonlocal process
                    command = ([str(native), f"--port={options.port}", "--host=127.0.0.1"]
                               if options.transport == "direct" else
                               [str(intermediary), "--port", str(options.port),
                                "--native-port", str(options.native_port), "--native-driver", str(native)])
                    process = subprocess.Popen(command, env=environment, stdout=log,
                                               stderr=log, start_new_session=True)

                def ready() -> bool:
                    if process is None or process.poll() is not None:
                        raise SmokeFailure(f"Native test driver exited; see {artifacts / 'driver.log'}")
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
                        if driver.visible(".client-care [role=alert]"):
                            print("Intake diagnostic: " + driver.safe(driver.text(".client-care [role=alert]")), flush=True)
                    except SmokeFailure:
                        pass
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
