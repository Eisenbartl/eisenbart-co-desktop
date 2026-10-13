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
            raise DriverFailure("connection error", self.safe(f"WebDriver unavailable: {error}")) from None
        value = result.get("value", result)
        if isinstance(value, dict) and "error" in value:
            code = str(value["error"])
            detail = self.safe(str(value.get("message", code)))
            raise DriverFailure(code, f"WebDriver {code}: {detail}")
        return value

    def command(self, method: str, path: str, payload: object = None) -> object:
        if self.session_id is None:
            raise SmokeFailure("No native desktop session is open")
        return self.request(method, f"/session/{self.session_id}{path}", payload)

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

    def find(self, selector: str, using: str = "css selector") -> str:
        value = self.command("POST", "/element", {"using": using, "value": selector})
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
        option = self.command("POST", f"/element/{select}/element", {
            "using": "xpath", "value": f"./option[normalize-space(.)={json.dumps(value)}]",
        })
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


def run_checks(driver: WebDriver, binary: Path, password: str,
               wrong_password: str, artifacts: Path, restart_driver: Callable[[], None]) -> None:
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
    if driver.script("return Boolean(window.__TAURI_INTERNALS__)") is not True:
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
    create_client(second_name, second_last, "Create a fictional spending plan")
    driver.wait_text(".notes-history", "No session notes yet.", "second client's empty note history")
    if note in driver.text(".notes-history"):
        raise SmokeFailure("The first client's note appeared in another client's record")
    status("two clients and isolated counseling notes")

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
                      first_goal, edited_goal, note, organization, counselor]

    def assert_locked() -> None:
        markup = str(driver.script("return document.documentElement.outerHTML"))
        if any(value in markup for value in private_values):
            raise SmokeFailure("Private workspace data remained in the DOM after locking")
        if driver.script("return document.querySelector('#password').value") != "":
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
    status("manual lock clears private data; incorrect password rejected; correct unlock")

    # Verify the application is responsive before a deliberate process stop.
    # Restart the native driver too: WebKitGTK session deletion/reuse can race
    # its browser shutdown, independent of application storage correctness.
    if driver.script("return Boolean(window.__TAURI_INTERNALS__)") is not True:
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
    driver.open_client("Robin Synthetic")
    driver.wait_text(".editor-panel", "Robin Synthetic", "persisted second client")
    driver.wait_text(".notes-history", "No session notes yet.", "persisted second client's empty note history")
    if note in driver.text(".notes-history"):
        raise SmokeFailure("Persisted notes crossed client boundaries after relaunch")
    status("full app relaunch preserves profiles, clients, edits and isolated notes")


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
                    run_checks(driver, binary, password, wrong_password, artifacts, restart_driver)
                except (SmokeFailure, OSError, ValueError):
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
