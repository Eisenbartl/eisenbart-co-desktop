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
import re
from pathlib import Path
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import traceback
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
        self.last_client_id: str | None = None
        self.assessment_periods: dict[str, tuple[str,str]] = {}
        self.restoring_period = False
        self.restoring_record = False
        self.record_intents: dict[tuple[str,str],str] = {}

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
        if block not in ("center", "start", "nearest"):
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
        if selector=='.page-heading' and not self.visible(selector):selector='.client-workspace-header'
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
        if using=="css selector":self.workflow_for(selector)
        return self.wait(lambda: self.visible(selector, using), f"visible {selector}")

    def interactive(self, selector: str, using: str = "css selector") -> str:
        if selector=='#workspace-client':selector='.workspace-app-tab.active [data-client-tab]'
        if using=='css selector':self.workflow_for(selector)
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
        match=re.fullmatch(r"nav\[aria-label='Main navigation'\] button\[aria-label='([^']+)'\]",selector)
        if match and match.group(1) not in ['Dashboard','Clients','Settings']:
            self.client_section(match.group(1));return
        if selector=='.page-heading button' and not self.visible(selector):
            self.click("nav[aria-label='Main navigation'] button[aria-label='Clients']")
        if selector=='.page-heading' and not self.visible(selector):selector='.client-workspace-header'
        if self.visible('.workspace-app-tab.active [data-client-tab]'):
            self.last_client_id=str(self.observe_script("return document.querySelector('.workspace-app-tab.active [data-client-tab]')?.dataset.clientTab"))
            for control in ['#saved-assessment','#saved-action-plan']:
                value=self.observe_script('return document.querySelector(arguments[0])?.value',[control])
                if isinstance(value,str) and value:self.record_intents[(self.last_client_id,control)]=value
        if using=='css selector':self.workflow_for(selector)
        element = self.interactive(selector, using)
        # Sticky workspace tabs should not be vertically centered: WebKit can
        # retain an obsolete sticky-layer hit region after that forced scroll.
        # Observe the real hit target before one native click; never retry writes.
        is_tab=self.command("GET",f"/element/{element}/attribute/role")=="tab"
        if is_tab:
            contained=self.observe_script("const e=arguments[0],r=e.getBoundingClientRect(),p=e.closest('.workspace-tabbar')?.getBoundingClientRect();return !!p&&r.left>=p.left&&r.right<=p.right&&r.top>=p.top&&r.bottom<=p.bottom;",[{ELEMENT_KEY:element}])
            if not contained:self.scroll_element(element,"nearest")
            self.command("POST","/execute/async",{"script":"const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));","args":[]})
            self.wait(lambda:self.observe_script("const e=arguments[0],r=e.getBoundingClientRect(),h=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return !!h&&(h===e||e.contains(h));",[{ELEMENT_KEY:element}]) is True,"painted and unobstructed workspace tab")
        else:self.scroll_element(element)
        self.command("GET", f"/element/{element}/displayed")
        try:
            self.command("POST", f"/element/{element}/click", {})
        except DriverFailure as error:
            # Add read-only geometry diagnostics; never replay an uncertain click.
            bounds=self.observe_script("const r=arguments[0].getBoundingClientRect();const h=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return {x:r.x,y:r.y,width:r.width,height:r.height,disabled:arguments[0].matches(':disabled'),hit:h?.tagName,hitClass:h?.className};",[{ELEMENT_KEY:element}])
            raise SmokeFailure(f"Native click failed for {selector}: {error}; geometry={json.dumps(bounds)}") from None

    def click_button(self, text: str) -> None:
        locator=f"//button[normalize-space(.)={json.dumps(text)}]"
        if text.startswith('Review card row ') and not self.visible(locator,'xpath'):
            self.client_section('Review & reconciliation');self.click_button('Card transactions')
        if text.startswith('Review payments for ') and not self.visible(locator,'xpath'):
            self.client_section('Review & reconciliation');self.click_button('Recurring payments')
        if text.startswith('Reverify prior classification for ') and not self.visible(locator,'xpath'):
            self.element('.financial-facts-editor > summary')
            node=self.find('.financial-facts-editor')
            if not self.command('GET',f'/element/{node}/property/open'):self.click('.financial-facts-editor > summary')
        if re.fullmatch(r'Review .+\.(csv|pdf) record \d+',text,re.IGNORECASE) and not self.visible(f"//button[normalize-space(.)={json.dumps(text)}]",'xpath'):self.client_section('Review & reconciliation');self.click_button('Review queue')
        if text.startswith('Review balance source record ') and not self.visible(locator,'xpath'):
            self.client_section('Review & reconciliation');self.click_button('Running balances')
            choose_balance_statement(self)
        local_areas={
            'Session notes':'Session notes','History & records':'Session notes',
            'Profile':'Profile & intake','Intake & follow-ups':'Profile & intake',
            'Card transactions':'Review & reconciliation','Review queue':'Review & reconciliation','Verified figures':'Review & reconciliation',
            'Running balances':'Review & reconciliation','Recurring payments':'Review & reconciliation',
            'Transfer matching':'Review & reconciliation','Source checks':'Review & reconciliation',
            'Classification assistance':'Review & reconciliation','Category rules':'Review & reconciliation',
            'Activity history':'Review & reconciliation','Bank & card activity':'Review & reconciliation',
            'Card completeness':'Review & reconciliation','Completeness & final validation':'Review & reconciliation',
            'Final report checklist':'Assessments & reports','Print & presentation':'Assessments & reports',
            'Assessments & action plans':'Assessments & reports',
        }
        if text in local_areas and not self.visible(locator,'xpath'):
            self.client_section(local_areas[text])
        if text=='Working tools':
            current=self.observe_script("return document.querySelector('nav[aria-label=\"Client workspace sections\"] [aria-current=page]')?.getAttribute('aria-label')")
            text={'Import statements':'Import & sources','Review & reconciliation':'Review queue','Financial dashboard':'Approved activity','Assessments & reports':'Assessments & action plans'}.get(str(current),text)
        route={'Add income':'Monthly budget','Add debt':'Debt payoff','Add cash-flow account':'Cash-flow calendar','Add dated activity':'Cash-flow calendar','Add follow-up':'Profile & intake'}
        if text in route and not self.visible(f"//button[normalize-space(.)={json.dumps(text)}]",'xpath'):self.client_section(route[text])
        self.click(f"//button[normalize-space(.)={json.dumps(text)}]", "xpath")

    def client_section(self, label: str, force: bool = False) -> None:
        """Navigate through actual client tabs/rail; never invoke application IPC.

        Old regression fixtures describe logical workflows that used to share a
        single record page. These page-object routes now open their focused view.
        Read-only visible/text/observe assertions never route or mutate anything.
        """
        aliases = {'Documents':'Import statements','Transactions':'Review & reconciliation',
                   'Financial Review':'Financial dashboard','Budget Planner':'Monthly budget',
                   'Debt Planner':'Debt payoff','Savings Planner':'Savings & sinking funds',
                   'Cash-Flow Calendar':'Cash-flow calendar','Financial Scenarios':'Scenario comparisons',
                   'Reports':'Assessments & reports'}
        label=aliases.get(label,label)
        if not self.visible("nav[aria-label='Client workspace sections']"):
            selector=(f"[role=tab][data-client-tab={json.dumps(self.last_client_id)}]" if self.last_client_id else '.workspace-app-tab:last-child [role=tab]')
            if self.visible(selector):
                self.click(selector)
            elif self.last_client_id:
                # Lock/restart intentionally clears every private in-app tab.
                # Reopen the fixture's exact retained ID through the directory,
                # rather than assuming an old global module selects a client.
                self.click("nav[aria-label='Main navigation'] button[aria-label='Clients']")
                row=f"//table[contains(@class,'client-directory-table')]//tr[td[normalize-space(.)={json.dumps(self.last_client_id)}]]//button"
                self.click(row,'xpath')
            else:
                raise SmokeFailure('A client must be explicitly opened before selecting '+label)
            self.element("nav[aria-label='Client workspace sections']")
        button=f"nav[aria-label='Client workspace sections'] button[aria-label={json.dumps(label)}]"
        element=self.element(button)
        current=self.command('GET',f'/element/{element}/attribute/aria-current')
        if force or current!='page':self.click(button)
        if self.visible('[role=alertdialog]'):raise SmokeFailure('Unsaved changes block routing to '+label+'; the fixture must choose Keep editing or Discard changes explicitly.')
        self.last_client_id=str(self.observe_script("return document.querySelector('.workspace-app-tab.active [data-client-tab]')?.dataset.clientTab"))

    def workflow_for(self, selector: str) -> None:
        # An intended action/wait may choose its logical workflow once. Locator
        # polling and negative/privacy observations remain strictly read-only.
        if self.restoring_period or self.restoring_record or self.visible(selector):return
        routes=[
          (('#first-name','#last-name','#email','#phone','#household-size','#client-status','#goals','.editor-panel'),'Profile & intake','Profile'),
          (('.client-care','#intake-','#follow-','.care-'),'Profile & intake','Intake & follow-ups'),
          (('#new-note','.notes-history','.notes-panel','.session-note','.note-correction'),'Session notes','Session notes'),
          (('.budget-','#budget-'),'Monthly budget',None),
          (('.debt-panel','#debt-','.debt-results'),'Debt payoff',None),
          (('.savings-panel','#savings-','.savings-results'),'Savings & sinking funds',None),
          (('.cashflow-panel','#cashflow-','.cashflow-'),'Cash-flow calendar',None),
          (('.combined-scenarios','#scenario-','.saved-scenario'),'Scenario comparisons',None),
          (('#csv-','#statement-format','.pdf-line-selection'),'Import statements',None),
          (('.financial-dashboard','#assessment-from','#assessment-through','.assessment-totals','.assessment-basis','.assessment-review','.classified-totals','.verified-balance-totals','.verified-ratios','.verified-fact-metrics'),'Financial dashboard',None),
          (('#card-row-','.card-transaction-review','.card-review-history'),'Review & reconciliation','Card transactions'),
          (('.financial-facts','#facts-','#fact-','.facts-classifications'),'Review & reconciliation','Verified figures'),
          (('.statement-balance','#balance-','.balance-'),'Review & reconciliation','Running balances'),
          (('.recurring-','#recurring-'),'Review & reconciliation','Recurring payments'),
          (('.transfer-review','#transfer-'),'Review & reconciliation','Transfer matching'),
          (('.imports-sources',),'Review & reconciliation','Source checks'),
          (('.imports-table','.imports-totals','.imports-progress'),'Review & reconciliation','Review queue'),
          (('.saved-assessment','#saved-assessment','#record-','.captured-','#assessment-pdf','.client-action','#action-','.saved-client-action','#saved-action-plan','#client-plan-pdf','.action-plan-status'),'Assessments & reports','Assessments & action plans'),
        ]
        for prefixes,label,area in routes:
            if selector.startswith(prefixes):
                self.client_section(label)
                if area:
                    if label in ['Profile & intake','Session notes']:
                        selector=f"//nav[@aria-label='Client record sections']//button[normalize-space(.)={json.dumps(area)}]"
                        tab=self.element(selector,'xpath')
                        if self.command('GET',f'/element/{tab}/attribute/aria-current')!='page':self.click(selector,'xpath')
                    else:self.click_button(area)
                if label=='Financial dashboard' and self.last_client_id in self.assessment_periods:
                    # A focused view remounts when returning. Re-select the
                    # fixture's intended period through real controls, scoped
                    # by stable client ID; do not assume product persistence.
                    period=self.assessment_periods[self.last_client_id]
                    self.wait(lambda:self.visible('#assessment-from'),'financial period controls')
                    self.restoring_period=True
                    try:view_financial_dashboard(self,*period)
                    finally:self.restoring_period=False
                needs_record=selector.startswith(('#record-','.saved-assessment-view','.saved-assessment-status','.captured-','#assessment-pdf','.client-action-plans','#saved-action-plan','#action-','.saved-client-action','#client-plan-pdf','.action-plan-status'))
                record=self.record_intents.get((self.last_client_id,'#saved-assessment'))
                if label=='Assessments & reports' and needs_record and record:
                    self.wait(lambda:self.visible('.saved-assessments > summary'),'saved report controls')
                    node=self.find('.saved-assessments')
                    if not self.command('GET',f'/element/{node}/property/open'):
                        # Reopen the same immutable saved ID through actual
                        # dropdown options after crossing focused sections.
                        self.restoring_record=True
                        try:
                            self.click('.saved-assessments > summary')
                            select=self.interactive('#saved-assessment')
                            self.scroll_element(select)
                            option=self.command('POST',f'/element/{select}/element',{'using':'xpath','value':f'./option[@value={json.dumps(record)}]'})
                            self.command('POST',f"/element/{option[ELEMENT_KEY]}/click",{})
                            self.wait(lambda:self.visible('.saved-assessment-view'),'selected saved report')
                        finally:self.restoring_record=False
                plan=self.record_intents.get((self.last_client_id,'#saved-action-plan'))
                needs_plan=selector.startswith(('.saved-client-action','#client-plan-pdf','.action-plan-status'))
                if label=='Assessments & reports' and needs_plan and plan and not self.visible('.saved-client-action-plan'):
                    self.restoring_record=True
                    try:
                        self.wait(lambda:self.visible('.client-action-plans > summary'),'saved client plan controls')
                        node=self.find('.client-action-plans')
                        if not self.command('GET',f'/element/{node}/property/open'):self.click('.client-action-plans > summary')
                        select=self.interactive('#saved-action-plan')
                        self.scroll_element(select)
                        option=self.command('POST',f'/element/{select}/element',{'using':'xpath','value':f'./option[@value={json.dumps(plan)}]'})
                        self.command('POST',f"/element/{option[ELEMENT_KEY]}/click",{})
                        self.wait(lambda:self.visible('.saved-client-action-plan'),'selected saved client plan')
                    finally:self.restoring_record=False
                if selector.startswith(('#facts-','#fact-','.facts-classifications')):
                    self.wait(lambda:self.visible('.financial-facts-editor > summary'),'verified figure controls')
                    node=self.find('.financial-facts-editor')
                    if not self.command('GET',f'/element/{node}/property/open'):self.click('.financial-facts-editor > summary')
                return

    def open_client(self, name: str) -> None:
        self.click("nav[aria-label='Main navigation'] button[aria-label='Clients']")
        self.click(f"//button[@aria-label={json.dumps('Open workspace for '+name)}]",'xpath')
        self.wait_text('.client-workspace-header h1',name,'scoped client workspace opened')
        self.last_client_id=str(self.observe_script("return document.querySelector('.workspace-app-tab.active [data-client-tab]')?.dataset.clientTab"))

    def fill(self, selector: str, value: str) -> None:
        self.workflow_for(selector)
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

    def replace_text(self, selector: str, value: str) -> None:
        # Replace an existing controlled value with actual keyboard events.
        # WebKit's clear endpoint can lose focus between clearing and typing.
        # Focus first, clear once, recheck effective enabled state, then type once.
        # No uncertain mutation is retried and no application data is scripted.
        self.click(selector)
        element = self.interactive(selector)
        self.command("POST", f"/element/{element}/value", {"text": "\ue009a\ue000\ue003"})
        self.wait_value(selector, "", "keyboard-cleared controlled field")
        element = self.interactive(selector)
        self.command("POST", f"/element/{element}/value", {"text": value})
        self.wait_value(selector, value, "complete controlled-field replacement")

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
        if selector=='#workspace-client':
            label=self.observe_script("return document.querySelector('nav[aria-label=\"Client workspace sections\"] [aria-current=page]')?.getAttribute('aria-label')")
            area=self.observe_script("return document.querySelector('.client-workspace-content .section-navigation [aria-current=page]')?.textContent")
            self.open_client(value)
            if label:self.client_section(str(label))
            if area and self.visible(f"//button[normalize-space(.)={json.dumps(str(area))}]",'xpath'):self.click_button(str(area))
            return
        if using=='css selector':self.workflow_for(selector)
        # WebDriver clicks the HTML option rather than calling application code.
        select = self.interactive(selector, using)
        self.scroll_element(select)
        try:
            option = self.command("POST", f"/element/{select}/element", {
                "using": "xpath", "value": f"./option[normalize-space(.)={json.dumps(value, ensure_ascii=False)}]",
            })
        except DriverFailure as error:
            available = self.command("GET", f"/element/{select}/property/textContent")
            raise SmokeFailure(f"Could not select {value!r} in {selector}; choices={available!r}: {error}") from None
        if not isinstance(option, dict) or ELEMENT_KEY not in option:
            raise SmokeFailure("Native driver did not return the requested workflow option")
        try:
            self.command("POST", f"/element/{option[ELEMENT_KEY]}/click", {})
        except DriverFailure as error:
            raise SmokeFailure(f"Native selection failed for {selector} ({value!r}): {error}") from None

    def field_value(self, selector: str) -> str:
        element = self.find(selector)
        return str(self.command("GET", f"/element/{element}/property/value"))

    def wait_value(self, selector: str, value: str, description: str) -> None:
        self.workflow_for(selector)
        self.wait(lambda: self.field_value(selector) == value, description)

    def text(self, selector: str = "body") -> str:
        element = self.find(selector)
        return str(self.command("GET", f"/element/{element}/text"))

    def wait_text(self, selector: str, value: str, description: str) -> None:
        if selector=='.financial-facts-editor':
            self.element('.financial-facts-editor > summary')
            node=self.find('.financial-facts-editor')
            if not self.command('GET',f'/element/{node}/property/open'):self.click('.financial-facts-editor > summary')
        if selector=='.imports-panel' and value=='No imported transactions yet':self.client_section('Review & reconciliation')
        self.workflow_for(selector)
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
    driver.wait_text(".pdf-line-selection", "Fictional payroll", "real PDF parser process returns selectable source text")
    driver.wait_text(".pdf-selected-summary", "Selected transactions: 2 / 500", "clear transaction candidates start selected")
    for merchant in ['Fictional payroll','Fictional groceries']:
        box=driver.find(f"//table[contains(@class,'pdf-candidates')]//tr[td//strong[normalize-space(.)={json.dumps(merchant)}]]//input[@type='checkbox']",'xpath')
        if driver.command("GET",f"/element/{box}/property/checked") is not True:raise SmokeFailure("Clear PDF transaction was not prechecked")
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
    driver.wait_text(".assessment-totals","Operating cash flow$199.99", "dashboard recomputes corrected approval at the reselected period")
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
    driver.wait_text(".assessment-totals","Operating cash flow$500.00", "dashboard excludes reversed expense at the reselected period")
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
    if driver.command("GET", f"/element/{driver.element('.saved-assessments')}/property/open") is not True:
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
    if driver.command("GET", f"/element/{driver.element('.saved-assessments')}/property/open") is not True:
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
    # Complete all ephemeral proposal checks before navigating to saved plans.
    driver.wait_value("#budget-amount-0", "1500.00", "combined preview does not write saved income")
    driver.wait_value("#debt-monthly-payment", "100.00", "combined preview does not write saved debt")
    driver.wait_value("#savings-emergencyContribution", "200.00", "combined preview does not write saved savings")
    driver.wait_value("#cashflow-date-0", "2026-11-03", "combined preview does not write saved dated schedules")
    driver.wait_value("#savings-goal-monthlyContribution-0", "50.00", "goal proposal leaves the live contribution unchanged")
    driver.wait_value("#debt-monthly-payment", "100.00", "extra payment never changes the live base payment")
    driver.wait_value("#budget-amount-0", "1500.00", "new expense does not change the live budget")
    driver.wait_value("#budget-amount-0", "1500.00", "temporary income never changes the saved normal income")
    if driver.visible("#budget-amount-6"):
        raise SmokeFailure("New proposal expense unexpectedly entered the live budget")
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
    driver.click(".combined-scenarios > details > summary")
    driver.click(".saved-assessments > summary")
    driver.select("#saved-assessment", "Alex temporary income report — 2026-09-01 to 2026-10-31 — Needs review")
    driver.wait_text(".selected-combined-scenario", "$758.34", "immutable extended report restored")
    driver.wait_text(".selected-combined-scenario .income-period-results", "-$975.00", "original reduced-income deficit restored")
    export_scenario_controls_pdf(driver, artifacts, label)
    driver.click(".saved-assessments > summary")
    driver.wait_value("#savings-goal-monthlyContribution-0", "50.00", "reopened goal proposal never applies live changes")
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
    driver.click(".combined-scenarios > details > summary")
    driver.click(".saved-assessments > summary")
    driver.select("#saved-assessment", "Alex selected combined report — 2026-09-01 to 2026-10-31 — Needs review")
    driver.wait_text(".selected-combined-scenario", "$983.34", "linked comparison restored and verified")
    driver.wait_text(".saved-assessment-view .scenario-budget-totals", "Income$3,466.67", "original proposed model set retained")
    export_saved_combined_pdf(driver, artifacts, label)
    driver.click(".saved-assessments > summary")
    driver.wait_value("#budget-amount-0", "1500.00", "reopened proposal never applies live income")
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
    save_client_action_plan(driver, artifacts)
    if driver.command("GET", f"/element/{driver.element('.client-care')}/property/open") is not True:
        driver.click(".client-care > summary")
    driver.replace_text("#intake-employer", "Alex changed employer")
    driver.wait_value("#intake-employer", "Alex changed employer", "selected employer replacement")
    driver.click_button("Save intake and follow-ups")
    driver.wait_text(".saved-assessment-status", "Needs review", "selected intake change flags report")
    if driver.command("GET", f"/element/{driver.element('.client-care')}/property/open") is not True:
        driver.click(".client-care > summary")
    driver.interactive("#intake-employer")
    driver.replace_text("#intake-employer", "Alex fictional employer")
    driver.wait_value("#intake-employer", "Alex fictional employer", "selected employer restored")
    driver.click_button("Save intake and follow-ups")
    driver.interactive("#record-title")
    driver.wait_text(".saved-assessment-status", "Needs review", "selected intake reversion never reactivates historical report")
    driver.scroll_to(".saved-assessment-view .report-intake-context", "start")
    driver.command("POST", "/execute/async", {"script":"const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));", "args":[]})
    driver.screenshot(artifacts / "native-selected-report-intake.png")
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
    if driver.last_client_id:driver.assessment_periods[driver.last_client_id]=(start,end)


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
    choose_balance_statement(driver)
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
        driver.wait_text(".client-workspace-header h1", f"{given} {family}", "saved client profile")

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
    driver.wait_text(".client-workspace-header h1", "Robin Synthetic", "persisted second client")
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
    check_transaction_history(driver, artifacts, data_root, password, binary, restart_driver)
    check_report_presentation(driver, artifacts, data_root, password, binary, restart_driver)
    check_category_assistance(driver, artifacts, data_root, password, binary, restart_driver)
    check_category_rules(driver, artifacts, data_root, password, binary, restart_driver)
    check_client_history(driver, artifacts, data_root, password, binary, restart_driver)
    check_spending_analysis(driver, artifacts, data_root, password, binary, restart_driver)
    check_statement_extraction(driver, artifacts, data_root, password, binary, restart_driver)
    check_unified_review(driver, artifacts, data_root, password, binary, restart_driver)
    check_households(driver, artifacts, data_root, password, binary, restart_driver)
    check_inactivity_policy(driver, artifacts, data_root, password, binary, restart_driver)
    check_password_change(driver, artifacts, data_root, password, binary, restart_driver)


def check_unified_review(driver: WebDriver, artifacts: Path, data_root: Path,
                         password: str, binary: Path, restart_driver: Callable[[], None]) -> None:
    """One current ledger, real original PDF canvas, shared decisions and encrypted recovery."""
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Clients']")
    driver.click_button('Add client');driver.fill('#first-name','Taylor');driver.fill('#last-name','Review');driver.click_button('Create client')
    driver.wait_text('.client-workspace-header','Taylor Review','dedicated unified review client')
    bank=data_root/'unified-bank.pdf';bank.write_bytes(build_pdf([
        ['Fictional unified financial institution','Questions? Please contact us'],
        ['Statement period 2026-09-01 to 2026-09-30','Date Description Amount Balance','Opening balance 0.00',
         '2026-09-01 Fictional unified payroll 1000.00 1000.00','2026-09-02 Fictional debit purchase -100.00 900.00',
         '2026-09-03 Fictional unknown purchase -50.00 850.00','2026-09-06 Fictional loan proceeds 500.00 1350.00','2026-09-07 Fictional loan payment -500.00 850.00','2026-09-08 Fictional reversed purchase 25.00 875.00','2026-09-09 Fictional institution fee -25.00 850.00','Closing balance 850.00']]))
    driver.client_section('Import statements');driver.select('#statement-format','Selectable-text PDF');driver.fill('#csv-path',str(bank));driver.click_button('Inspect PDF')
    driver.wait_text('.pdf-selected-summary','Selected transactions: 7 / 500','seven financial rows selected')
    for selector,value in [('#csv-account-label','Fictional unified checking'),('#csv-periodStart','2026-09-01'),('#csv-periodEnd','2026-09-30'),('#csv-openingBalance','0'),('#csv-closingBalance','850')]:driver.fill(selector,value)
    driver.click("//label[contains(.,'Authorize retaining the complete original PDF')]/input",'xpath');driver.click_button('Import into review queue');driver.wait_text('.imports-panel > .success-message','Every PDF record needs individual verification.','bank source captured')
    driver.client_section('Review & reconciliation');driver.click_button('Review queue')
    for number,kind,category,purpose in [(1,'Income','Wages',''),(2,'Expense','Food','Necessary spending'),(3,'Expense','Shopping','Discretionary spending'),(4,'Balance movement / external transfer / principal payment','Debt',''),(5,'Balance movement / external transfer / principal payment','Debt',''),(6,'Refund','Food',''),(7,'Fee','Fees','Necessary spending')]:
        driver.click_button(f'Review {bank.name} record {number}');driver.select('#review-kind',kind);driver.fill('#review-category',category);driver.select('#review-status','Approved')
        if purpose:driver.select('#review-spending-role',purpose)
        driver.fill('#review-reason','Verified original fictional source and discussed spending purpose');driver.click('#review-ack');driver.click_button('Save transaction decision');driver.wait_text('.imports-panel > .success-message','Transaction decision saved','bank row approved explicitly');driver.interactive('.imports-table tbody button')
    card=data_root/'unified-card.pdf';card.write_bytes(build_pdf([['Billing period 09/01/2026 to 09/30/2026','Purchases','09/02 Fictional unified card purchase 100.00','Payments and credits','09/05 Fictional unified card payment 50.00 CR'],['Questions? Please contact us','Total purchases 100.00']]))
    driver.client_section('Import statements');driver.click_button('Credit-card statements');driver.click('.card-import > summary');driver.select('#card-source-format','Selectable-text PDF');driver.fill('#card-csv-path',str(card));driver.click_button('Inspect card PDF')
    driver.wait_text('.pdf-selected-summary','Selected transactions: 2 / 200','card source rows selected')
    for selector,value in [('#card-account','Fictional unified credit'),('#card-period-start','2026-09-01'),('#card-period-end','2026-09-30'),('#card-opening','0'),('#card-closing','50')]:driver.fill(selector,value)
    driver.click('#card-retain');driver.click_button('Save card statement for individual review');driver.wait_text('.card-progress','Needs Clarification 2','card source captured')
    driver.client_section('Review & reconciliation');driver.click_button('Card transactions')
    for row,kind,category,purpose in [(1,'purchase','Food','Necessary spending'),(2,'payment','Card payment','')]:
        driver.click_button(f'Review card row {row}');driver.select('#card-row-kind',kind);driver.fill('#card-row-category',category);driver.select('#card-row-status','Approved')
        if purpose:driver.select('#card-row-purpose',purpose);driver.interactive('#card-row-reason')
        driver.fill('#card-row-reason','Verified fictional card source; payment is not another purchase');driver.click('#card-row-ack');driver.click_button('Save card transaction decision');driver.wait_text('.card-statements > .success-message','Card transaction decision saved','card decision saved')
    def current() -> None:
        driver.client_section('Review & reconciliation');driver.click_button('All transactions');driver.interactive('#unified-search');driver.wait_text('.unified-pagination','of 9 transactions','one current row for each source record')
    def inspect(description: str) -> None:
        driver.click(f"//table[contains(@class,'unified-table')]//tr[td/strong[normalize-space(.)={json.dumps(description)}]]//button",'xpath');driver.wait_text('.unified-extracted',description,'selected transaction evidence');driver.interactive("//button[normalize-space(.)='Correct / approve transaction']",'xpath')
    def save_details(reason: str) -> None:
        driver.fill('#inspection-reason',reason);driver.click('#inspection-ack');driver.click_button('Save inspection details');driver.wait_text('.unified-transactions > .success-message','Current totals and evidence have refreshed','supplemental details saved and refreshed');driver.interactive('#unified-search')
    def method_analysis() -> None:
        if not driver.observe_script("return document.querySelector('.unified-method-analysis')?.open===true"):driver.click('.unified-method-analysis > summary')
    def canvas_ready() -> None:
        driver.wait(lambda:driver.observe_script("return (()=>{const c=document.querySelector('.original-document canvas');if(!c||c.hidden||c.width<100||document.querySelector('.original-document [role=alert]'))return false;const d=c.getContext('2d').getImageData(0,0,c.width,c.height).data;let n=0;for(let i=0;i<d.length;i+=4)if(d[i+3]>0&&d[i]<180&&d[i+1]<180&&d[i+2]<180)n++;return n>200;})()"),'actual nonblank original PDF page rendered',45)
    current()
    dates=driver.observe_script("return Array.from(document.querySelectorAll('.unified-table tbody tr')).map(r=>r.cells[0].firstChild.textContent)")
    if dates!=['2026-09-09','2026-09-08','2026-09-07','2026-09-06','2026-09-05','2026-09-03','2026-09-02','2026-09-02','2026-09-01']:raise SmokeFailure('Current transactions did not sort chronologically across accounts')
    driver.select('#unified-order','Oldest first');driver.click_button('Apply transaction filters');driver.interactive('#unified-search')
    if driver.observe_script("return document.querySelector('.unified-table tbody tr').cells[0].firstChild.textContent")!='2026-09-01':raise SmokeFailure('Oldest-first sort was not applied')
    driver.click('.unified-more-filters > summary');driver.select('#unified-accountType','Credit card');driver.select('#unified-kind','purchase');driver.click_button('Apply transaction filters');driver.wait_text('.unified-pagination','of 1 transactions','combined account/type filter')
    driver.click_button('Reset transaction filters');driver.wait_text('.unified-pagination','of 9 transactions','reset restores combined table')
    method_analysis();driver.wait_text('.unified-method-analysis > summary','$250.00','payment excluded from purchase total')
    inspect('Fictional debit purchase');driver.click_button('Open original document');canvas_ready();driver.wait_value('#original-pdf-page','2','known source page navigation')
    driver.select('#original-pdf-zoom','75%');canvas_ready();driver.click_button('Previous PDF page');driver.wait_value('#original-pdf-page','1','PDF page control');canvas_ready();driver.click_button('Next PDF page');canvas_ready()
    driver.command('POST','/window/rect',{'width':1600,'height':1100});driver.scroll_to('.unified-inspection','start');driver.screenshot(artifacts/'native-unified-original.png')
    driver.click_button('Review account details');driver.fill('#inspection-institution','Fictional Unified Bank');driver.fill('#inspection-last-four','9876');save_details('Verified fictional institution and last four digits from original')
    driver.click_button('Edit method & posting date');driver.select('#inspection-method','Debit-card purchase');driver.fill_native_date('#inspection-posting-date','2026-09-03');save_details('Verified checking-linked debit card method and separate posting date')
    method_analysis();expected=['$100.00 · 40.00%','$100.00 · 40.00%','$50.00 · 20.00%']
    driver.wait(lambda:driver.observe_script("return Array.from(document.querySelectorAll('.unified-method strong')).slice(0,3).map(e=>e.textContent)")==expected,'exact credit/debit/unknown shares with unknown in denominator')
    driver.click_button('Close inspection')
    for description,method,kind in [('Fictional loan proceeds','ACH credit','balanceMovement'),('Fictional loan proceeds','External transfer','balanceMovement'),('Fictional loan payment','ACH debit','balanceMovement'),('Fictional reversed purchase','Reversal','refund')]:
        inspect(description);driver.click_button('Edit method & posting date');driver.select('#inspection-method',method);save_details('Verified channel without changing financial treatment of '+description);driver.wait_text('.unified-extracted',kind,'channel preserves the financial kind');driver.click_button('Close inspection')
    method_analysis();driver.wait_text('.unified-method-analysis > summary','$250.00','movements and refund remain excluded from purchase totals')
    inspect('Fictional debit purchase')
    status('unified review: current chronological bank/card rows, compound filters, real local PDF pages/zoom, scoped account details and exact purchase-method coverage; ACH/external channels and reversal preserve economic treatment')
    driver.click_button('Correct / approve transaction');driver.interactive('#review-status');driver.select('#review-spending-role','Discretionary spending');driver.fill('#review-reason','Discussed changed purpose; preserve original source and inactive inspection details');driver.click('#review-ack');driver.click_button('Save transaction decision')
    driver.wait_text('.unified-transactions > .success-message','Current totals and evidence have refreshed','shared bank decision refreshed current table')
    driver.wait_text('.unified-extracted','Unclassified method','earlier method binding made inactive after financial decision')
    driver.click_button('Original values & history');driver.wait_text('.unified-extracted','Verified checking-linked debit card method','earlier method reason retained');driver.wait_text('.transaction-activity','Discussed changed purpose','existing financial history reused')
    driver.click_button('Edit method & posting date');driver.select('#inspection-method','Debit-card purchase');driver.fill_native_date('#inspection-posting-date','2026-09-03');save_details('Reverified method after the new financial approval')
    driver.click_button('Close inspection');inspect('Fictional unknown purchase');driver.click_button('Correct / approve transaction');driver.interactive('#review-status');driver.select('#review-status','Rejected');driver.fill('#review-reason','Synthetic duplicate concern pending clarification');driver.click('#review-ack');driver.click_button('Save transaction decision');driver.wait_text('.unified-transactions > .success-message','Current totals and evidence have refreshed','rejection uses existing bank workflow')
    driver.click_button('Edit method & posting date');driver.click('#inspection-duplicate');save_details('Confirmed rejected duplicate as a synthetic review case')
    driver.click_button('Close inspection');driver.select('#unified-reconciliation','Confirmed duplicate (rejected)');driver.click_button('Apply transaction filters');driver.wait_text('.unified-pagination','of 1 transactions','confirmed duplicate distinguished from suspicion');driver.wait_text('.unified-table','Confirmed duplicate · excluded','confirmed exclusion remains visible')
    driver.click_button('Reset transaction filters');driver.wait_text('.unified-pagination','of 9 transactions','source rows not deleted');inspect('Fictional unknown purchase');driver.click_button('Correct / approve transaction');driver.interactive('#review-status');driver.select('#review-status','Approved');driver.select('#review-spending-role','Discretionary spending');driver.fill('#review-reason','Clarified synthetic concern: this is a separate purchase; reverse duplicate rejection');driver.click('#review-ack');driver.click_button('Save transaction decision');driver.wait_text('.unified-transactions > .success-message','Current totals and evidence have refreshed','reapproval preserves duplicate history and restores eligible figures')
    driver.click_button('Close inspection');inspect('Fictional unified card purchase');driver.click_button('Correct / approve transaction');driver.interactive('.unified-review-embedded #card-row-status');driver.select('#card-row-purpose','Discretionary spending');driver.interactive('#card-row-reason');driver.fill('#card-row-reason','Client discussed card purchase purpose; original card evidence unchanged');driver.click('#card-row-ack');driver.click_button('Save card transaction decision');driver.wait_text('.unified-transactions > .success-message','Current totals and evidence have refreshed','shared card purpose decision saved from unified inspection')
    driver.click_button('Close inspection');driver.client_section('Spending Analysis');driver.interactive('#spending-search');driver.fill('#spending-from','2026-09');driver.fill('#spending-through','2026-09');driver.click_button('Update analysis');driver.interactive('#spending-search');driver.wait_text('.sa-metrics','$250.00','combined spending receives existing financial decisions');driver.wait_value("select[aria-label='Type for Fictional debit purchase on 2026-09-02']",'discretionary','bank purpose propagated');driver.wait_value("select[aria-label='Type for Fictional unified card purchase on 2026-09-02']",'discretionary','card purpose propagated')
    current();method_analysis();driver.wait_text('.unified-method-analysis > summary','$250.00','repayments/refunds not doubled after amendments');driver.scroll_to('.unified-panel-heading','start');driver.screenshot(artifacts/'native-unified-transactions.png')
    driver.command('POST','/window/rect',{'width':800,'height':700});inspect('Fictional debit purchase');driver.click_button('Open original document');canvas_ready()
    if driver.observe_script('return document.documentElement.scrollWidth>document.documentElement.clientWidth'):raise SmokeFailure('Unified inspection overflows an 800px workspace')
    driver.scroll_to('.unified-inspection','start');driver.screenshot(artifacts/'native-unified-narrow.png');driver.command('POST','/window/rect',{'width':1200,'height':800});driver.click_button('Close inspection')
    status('unified review: shared bank/card purpose decisions, stale-detail invalidation, original history, confirmed/reversed duplicate handling, downstream analysis and narrow source inspection')
    folder=data_root/'unified-review-backups';folder.mkdir();firm=driver.text('.workspace-label strong')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']");driver.click_button('Backup & recovery');driver.fill('#backup-folder',str(folder));driver.fill('#backup-password',password);driver.click_button('Create encrypted backup');driver.wait_text('.backup-success','Encrypted backup saved.','unified evidence and detail history backed up');backup=driver.text('.backup-file-path')
    driver.click_button('Lock workspace');driver.wait_text('.auth-card','Welcome back','unified workspace locked')
    if driver.observe_script("return document.querySelector('.unified-transactions,.original-document')!==null"):raise SmokeFailure('Unified source data remained visible on lock')
    restart_driver();driver.open(binary);driver.fill('#password',password);driver.click_button('Unlock workspace');driver.open_client('Taylor Review');current();method_analysis();driver.wait_text('.unified-method-analysis > summary','$250.00','exact methods persist across restart')
    driver.click('.lock-button');driver.wait_text('.auth-card','Welcome back','lock before unified recovery');driver.click('.restore-panel > summary');driver.fill('#restore-file',backup);driver.fill('#restore-password',password);driver.click_button('Verify backup for recovery');driver.wait_text('.restore-review',firm,'unified metadata and source histories validated');driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace');driver.wait_text('.auth-card','Workspace restored. Unlock it','unified recovery activated');driver.fill('#password',password);driver.click_button('Unlock workspace');driver.open_client('Taylor Review');current();inspect('Fictional debit purchase');driver.click_button('Original values & history');driver.wait_text('.unified-extracted','Reverified method after the new financial approval','supplemental history restored');driver.click_button('Open original document');canvas_ready()
    driver.click_button('Close inspection');driver.open_client('Robin Synthetic');driver.client_section('Review & reconciliation');driver.click_button('All transactions');driver.wait_text('.unified-transactions','No current transactions match these filters.','other client has no source rows')
    if 'Fictional Unified Bank' in driver.text('.unified-transactions'):raise SmokeFailure('Unified account metadata crossed client scope')
    driver.open_client('Alex Fictional');driver.client_section('Profile & intake')
    status('unified review: actual original PDF and supplemental/financial history survive lock, full restart and encrypted recovery; client isolation preserved')


def check_households(driver: WebDriver, artifacts: Path, data_root: Path,
                     password: str, binary: Path, restart_driver: Callable[[], None]) -> None:
    """Native three-person allocations, payments, controlled handoff, tabs and encrypted recovery."""
    active = '[data-household-active="true"]'
    drawer = active + ' .household-drawer'
    def ready() -> None:
        driver.wait(lambda:driver.observe_script("return document.querySelector(arguments[0])?.getAttribute('aria-busy')",[active]) == 'false', 'household calculations ready')
    def section(label: str) -> None:
        driver.click(f"//*[@data-household-active='true']//nav[@aria-label='Household sections']//button[normalize-space(.)={json.dumps(label)}]", 'xpath')
        ready()
    def save_member(name: str, linked: bool, planning: bool = False) -> None:
        driver.click_button('Add member')
        if linked:
            label=driver.observe_script("return Array.from(document.querySelector(arguments[0]).options).find(o=>o.textContent.startsWith(arguments[1]+' ·'))?.textContent",[drawer+' select[data-field="linked-client"]',name])
            if not isinstance(label,str):raise SmokeFailure('Dedicated synthetic client is missing from member selector')
            driver.select(drawer+' select[data-field="linked-client"]',label)
        else:driver.fill(drawer+' input[data-field="member-name"]',name)
        if planning:
            driver.click(drawer+' input[data-consent="planning"]')
            driver.fill(drawer+' textarea','Synthetic household budget authorization only. No income, balances or private statements shared.')
        driver.click_button('Save member & permissions')
        driver.wait(lambda:not driver.visible(drawer),'member editor closed after successful save');ready()
        driver.wait_text(active+' .household-content',name,'saved household member')
    driver.command('POST','/window/rect',{'width':1536,'height':1024})
    for first in ['Jordan','Nora']:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Clients']")
        driver.click_button('Add client');driver.fill('#first-name',first);driver.fill('#last-name','Household');driver.click_button('Create client')
        driver.wait_text('.client-workspace-header h1',first+' Household','dedicated synthetic household client')
    driver.click('button[aria-label="Create a household workspace"]')
    driver.fill('.household-create input','Oak Native Household');driver.click_button('Save household')
    driver.wait_text(active+' .household-identity h2','Oak Native Household','household opened in top tab');ready()
    tab=driver.observe_script("return document.querySelector('.household-app-tab.active [role=tab]')?.dataset.clientTab")
    if not isinstance(tab,str):raise SmokeFailure('Household tab has no stable identifier')
    section('Members & Permissions');save_member('Jordan Household',True,True);save_member('Nora Household',True);save_member('Michael Demo',False)
    status('household: two stable client links, limited member and explicit permissions')
    section('Shared Expenses');driver.click_button('Add expense')
    driver.fill(drawer+' input[data-field="expense-name"]','Rent');driver.fill(drawer+' input[data-field="amount"]','2400.00')
    driver.replace_text(drawer+' input[aria-label="Fixed dollars for Jordan Household"]','500.00');driver.replace_text(drawer+' input[aria-label="Remainder weight for Jordan Household"]','0.0');driver.wait_text(drawer+' .household-preview','$500.00','decimal zero excludes the fixed-only participant');driver.wait_text(drawer+' .household-preview','$950.00','exact equal remainder for the other members');driver.replace_text(drawer+' input[aria-label="Fixed dollars for Jordan Household"]','0.00');driver.replace_text(drawer+' input[aria-label="Remainder weight for Jordan Household"]','1')
    driver.select(drawer+' select[data-field="allocation-method"]','Custom percentage remainder')
    for person,percent in [('Jordan Household','25'),('Nora Household','37.5'),('Michael Demo','37.5')]:
        driver.replace_text(drawer+f' input[aria-label="Remainder weight for {person}"]',percent)
    driver.wait_text(drawer+' .household-preview','$600.00','exact Jordan responsibility preview')
    driver.wait_text(drawer+' .household-preview','$900.00','exact remaining member shares')
    driver.click_button('Save expense agreement');driver.wait(lambda:not driver.visible(drawer),'agreement saved');ready()
    section('Payments & Settlements');driver.click_button('Record payment')
    driver.select(drawer+' select[data-field="payment-expense"]','Rent');driver.fill(drawer+' input[data-field="amount"]','2400.00');driver.select(drawer+' select[data-field="payment-state"]','Confirmed by counselor / source');driver.click_button('Save payment record');driver.wait(lambda:not driver.visible(drawer),'external payment saved');ready()
    driver.wait_text(active+' .household-content','$900.00','outstanding reimbursement suggestions')
    section('Overview');driver.wait_text(active+' .household-stats','$2,400.00','fronted payment is not duplicated spending');driver.wait_text(active+' .household-stats','Necessary $2,400.00 · Discretionary $0.00','overview necessary/discretionary plan breakdown')
    if '$4,200.00' in driver.text(active):raise SmokeFailure('Household double-counted the external payment and reimbursements')
    driver.scroll_to(active+' .household-page-heading','start');driver.screenshot(artifacts/'native-household-overview.png')
    section('Payments & Settlements');driver.click_button('Record payment')
    driver.select(drawer+' select[data-field="movement-kind"]','settlement');driver.select(drawer+' select[data-field="paying-member"]','Nora Household');driver.select(drawer+' select[data-field="receiving-member"]','Jordan Household');driver.fill(drawer+' input[data-field="amount"]','900.00');driver.select(drawer+' select[data-field="payment-state"]','Confirmed by counselor / source');driver.click_button('Save payment record');driver.wait(lambda:not driver.visible(drawer),'settlement saved');ready()
    section('Overview');driver.wait_text(active+' .household-stats','$2,400.00','internal settlement does not add spending')
    status('household: exact $600/$900/$900 agreement, fronted bill and confirmed internal settlement')
    section('Household Budget');driver.select(active+' .household-toolbar select','Jordan Household');driver.click_button('Preview budget reconciliation');driver.wait_text(active+' .household-content','Household shares to add','controlled budget preview');driver.wait_text(active+' .household-content','$600.00','only member share proposed')
    driver.click(active+' input[data-field="budget-ack"]');driver.click_button('Save reconciled individual budget');ready();driver.wait_text(active+' .household-notice','Individual budget saved','explicit budget revision')
    section('Goals & Scenarios');driver.click_button('Add shared goal');driver.fill(drawer+' input[data-field="goal-name"]','Shared emergency reserve');driver.fill(drawer+' input[data-field="goal-target"]','1200.00');driver.fill(drawer+' input[data-field="goal-monthly"]','100.00');driver.click_button('Save shared goal');driver.wait(lambda:not driver.visible(drawer),'shared savings goal saved');ready();driver.wait_text(active+' .household-content','$33.33','shared savings contribution cents');driver.wait_text(active+' .household-content','$33.34','deterministic leftover cent in shared savings')
    section('Shared Expenses');driver.click_button('Create revision');driver.replace_text(drawer+' input[data-field="amount"]','2600.00');driver.wait_text(drawer+' .household-preview','$650.00','hypothetical revised rent share');driver.click(drawer+' details > summary');driver.fill(drawer+' input[data-field="scenario-name"]','Future rent option');driver.click_button('Save expense scenario');driver.wait(lambda:not driver.visible(drawer),'hypothetical scenario saved');ready();section('Goals & Scenarios');driver.click_button('Preview impact');driver.wait_text(active+' .household-preview','$650.00','saved hypothetical impact preview');section('Overview');driver.wait_text(active+' .household-stats','$2,400.00','hypothetical proposal does not change active spending');driver.wait_text(active+' .household-content','Shared emergency reserve','goal visible on household overview');section('Household Budget')
    status('household: exact shared goal contributions and non-writing rent scenario preview')
    jordan="[role=tab][aria-label='Jordan Household']";driver.click(jordan);driver.client_section('Monthly budget');driver.wait_text('.budget-panel','$600.00','individual receives only agreed household share')
    driver.click(f'[role=tab][data-client-tab={json.dumps(tab)}]');ready();driver.wait_text(active+' h1','Household Budget','household last section remembered')
    section('Members & Permissions');driver.click_button('Add member');driver.fill(drawer+' input[data-field="member-name"]','Private retained household draft')
    driver.click(jordan);driver.wait_text('.budget-panel','$600.00','individual tab kept its working section')
    driver.click(f'[role=tab][data-client-tab={json.dumps(tab)}]');driver.wait_value(drawer+' input[data-field="member-name"]','Private retained household draft','household draft retained between tabs')
    driver.click(drawer+' button[aria-label="Close household editor"]');driver.click_button('Keep editing');driver.wait_value(drawer+' input[data-field="member-name"]','Private retained household draft','explicit keep draft')
    driver.click(drawer+' button[aria-label="Close household editor"]');driver.click_button('Discard changes');driver.wait(lambda:not driver.visible(drawer),'explicit draft discard')
    section('Reports & History');driver.click_button('Record report snapshot');ready();driver.wait_text(active+' .household-notice','snapshot recorded','immutable historical report')
    report_option=driver.observe_script("return Array.from(document.querySelector(arguments[0]).options).find(o=>o.value)?.textContent",[active+' .household-panel select'])
    driver.select(active+' .household-panel select',str(report_option));driver.element(active+' .household-report')
    pdf=artifacts/'household-native.pdf';driver.fill(active+' input[data-field="household-pdf-path"]',str(pdf));driver.click_button('Export saved household PDF');driver.wait_text(active+' .household-notice','Household PDF saved','offline household PDF export')
    content=pdf.read_bytes()
    if not content.startswith(b'%PDF-'):raise SmokeFailure('Household report is not a PDF')
    text=subprocess.run(['pdftotext',str(pdf),'-'],check=True,capture_output=True,text=True).stdout
    if '600.00' not in text or '2400.00' not in text or 'Shared emergency reserve' not in text:raise SmokeFailure('Household PDF lacks independently expected amounts or shared goal')
    driver.click_button('Export saved household PDF');driver.wait_text(active+' [role=alert]','already exists','household export refuses overwriting')
    if pdf.read_bytes()!=content:raise SmokeFailure('Household PDF bytes changed after refused overwrite')
    status('household: controlled budget handoff, tab sections/drafts and immutable no-overwrite PDF')
    driver.command('POST','/window/rect',{'width':800,'height':960});driver.scroll_to(active+' h1','start');driver.screenshot(artifacts/'native-household-narrow.png')
    if driver.observe_script('return document.documentElement.scrollWidth>document.documentElement.clientWidth'):raise SmokeFailure('Household screen overflows the native window')
    driver.command('POST','/window/rect',{'width':1536,'height':1024})
    driver.click_button('Lock workspace');driver.element('#password')
    if driver.visible(active):raise SmokeFailure('Household data remained visible after locking')
    restart_driver();driver.open(binary);driver.fill('#password',password);driver.click_button('Unlock workspace')
    def reopen() -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Clients']");driver.click_button('Households')
        driver.click("//button[contains(@class,'household-directory-card')][strong[normalize-space(.)='Oak Native Household']]",'xpath');ready()
        driver.wait_text(active+' .household-stats','$2,400.00','persisted household expenses')
        driver.wait_text(active+' .household-content','$600.00','persisted allocation shares')
    reopen();folder=data_root/'household-backup';folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']");driver.click_button('Backup & recovery');driver.fill('#backup-folder',str(folder));driver.fill('#backup-password',password);driver.click_button('Create encrypted backup');driver.wait_text('.backup-success','Encrypted backup saved.','household journal backed up');backup=driver.text('.backup-file-path')
    driver.click_button('Lock workspace');driver.click('.restore-panel > summary');driver.fill('#restore-file',backup);driver.fill('#restore-password',password);driver.click_button('Verify backup for recovery');driver.wait_text('.restore-review','Verified backup','household history verified before recovery');driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace');driver.wait_text('.auth-card','Workspace restored.','household backup activated');driver.fill('#password',password);driver.click_button('Unlock workspace');reopen()
    status('household: narrow layout, lock privacy, complete restart and encrypted recovery')
    driver.open_client('Alex Fictional')


def check_statement_extraction(driver: WebDriver, artifacts: Path, data_root: Path,
                               password: str, binary: Path, restart_driver: Callable[[], None]) -> None:
    """Actual PDF worker, visible selection/review, exact source-bound purposes and recovery."""
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Clients']")
    driver.click_button('Add client');driver.fill('#first-name','Morgan');driver.fill('#last-name','Extraction');driver.click_button('Create client')
    driver.wait_text('.client-workspace-header','Morgan Extraction','separate statement extraction client')
    path=data_root/'extracted.pdf'
    path.write_bytes(build_pdf([
      ['Fictional institution welcome','Questions? Please contact us','Your purchases may be subject to a fee of $25.00'],
      ['Statement period 2026-09-01 to 2026-09-30','Date Description Amount Balance','Opening balance 0.00',
       '2026-09-01 Fictional payroll 1000.01 1000.01','2026-09-02 Fictional groceries -25.11 974.90','2026-09-03 Fictional coffee -3.02 971.88','Ending balance 971.88'],
      ['Member FDIC','Questions? Please contact us','Total withdrawals 28.13']]))
    driver.client_section('Import statements');driver.select('#statement-format','Selectable-text PDF');driver.fill('#csv-path',str(path));driver.click_button('Inspect PDF')
    driver.wait_text('.pdf-selected-summary','Selected transactions: 3 / 500','only actual transaction rows start checked')
    driver.wait_text('.pdf-transaction-inspection','across 1 of 3 source pages','nonfinancial pages excluded from checklist')
    if driver.observe_script("return document.querySelectorAll('.pdf-candidates tbody tr').length")!=3:raise SmokeFailure('Fine print became a transaction candidate')
    if 'Questions?' in driver.text('.pdf-candidates'):raise SmokeFailure('Institution fine print entered transaction checklist')
    driver.wait_text('.pdf-selected-summary','Withdrawals / debits$28.13','running balances excluded from exact selected spending')
    toggle="//table[contains(@class,'pdf-candidates')]//tr[td//strong[normalize-space(.)='Fictional coffee']]//input[@type='checkbox']"
    if driver.command('GET',f'/element/{driver.find(toggle,"xpath")}/property/checked') is not True:raise SmokeFailure('Clear coffee transaction was not checked')
    driver.click(toggle,'xpath');driver.wait_text('.pdf-selected-summary','Selected transactions: 2 / 500','manual uncheck changes selection count')
    driver.wait_text('.pdf-selected-summary','Withdrawals / debits$25.11','manual uncheck changes exact debit total')
    driver.wait_text('.pdf-selected-summary','Net selected movement$974.90','manual uncheck updates net')
    if driver.observe_script("return getComputedStyle(document.querySelector('.pdf-transaction-inspection')).maxHeight")!='none':raise SmokeFailure('Transaction inspector is still clipped by the old raw-line scroll box')
    driver.command('POST','/window/rect',{'width':1600,'height':1100})
    driver.scroll_to('.pdf-selected-summary','start');driver.command('POST','/execute/async',{'script':'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));','args':[]});driver.screenshot(artifacts/'native-extracted-candidates.png')
    driver.command('POST','/window/rect',{'width':800,'height':700})
    if driver.observe_script('return document.documentElement.scrollWidth>document.documentElement.clientWidth'):raise SmokeFailure('PDF inspector overflows the narrow workspace')
    driver.command('POST','/window/rect',{'width':1200,'height':800})
    driver.fill('#csv-account-label','Fictional extracted savings');driver.select('#csv-account-kind','Savings')
    driver.fill('#csv-openingBalance','0');driver.fill('#csv-closingBalance','971.88')
    driver.click("//label[contains(.,'Authorize retaining the complete original PDF')]/input",'xpath')
    driver.click_button('Import into review queue');driver.wait_text('.imports-panel > .success-message','Every PDF record needs individual verification.','PDF capture remains separate from approval')
    driver.client_section('Review & reconciliation');driver.wait_text('.imports-progress','Needs Clarification 2','only checked transactions imported')
    if driver.observe_script("return document.querySelectorAll('.imports-table tbody tr').length")!=2:raise SmokeFailure('Unchecked transaction imported')
    def bank_decide(number:int,kind:str,category:str,purpose:str,reason:str)->None:
        driver.interactive(f"//button[normalize-space(.)={json.dumps(f'Review {path.name} record {number}')} ]",'xpath');driver.click_button(f'Review {path.name} record {number}');driver.select('#review-kind',kind);driver.fill('#review-category',category);driver.select('#review-status','Approved')
        if purpose:driver.select('#review-spending-role',purpose)
        driver.fill('#review-reason',reason);driver.click('#review-ack');driver.click_button('Save transaction decision');driver.wait_text('.imports-panel > .success-message','Transaction decision saved','source and purpose committed');driver.interactive('.imports-table tbody button')
    bank_decide(1,'Income','Wages','','Verified original fictional payroll; not a spending purchase')
    bank_decide(2,'Expense','Food','Necessary spending','Discussed necessary groceries with fictional client')
    driver.wait_text('.imports-totals','Net spending$25.11','approved-only source totals update')
    def analysis()->None:
        driver.client_section('Spending Analysis');driver.interactive('#spending-search');driver.fill('#spending-from','2026-09');driver.fill('#spending-through','2026-09');driver.click_button('Update analysis');driver.interactive('#spending-search')
    analysis();driver.wait_text('.sa-metrics','$25.11','review purpose automatically appears in Spending Analysis')
    driver.wait_value("select[aria-label='Type for Fictional groceries on 2026-09-02']",'necessary','necessary purpose source binding')
    driver.client_section('Review & reconciliation');bank_decide(2,'Expense','Food','Discretionary spending','Client discussed discretionary purpose amendment; preserve original import')
    analysis();driver.wait_value("select[aria-label='Type for Fictional groceries on 2026-09-02']",'discretionary','review amendment automatically recalculates analysis')
    driver.wait_text('.sa-metrics','100.00%','exact discretionary percentage updated')
    card=data_root/'extracted-card.pdf';card.write_bytes(build_pdf([
      ['Billing period 09/01/2026 to 09/30/2026','Purchases','09/05 Fictional card grocery 40.02','Payments and credits','09/06 Fictional card payment 20.01 CR','Minimum payment due 10.00'],
      ['Questions? Please contact us','Total purchases 40.02','Member FDIC']]))
    driver.client_section('Import statements');driver.click_button('Credit-card statements');driver.click('.card-import > summary');driver.select('#card-source-format','Selectable-text PDF');driver.fill('#card-csv-path',str(card));driver.click_button('Inspect card PDF')
    driver.wait_text('.pdf-selected-summary','Selected transactions: 2 / 200','card transactions preselected without fine print')
    driver.wait_text('.pdf-selected-summary','Charges / debt increases$40.02','purchase signs recognized')
    driver.wait_text('.pdf-selected-summary','Payments / debt reductions$20.01','credit marker reduces card debt')
    for selector,value in [('#card-account','Fictional extracted card'),('#card-period-start','2026-09-01'),('#card-period-end','2026-09-30'),('#card-opening','100'),('#card-closing','120.01')]:driver.fill(selector,value)
    driver.click('#card-retain');driver.interactive("//button[normalize-space(.)='Save card statement for individual review']",'xpath');driver.click_button('Save card statement for individual review')
    driver.wait_text('.card-progress','Needs Clarification 2','card PDF does not approve records')
    if driver.visible("//button[normalize-space(.)='Review card row 1']",'xpath'):raise SmokeFailure('Review decisions leaked into statement import page')
    driver.client_section('Review & reconciliation');driver.click_button('Card transactions')
    if driver.visible('.card-import'):raise SmokeFailure('Import controls leaked into card review page')
    def card_decide(row:int,kind:str,category:str,purpose:str)->None:
        driver.click_button(f'Review card row {row}');driver.select('#card-row-kind',kind);driver.fill('#card-row-category',category);driver.select('#card-row-status','Approved')
        if purpose:
            driver.select('#card-row-purpose',purpose);driver.interactive('#card-row-reason')
        driver.fill('#card-row-reason','Discussed fictional card source, spending purpose and payment treatment with client');driver.click('#card-row-ack');driver.click_button('Save card transaction decision');driver.wait_text('.card-statements > .success-message','Card transaction decision saved','card decision and purpose saved')
    card_decide(1,'purchase','Food','Necessary spending');card_decide(2,'payment','Card payment','')
    driver.wait_text('.card-totals','Net card spendingUSD 40.02','card payment excluded from spending')
    driver.wait_text('.card-reconciliation','Available statement arithmetic reconciles','selected card source and approved movement reconcile')
    analysis();driver.wait_text('.sa-metrics','$65.13','bank and card approved spending combined exactly')
    driver.wait_value("select[aria-label='Type for Fictional card grocery on 2026-09-05']",'necessary','card purpose automatically available in analysis')
    driver.click("button[aria-label='Classification history for Fictional groceries on 2026-09-02']");driver.wait_text('.sa-history','Discussed necessary groceries','original review-purpose history retained');driver.wait_text('.sa-history','discretionary purpose amendment','amended review-purpose history retained');driver.click_button('Close classification history')
    driver.command('POST','/execute/async',{'script':'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));','args':[]})
    driver.command('POST','/window/rect',{'width':1600,'height':1100});driver.scroll_to('.sa-heading','start');driver.command('POST','/execute/async',{'script':'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));','args':[]});driver.screenshot(artifacts/'native-extracted-spending.png');driver.command('POST','/window/rect',{'width':1200,'height':800})
    folder=data_root/'extraction-backups';folder.mkdir();firm=driver.text('.workspace-label strong')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']");driver.click_button('Backup & recovery');driver.fill('#backup-folder',str(folder));driver.fill('#backup-password',password);driver.click_button('Create encrypted backup');driver.wait_text('.backup-success','Encrypted backup saved.','selected evidence and classifications backed up');backup=driver.text('.backup-file-path')
    driver.click_button('Lock workspace');driver.wait_text('.auth-card','Welcome back','extraction workspace locked');restart_driver();driver.open(binary);driver.fill('#password',password);driver.click_button('Unlock workspace');driver.open_client('Morgan Extraction');analysis();driver.wait_text('.sa-metrics','$65.13','automatic classifications persist across process restart')
    driver.click('.lock-button');driver.wait_text('.auth-card','Welcome back','lock before extraction recovery');driver.click('.restore-panel > summary');driver.fill('#restore-file',backup);driver.fill('#restore-password',password);driver.click_button('Verify backup for recovery');driver.wait_text('.restore-review',firm,'extraction backup validated');driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace');driver.wait_text('.auth-card','Workspace restored. Unlock it','extraction recovery complete');driver.fill('#password',password);driver.click_button('Unlock workspace');driver.open_client('Morgan Extraction');analysis();driver.wait_text('.sa-metrics','$65.13','exact bank/card classifications recovered')
    driver.open_client('Robin Synthetic');driver.client_section('Spending Analysis');driver.interactive('#spending-search')
    if 'Fictional card grocery' in driver.text('.spending-analysis'):raise SmokeFailure('Extracted financial source crossed client boundary')
    driver.open_client('Alex Fictional');driver.client_section('Profile & intake')
    status('transaction-focused bank/card PDFs: excluded fine print/pages/balances, editable preselection and exact draft totals, only selected imports, explicit atomic necessary/discretionary review, automatic combined analysis, unchanged source/history, separate pages, restart, encrypted recovery and client isolation')


def check_spending_analysis(driver: WebDriver, artifacts: Path, data_root: Path,
                            password: str, binary: Path, restart_driver: Callable[[], None]) -> None:
    """Visible native controls only; independent figures and actual encrypted recovery."""
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Clients']")
    driver.click_button('Add client');driver.fill('#first-name','Quinn');driver.fill('#last-name','Analysis')
    driver.click_button('Create client');driver.wait_text('.client-workspace-header','Quinn Analysis','separate spending fixture client')
    driver.client_section('Monthly budget');driver.click_button('Add expense');driver.fill('#budget-label-0','Reviewed monthly discretionary target');driver.fill('#budget-amount-0','100');driver.select('#budget-kind-0','Discretionary expense');driver.click_button('Save budget and calculate');driver.wait_text('.budget-panel','Budget saved and calculated.','real saved discretionary target')
    path=data_root/'spending-fictional.csv'
    path.write_text('Date,Description,Amount,Category,Type\n2026-07-09,Native subscription,-10,Memberships,expense\n2026-08-09,Native subscription,-10,Memberships,expense\n2026-09-01,Native rent,-1000,Housing,expense\n2026-09-02,Native dining,-100,Dining,expense\n'+''.join(f'2026-09-0{i+2},Native coffee {i},-10,Coffee,expense\n' for i in range(1,6))+'2026-09-09,Native subscription,-10,Memberships,expense\n2026-09-10,Native refund,20,Dining,refund\n2026-09-11,Native pending,-999,Dining,expense\n',encoding='utf-8')
    driver.client_section('Import statements');driver.fill('#csv-path',str(path));driver.click_button('Inspect CSV');driver.wait_text('.csv-preview',path.name,'synthetic spending source inspected')
    driver.fill('#csv-account-label','Native analysis checking')
    for key,heading in [('date','1: Date'),('description','2: Description'),('amount','3: Amount'),('category','4: Category'),('kind','5: Type')]:driver.select(f'#csv-map-{key}',heading)
    driver.click_button('Import into review queue');driver.wait_text('.imports-panel > .success-message','CSV imported into the review queue','source stays separate from analysis')
    driver.client_section('Spending Analysis')
    if driver.observe_script("return document.querySelectorAll('nav[aria-label=\"Client workspace sections\"] button').length")!=13:raise SmokeFailure('Expected thirteen client-local sections')
    def ready():driver.interactive('#spending-search')
    def period():
        driver.fill('#spending-from','');driver.fill('#spending-from','2026-09');driver.fill('#spending-through','');driver.fill('#spending-through','2026-09');driver.click_button('Update analysis');ready()
    ready();period();driver.wait_text('.sa-metrics','$0.00','pending sources never enter analysis')
    driver.click('.sa-boundaries > summary');driver.click_button('Open separate review & reconciliation');driver.interactive('.imports-table tbody button')
    for number in range(2,13):
        driver.click_button(f'Review {path.name} record {number}');driver.select('#review-status','Approved');driver.fill('#review-reason','Fictional source and category individually verified for spending analysis');driver.click('#review-ack');driver.click_button('Save transaction decision');driver.wait_text('.imports-panel > .success-message','Transaction decision saved','explicit approved categorized fact');driver.interactive('.imports-table tbody button')
    driver.click_button('Recurring payments');driver.click_button('Review payments for Native subscription');driver.fill('#recurring-label','Verified fictional service');driver.select('#recurring-frequency','Monthly');driver.click('#recurring-subscription');driver.fill('#recurring-reason','Client confirmed these three fictional monthly payments are an active subscription');driver.click_button('Save payment pattern decision');driver.wait_text('.recurring-review','Counselor-classified subscription','subscription evidence confirmed separately')
    status('spending fixture: explicit source approvals and confirmed subscription saved')
    driver.client_section('Spending Analysis');ready();period();driver.wait_text('.sa-metrics','$1,160.00','approved spending excludes pending and refund');driver.wait_text('.sa-reconciliation','Unclassified spending purpose $1,160.00','categories never infer spending purpose');driver.wait_text('.sa-reconciliation','Net spending after refunds $1,140.00','exact refund reconciliation')
    def change(merchant,date,role,reason):
        driver.select(f"select[aria-label='Type for {merchant} on {date}']",role);driver.interactive('#spending-reason');driver.fill('#spending-reason',reason);driver.click('#spending-ack');driver.click_button('Save spending type');driver.wait_text('.spending-analysis > .success-message','Spending type saved.','purpose save explicitly confirmed');ready()
    change('Native rent','2026-09-01','Necessary','Client verified rent is necessary housing')
    change('Native dining','2026-09-02','Discretionary','Initial discussion classified the meal as discretionary')
    for i in range(1,6):change(f'Native coffee {i}',f'2026-09-0{i+2}','Discretionary','Client verified this fictional coffee purchase was discretionary')
    change('Native subscription','2026-09-09','Discretionary','Client verified this service is discretionary')
    driver.wait_text('.sa-metrics','$160.00','all discretionary purposes saved');driver.wait_text('.sa-reconciliation','Unclassified spending purpose $0.00','all eligible expense purposes explicit')
    driver.wait_text('.sa-opportunities','Frequent small purchases','six small discretionary purchases flagged');driver.wait_text('.sa-opportunities','$60.00 · 6 transactions','independent exact small-purchase total');driver.wait_text('.sa-opportunities','Reviewed discretionary subscriptions','confirmed subscription enters opportunity panel');driver.wait_text('.sa-opportunities','$10.00 · 1 transaction','period subscription charge only');driver.wait_text('.sa-opportunities','Discretionary spending above saved budget','actual spending above real target')
    status('spending fixture: exact classified totals and all three opportunity areas verified')
    driver.command('POST','/window/rect',{'width':1536,'height':1024});driver.scroll_to('.sa-heading','start');driver.screenshot(artifacts/'native-spending-desktop.png')
    if driver.observe_script('return document.documentElement.scrollWidth>document.documentElement.clientWidth'):raise SmokeFailure('Spending desktop has horizontal page overflow')
    # Draft is guarded, and cancellation retains discussion without persisting it.
    driver.select("select[aria-label='Type for Native dining on 2026-09-02']",'Necessary');driver.fill('#spending-reason','Unsaved private discussion');driver.click("nav[aria-label='Client workspace sections'] button[aria-label='Import statements']");driver.wait_text('[role=alertdialog]','Discard unsaved changes?','purpose navigation guard');driver.click_button('Keep editing');driver.wait_value('#spending-reason','Unsaved private discussion','guard cancellation preserves unsaved reason');driver.click_button('Cancel type change');driver.wait_value("select[aria-label='Type for Native dining on 2026-09-02']",'discretionary','cancel restores saved type in the native widget')
    change('Native dining','2026-09-02','Necessary','After discussing essential travel, client verified this meal was necessary')
    driver.wait_text('.sa-metrics','$1,100.00','necessary amount recalculated');driver.wait_text('.sa-metrics','$60.00','discretionary amount recalculated');driver.wait_text('.sa-metrics','94.83%','necessary share recalculated');driver.wait_text('.sa-metrics','5.17%','discretionary share recalculated')
    if 'Dining' in driver.text('.sa-bars') or 'Discretionary spending above saved budget' in driver.text('.sa-opportunities'):raise SmokeFailure('Stale discretionary chart or budget opportunity remained')
    driver.wait_text('.sa-metrics','$1,160.00','purpose change preserves gross spending');driver.wait_text('.sa-reconciliation','$1,140.00','purpose change preserves net spending')
    driver.click("button[aria-label='Classification history for Native dining on 2026-09-02']");driver.wait_text('.sa-history','-100','original imported sign and value retained');driver.wait_text('.sa-history','Initial discussion classified the meal as discretionary','initial purpose retained');driver.wait_text('.sa-history','After discussing essential travel','amended purpose retained');driver.wait_text('.sa-history','America/Chicago','history date zone explicit');driver.scroll_to('.sa-history h3','start');driver.screenshot(artifacts/'native-spending-history.png');driver.click_button('Close classification history')
    driver.fill('#spending-search','Native coffee 1');driver.click_button('Filter transactions');ready();driver.wait_text('.sa-pagination','1–1 of 1','table-only search')
    driver.wait_text('.sa-metrics','$1,160.00','table search does not narrow snapshot');driver.click_button('Clear table filters');ready()
    driver.command('POST','/window/rect',{'width':800,'height':700});driver.scroll_to('.sa-heading','start');driver.screenshot(artifacts/'native-spending-narrow.png')
    if driver.observe_script('return document.documentElement.scrollWidth>document.documentElement.clientWidth'):raise SmokeFailure('Spending 800px layout has horizontal page overflow')
    driver.command('POST','/window/rect',{'width':1200,'height':800})
    status('spending fixture: type amendment, updated metrics/charts, original history, guard, table scope and responsive layout verified')
    firm=driver.text('.workspace-label strong');folder=data_root/'spending-backups';folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']");driver.click_button('Backup & recovery');driver.fill('#backup-folder',str(folder));driver.fill('#backup-password',password);driver.click_button('Create encrypted backup');driver.wait_text('.backup-success','Encrypted backup saved.','purpose history included in encrypted backup');backup=driver.text('.backup-file-path')
    driver.click_button('Lock workspace');driver.wait_text('.auth-card','Welcome back','spending workspace locked')
    if driver.observe_script("return document.querySelector('.spending-analysis,.sa-history')!==null"):raise SmokeFailure('Private spending analysis remained on lock')
    restart_driver();driver.open(binary);driver.fill('#password',password);driver.click_button('Unlock workspace');driver.open_client('Quinn Analysis');driver.client_section('Spending Analysis');ready();period();driver.wait_text('.sa-metrics','$1,100.00','purpose saved across complete process restart')
    driver.click('.lock-button');driver.wait_text('.auth-card','Welcome back','restart result locked before recovery');driver.click('.restore-panel > summary');driver.fill('#restore-file',backup);driver.fill('#restore-password',password);driver.click_button('Verify backup for recovery');driver.wait_text('.restore-review',firm,'spending backup verified');driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace');driver.wait_text('.auth-card','Workspace restored. Unlock it','spending backup activated');driver.fill('#password',password);driver.click_button('Unlock workspace');driver.open_client('Quinn Analysis');driver.client_section('Spending Analysis');ready();period();driver.wait_text('.sa-metrics','$1,100.00','purpose restored from encrypted backup');driver.wait_text('.sa-metrics','$60.00','discretionary totals restored exactly');driver.click("button[aria-label='Classification history for Native dining on 2026-09-02']");driver.wait_text('.sa-history','Initial discussion classified the meal as discretionary','original purpose history restored');driver.wait_text('.sa-history','After discussing essential travel','amendment reason restored');driver.click_button('Close classification history')
    driver.open_client('Robin Synthetic');driver.client_section('Spending Analysis');ready()
    if 'Native dining' in driver.text('.spending-analysis') or 'Native analysis checking' in driver.text('.spending-analysis'):raise SmokeFailure('Spending evidence crossed client boundary')
    driver.open_client('Alex Fictional');driver.client_section('Profile & intake')
    status('Spending Analysis: approved-only exact totals, real budget/subscription/small-purchase opportunities, discussion-bound purpose edits, all affected metrics and charts refreshed, guard cancellation, immutable original/history, table scope, desktop/800px layout, lock privacy, full restart, encrypted recovery and client isolation')


def check_inactivity_policy(driver: WebDriver, artifacts: Path, data_root: Path,
                            password: str, binary: Path, restart_driver: Callable[[], None]) -> None:
    def security() -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']")
        driver.click_button('Security')
        driver.interactive('#inactivity-minutes')
    def save(minutes: str) -> None:
        driver.fill('#inactivity-minutes',minutes)
        driver.click_button('Save inactivity timeout')
        driver.wait_text('.inactivity-settings [role=status]','Inactivity timeout saved.','explicit timeout save')
    security()
    driver.wait_value('#inactivity-minutes','15','legacy fifteen-minute default')
    save('1')
    driver.click('.inactivity-history > summary')
    driver.wait_text('.inactivity-history','Revision 1: 1 minutes','immutable policy version')
    driver.scroll_to('.inactivity-settings','start')
    driver.screenshot(artifacts/'native-inactivity-policy.png')
    if driver.observe_script('return document.documentElement.scrollWidth > document.documentElement.clientWidth'):
        raise SmokeFailure('Security policy screen overflows horizontally')
    driver.open_client('Alex Fictional');driver.click_button('Session notes');driver.interactive('#new-note')
    driver.wait_text('.notes-history','Synthetic private note for Alex only.','private client visible before inactivity')
    started=time.monotonic()
    # Read-only WebDriver DOM observations neither invoke app commands nor send
    # keyboard/pointer activity. Exercise the actual Rust timer, without a clock
    # override or shortened product timeout.
    driver.wait(lambda:driver.visible('#password'),'real one-minute inactivity lock',seconds=95)
    if time.monotonic()-started<45:
        raise SmokeFailure('Workspace locked before a real one-minute idle interval')
    if driver.visible('.notes-history') or driver.visible('#inactivity-minutes'):
        raise SmokeFailure('Private UI remained after inactivity locking')
    driver.fill('#password',password);driver.click_button('Unlock workspace')
    security();driver.wait_value('#inactivity-minutes','1','saved policy after unlock')
    restart_driver();driver.open(binary)
    driver.fill('#password',password);driver.click_button('Unlock workspace')
    security();driver.wait_value('#inactivity-minutes','1','saved policy after full restart')
    folder=data_root/'inactivity-policy-backup';folder.mkdir()
    driver.click_button('Backup & recovery');driver.fill('#backup-folder',str(folder));driver.fill('#backup-password',password)
    driver.click_button('Create encrypted backup');driver.wait_text('.backup-success','Encrypted backup saved.','custom policy backed up')
    backups=list(folder.glob('*.ebcbackup'))
    if len(backups)!=1:raise SmokeFailure('Expected one custom-policy backup')
    security();save('15')
    driver.click('.lock-button');driver.click('.restore-panel > summary')
    driver.fill('#restore-file',str(backups[0]));driver.fill('#restore-password',password)
    driver.click_button('Verify backup for recovery');driver.wait_text('.restore-review','Verified backup','policy backup review')
    driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace')
    driver.wait_text('.auth-card','Workspace restored.','policy backup restored')
    driver.fill('#password',password);driver.click_button('Unlock workspace')
    security();driver.wait_value('#inactivity-minutes','1','backup retains original policy rather than later fifteen-minute setting')
    save('15')
    driver.open_client('Alex Fictional');driver.click_button('Session notes');driver.interactive('#new-note');driver.wait_text('.notes-history','Synthetic private note for Alex only.','policy restore preserves original notes')
    status('configurable inactivity policy, default and version history, real one-minute Rust timer lock and private UI clearing, full restart, encrypted policy recovery and unchanged client notes')


def check_password_change(driver: WebDriver, artifacts: Path, data_root: Path,
                          password: str, binary: Path, restart_driver: Callable[[], None]) -> None:
    new_password = "Changed-" + secrets.token_urlsafe(24)
    wrong_password = "Wrong-" + secrets.token_urlsafe(24)
    driver.protected_values.extend([new_password, wrong_password])
    folder = data_root / 'password-change-backups'
    folder.mkdir()
    def settings(tab: str) -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']")
        driver.click_button(tab)
    def backup(secret: str) -> Path:
        settings('Backup & recovery')
        before = set(folder.glob('*.ebcbackup'))
        driver.fill('#backup-folder', str(folder)); driver.fill('#backup-password', secret)
        driver.click_button('Create encrypted backup')
        driver.wait_text('.backup-success', 'Encrypted backup saved.', 'password-stage backup completed')
        added = set(folder.glob('*.ebcbackup')) - before
        if len(added) != 1: raise SmokeFailure('Expected one new password-stage encrypted backup')
        return added.pop()
    old_backup = backup(password)
    old_bytes = old_backup.read_bytes()
    settings('Security')
    driver.wait_text('.password-change', 'original passwords', 'old-copy password disclosure')
    driver.scroll_to('.password-change', 'start')
    driver.screenshot(artifacts / 'native-workspace-password-safeguards.png')
    if driver.observe_script('return document.documentElement.scrollWidth > document.documentElement.clientWidth'):
        raise SmokeFailure('Password-change screen overflows horizontally')
    driver.fill('#password-change-current', wrong_password)
    driver.fill('#password-change-new', new_password)
    driver.fill('#password-change-confirmation', new_password)
    driver.click('#password-change-acknowledge')
    driver.click_button('Change password and lock')
    driver.wait_text('.password-change [role=alert]', 'current password is incorrect', 'wrong current password rejected')
    for field in ['current', 'new', 'confirmation']:
        driver.wait_value('#password-change-' + field, '', 'submitted password cleared')
    driver.fill('#password-change-current', password)
    driver.fill('#password-change-new', new_password)
    driver.fill('#password-change-confirmation', new_password)
    driver.click('#password-change-acknowledge')
    driver.click_button('Change password and lock')
    driver.wait_text('.auth-card', 'Workspace password changed.', 'successful password change locks and clears workspace')
    if driver.visible('.app-shell') or driver.visible('.password-change'):
        raise SmokeFailure('Private workspace remained visible after password change')
    driver.fill('#password', password); driver.click_button('Unlock workspace')
    driver.wait_text('.auth-card [role=alert]', 'password is incorrect', 'old password rejected by active generation')
    driver.fill('#password', new_password); driver.click_button('Unlock workspace')
    driver.open_client('Alex Fictional');driver.click_button('Session notes');driver.interactive('#new-note')
    driver.wait_text('.notes-history', 'Synthetic private note for Alex only.', 'original notes survive rotation')
    restart_driver(); driver.open(binary)
    driver.fill('#password', new_password); driver.click_button('Unlock workspace')
    driver.open_client('Alex Fictional');driver.click_button('Session notes');driver.interactive('#new-note')
    driver.wait_text('.notes-history', 'Synthetic private note for Alex only.', 'new password and notes survive complete restart')
    new_backup = backup(new_password)
    for path, secret, wrong in [(old_backup, password, new_password), (new_backup, new_password, password)]:
        driver.click('.lock-button'); driver.click('.restore-panel > summary')
        driver.fill('#restore-file', str(path)); driver.fill('#restore-password', wrong)
        driver.click_button('Verify backup for recovery')
        driver.wait_text('.restore-panel [role=alert]', 'could not be verified', 'backup rejects other-generation password')
        driver.fill('#restore-password', secret); driver.click_button('Verify backup for recovery')
        driver.wait_text('.restore-review', 'Verified backup', 'backup accepts its original password')
        driver.fill('#restore-confirmation', 'RESTORE'); driver.click_button('Restore verified workspace')
        driver.wait_text('.auth-card', 'Workspace restored.', 'original-password backup activated')
        driver.fill('#password', secret); driver.click_button('Unlock workspace')
        driver.open_client('Alex Fictional');driver.click_button('Session notes');driver.interactive('#new-note')
        driver.wait_text('.notes-history', 'Synthetic private note for Alex only.', 'records retained through password-generation backup recovery')
    if old_backup.read_bytes() != old_bytes:
        raise SmokeFailure('Password change or restore rewrote the older encrypted backup')
    status('workspace password change, wrong-password rejection, empty submitted secrets, locked private UI, new-password restart, unchanged older backup, and old/new original-password encrypted recovery')


def check_report_presentation(driver: WebDriver, artifacts: Path, data_root: Path,
                              password: str, binary: Path, restart_driver: Callable[[], None]) -> None:
    card_title='Fictional approved bank/card assessment'
    bank_title='Fictional final-ready completed-period assessment'
    def open_page(select_client: bool=False) -> None:
        driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']")
        if not select_client:driver.interactive('.saved-assessments > summary')
        driver.click_button('Print & presentation')
        if select_client:driver.select('#workspace-client','Jamie Complete')
        driver.interactive('#workspace-client');driver.interactive('#presentation-assessment')
    def choose(title: str,final: bool=True) -> None:
        driver.interactive('#presentation-assessment')
        choice=driver.observe_script("return Array.from(document.querySelector('#presentation-assessment').options).find(o=>o.textContent.startsWith(arguments[0]))?.textContent",[title])
        if not choice:raise SmokeFailure('Original report is missing from presentation directory')
        driver.select('#presentation-assessment',choice);driver.interactive('#presentation-kind')
        if final:driver.select('#presentation-kind','Original final counselor approval')
        driver.interactive('#presentation-layout')
    def export(name: str) -> Path:
        target=artifacts / name
        driver.fill_pdf_destination('#presentation-pdf-path',target)
        driver.click_button('Export presentation PDF')
        driver.wait_text('.report-presentation > .success-message',str(target),'reviewed presentation PDF exported')
        driver.wait(lambda:target.is_file(),'presentation PDF file exists')
        driver.interactive('#presentation-layout')
        driver.wait_text('.saved-pdf-viewer .saved-pdf-path',str(target),'opening workflow bound to the actual exported file')
        driver.wait_text('.saved-pdf-viewer','remains in your chosen folder after lock','readable export lifetime is explicit')
        driver.wait_text('.saved-pdf-viewer','Opening from this app is available on Windows only','Linux does not pretend to launch a Windows viewer or print')
        if driver.observe_script("return document.querySelector('#presentation-viewer-consent')!==null"):raise SmokeFailure('Windows opening consent appeared on unsupported Linux host')
        return target
    def check_pdf(path: Path,card: bool=True) -> None:
        first=subprocess.run(['pdftotext','-f','1','-l','1','-layout',str(path),'-'],check=True,capture_output=True,text=True).stdout
        all_text=subprocess.run(['pdftotext','-layout',str(path),'-'],check=True,capture_output=True,text=True).stdout
        compact=''.join(first.split())
        expected=['Jamie Complete','2026-09-01','2026-09-30','Financial summary / USD','counselor-summary-v1','CURRENT CONTEXT NEEDS REVIEW']
        expected+=['1287.10','712.91','749.96','50.03','1037.05'] if card else ['Bank-only approved activity','799.99','1200.02']
        for value in expected:
            if ''.join(value.split()) not in compact:raise SmokeFailure(f'First PDF page omitted {value}')
        if 'Approval SHA-256' in first or 'Approved check /' in first:raise SmokeFailure('Approval audit detail displaced the financial summary')
        if 'Evidence appendix / original detailed report' not in all_text:raise SmokeFailure('Summary PDF omitted detailed evidence appendix')
        if card and compact.count('1037.05')!=1:raise SmokeFailure('Card debt was duplicated on the summary page')
    open_page();choose(card_title)
    driver.wait_text('.presentation-summary','Combined recorded spendingUSD 1287.10','reviewed card figures displayed without frontend calculation')
    driver.wait_text('.presentation-summary','Current context needs review','historical approval status is explicit')
    identity=driver.observe_script("return Array.from(document.querySelectorAll('.presentation-summary .field-help')).find(e=>e.textContent.startsWith('Original record'))?.textContent")
    if not identity:raise SmokeFailure('Original report integrity reference missing')
    driver.scroll_to('.presentation-summary','start');driver.screenshot(artifacts / 'native-report-presentation.png')
    summary=export('summary-card.pdf');check_pdf(summary);original=summary.read_bytes()
    driver.scroll_to('.saved-pdf-viewer','start');driver.screenshot(artifacts / 'native-saved-pdf-print-workflow.png')
    driver.select('#presentation-layout','Original detailed layout')
    if driver.observe_script("return document.querySelector('.saved-pdf-viewer')!==null"):raise SmokeFailure('Changing layout retained a stale PDF opening reference')
    driver.fill_pdf_destination('#presentation-pdf-path',summary);driver.click_button('Export presentation PDF')
    driver.wait_text('.report-presentation > [role=alert]','already exists','existing summary PDF cannot be overwritten')
    if summary.read_bytes()!=original:raise SmokeFailure('Detailed export overwrote an existing summary PDF')
    detailed=export('summary-card-original-detailed.pdf')
    first=subprocess.run(['pdftotext','-f','1','-l','1','-layout',str(detailed),'-'],check=True,capture_output=True,text=True).stdout
    if 'Final approval ID' not in first or 'counselor-summary-v1' in first:raise SmokeFailure('Original detailed layout changed interpretation')
    choose(bank_title);driver.select('#presentation-layout','Summary with evidence appendix')
    bank=export('summary-bank-v1.pdf');check_pdf(bank,False)
    choose(card_title,False)
    driver.wait_text('.presentation-summary','Bank-only approved activity','internal report remains bank-only rather than inheriting final combined approval figures')
    internal=export('summary-internal.pdf')
    first=subprocess.run(['pdftotext','-f','1','-l','1','-layout',str(internal),'-'],check=True,capture_output=True,text=True).stdout
    if 'Internal assessment / incomplete information' not in first or 'Combined recorded spending' in first:raise SmokeFailure('Internal summary was presented as a final card approval')
    choose(card_title)
    driver.click_button('Lock workspace');driver.wait_text('.auth-card','Welcome back','presentation workspace locked')
    if driver.observe_script("return document.querySelector('.report-presentation')!==null"):raise SmokeFailure('Private report presentation remained on locked page')
    restart_driver();driver.open(binary);driver.fill('#password',password);driver.click_button('Unlock workspace')
    open_page(True);choose(card_title)
    current=driver.observe_script("return Array.from(document.querySelectorAll('.presentation-summary .field-help')).find(e=>e.textContent.startsWith('Original record'))?.textContent")
    if current!=identity:raise SmokeFailure('Report presentation changed original identity after restart')
    check_pdf(export('summary-card-restarted.pdf'))
    backup_folder=data_root / 'presentation-backups';backup_folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']");driver.click_button('Backup & recovery')
    driver.fill('#backup-folder',str(backup_folder));driver.fill('#backup-password',password);driver.click_button('Create encrypted backup')
    driver.wait_text('.backup-success','Encrypted backup saved.','presentation records backed up')
    backup_path=Path(driver.text('.backup-file-path'));driver.click_button('Lock workspace');driver.click('.restore-panel > summary')
    driver.fill('#restore-file',str(backup_path));driver.fill('#restore-password',password);driver.click_button('Verify backup for recovery')
    driver.wait_text('.restore-review','Fictional Oak Counseling','original presentation sources verified for recovery')
    driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace');driver.wait_text('.auth-card','Workspace restored. Unlock it','presentation recovery activated')
    driver.fill('#password',password);driver.click_button('Unlock workspace');open_page(True);choose(card_title)
    current=driver.observe_script("return Array.from(document.querySelectorAll('.presentation-summary .field-help')).find(e=>e.textContent.startsWith('Original record'))?.textContent")
    if current!=identity:raise SmokeFailure('Encrypted recovery changed original presentation identity')
    check_pdf(export('summary-card-restored.pdf'))
    if summary.read_bytes()!=original:raise SmokeFailure('Restart or recovery rewrote the original exported summary')
    driver.select('#workspace-client','Alex Fictional');driver.interactive('#workspace-client');driver.interactive('#presentation-assessment')
    if card_title in driver.text('#presentation-assessment') or bank_title in driver.text('#presentation-assessment'):raise SmokeFailure('Another client received the original report choices')
    driver.select('#workspace-client','Robin Synthetic');driver.interactive('#workspace-client');driver.interactive('#presentation-assessment')
    driver.wait_text('.report-presentation','No saved assessments for this client','report presentation empty state is scoped')
    if driver.observe_script("return document.querySelector('.saved-pdf-viewer')!==null"):raise SmokeFailure('Saved PDF path crossed client context')
    status('summary-first PDF with full original appendix, v1/v2 exact financial basis and debt counted once, internal/final separation, original detailed layout, no overwrite, identity, privacy, restart and encrypted recovery')



def check_category_assistance(driver: WebDriver, artifacts: Path, data_root: Path,
                              password: str, binary: Path, restart_driver: Callable[[], None]) -> None:
    name='category-fictional.csv'
    merchant='Fictional classification market'
    source=data_root / name
    source.write_text('Date,Description,Amount,Category,Type\n'
                      f'2026-09-01,{merchant},-11.01,Food,expense\n'
                      f'2026-09-02,{merchant},-12.02,Food,expense\n'
                      f'2026-09-03,{merchant},-13.03,,expense\n'+
                      ''.join(f'2026-09-04,Fictional unmatched category row {i:02d},-{20+i}.00,,expense\n' for i in range(30)),encoding='utf-8')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Documents']")
    driver.interactive('#csv-path');driver.select('#workspace-client','Alex Fictional');driver.interactive('#csv-path')
    driver.fill('#csv-path',str(source));driver.click_button('Inspect CSV');driver.wait_text('.csv-preview',name,'fictional category source inspected')
    driver.fill('#csv-account-label','Fictional classification checking')
    for key,heading in [('date','1: Date'),('description','2: Description'),('amount','3: Amount'),('category','4: Category'),('kind','5: Type')]:driver.select(f'#csv-map-{key}',heading)
    driver.click_button('Import into review queue');driver.wait_text('.imports-panel > .success-message','CSV imported into the review queue','category fixture staged without automatic approval');driver.interactive('#csv-path')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Transactions']");driver.interactive('.imports-table tbody button')
    for number in [2,3]:
        driver.click_button(f'Review {name} record {number}');driver.interactive('#review-category')
        driver.select('#review-status','Approved');driver.fill('#review-reason','Fictional original source individually verified as Food')
        driver.click('#review-ack');driver.click_button('Save transaction decision');driver.wait_text('.imports-panel > .success-message','Transaction decision saved','two independent source approvals saved explicitly');driver.interactive('.imports-table tbody button')
    baseline=driver.text('.imports-totals')
    driver.click_button('Classification assistance');driver.interactive('#classification-search')
    driver.wait_text('.classification-count','31 unreviewed records','unreviewed categories counted without adding approved facts')
    first=driver.observe_script("return Array.from(document.querySelectorAll('.classification-results > li')).map(e=>e.dataset.recordId)")
    driver.click_button('Next suggestion page');driver.interactive('#classification-search');driver.wait_text('.classification-count','Showing 26–31','bounded second suggestion page')
    second=driver.observe_script("return Array.from(document.querySelectorAll('.classification-results > li')).map(e=>e.dataset.recordId)")
    if len(first)!=25 or len(second)!=6 or set(first)&set(second):raise SmokeFailure('Classification paging repeated or omitted candidates')
    driver.click_button('Previous suggestion page');driver.interactive('#classification-search')
    if driver.observe_script("return Array.from(document.querySelectorAll('.classification-results > li')).map(e=>e.dataset.recordId)")!=first:raise SmokeFailure('Classification first page was unstable')
    driver.fill('#classification-search',merchant);driver.click_button('Search category suggestions');driver.interactive('#classification-search')
    driver.wait_text('.classification-count','1 unreviewed records','exact merchant search narrows the eligible target')
    driver.wait_text('.classification-results','Suggested category: Food','two current approvals supply a category suggestion')
    driver.click('.classification-results details > summary');driver.wait_text('.classification-results','Category Food','source category evidence displayed')
    driver.scroll_to('.classification-assistance','start');driver.screenshot(artifacts / 'native-category-suggestions.png')
    driver.click_button('Use suggested category in unsaved review');driver.interactive('#review-category')
    if driver.observe_script("return document.querySelector('#review-category').value")!='Food':raise SmokeFailure('Category suggestion did not fill the unsaved category field')
    original_status=driver.observe_script("return document.querySelector('#review-status').value")
    if original_status not in ['Pending','Needs Clarification']:raise SmokeFailure('Category suggestion automatically selected approval')
    if driver.observe_script("return document.querySelector('#review-amount').value")!='-13.03':raise SmokeFailure('Category suggestion changed the source amount')
    driver.click_button('Discard category review draft');driver.interactive('.classification-results button')
    if driver.observe_script("return document.querySelector('#review-category')!==null"):raise SmokeFailure('Discarded category draft remained on screen')
    driver.click_button('Use suggested category in unsaved review');driver.interactive('#review-category')
    driver.fill('#review-reason','Fictional category evidence checked; transaction remains unapproved')
    driver.click_button('Save transaction decision');driver.wait_text('.imports-panel > .success-message','Transaction decision saved','category saved using the existing explicit unapproved decision');driver.interactive('#classification-search')
    driver.click_button('Working tools');driver.interactive('.imports-table tbody button')
    if driver.text('.imports-totals')!=baseline:raise SmokeFailure('Saving an unapproved suggested category changed approved financial totals')
    driver.click_button(f'Review {name} record 4');driver.interactive('#review-category')
    if driver.observe_script("return [document.querySelector('#review-category').value,document.querySelector('#review-status').value]")!=['Food',original_status]:raise SmokeFailure('Existing review did not preserve category/status exactly')
    driver.click('.review-history > summary');driver.wait_text('.review-history','transaction remains unapproved','category reason preserved in immutable decision history')
    folder=data_root / 'category-backups';folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']");driver.click_button('Backup & recovery')
    driver.fill('#backup-folder',str(folder));driver.fill('#backup-password',password);driver.click_button('Create encrypted backup');driver.wait_text('.backup-success','Encrypted backup saved.','category decision backed up')
    backup=driver.text('.backup-file-path');driver.click_button('Lock workspace');driver.wait_text('.auth-card','Welcome back','classification workspace locked')
    if driver.observe_script("return document.querySelector('.classification-assistance')!==null||document.querySelector('#review-category')!==null"):raise SmokeFailure('Classification data remained after lock')
    restart_driver();driver.open(binary);driver.click('.restore-panel > summary');driver.fill('#restore-file',backup);driver.fill('#restore-password',password);driver.click_button('Verify backup for recovery');driver.wait_text('.restore-review','Fictional Oak Counseling','category history verified after process restart')
    driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace');driver.wait_text('.auth-card','Workspace restored. Unlock it','category recovery activated');driver.fill('#password',password);driver.click_button('Unlock workspace')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Transactions']");driver.select('#workspace-client','Alex Fictional');driver.interactive('.imports-table tbody button')
    driver.click_button(f'Review {name} record 4');driver.interactive('#review-category')
    if driver.observe_script("return [document.querySelector('#review-category').value,document.querySelector('#review-status').value]")!=['Food',original_status]:raise SmokeFailure('Restart/recovery changed the saved category decision')
    driver.click('.review-history > summary');driver.wait_text('.review-history','transaction remains unapproved','restored category history retains the original reason')
    driver.click_button('Classification assistance');driver.interactive('#classification-search');driver.fill('#classification-search',merchant);driver.click_button('Search category suggestions');driver.interactive('#classification-search')
    driver.wait_text('.classification-assistance','No missing-category expenses or fees','saved category no longer offered as missing after recovery')
    driver.select('#workspace-client','Jamie Complete');driver.interactive('#classification-search')
    if name in driver.text('.classification-assistance') or merchant in driver.text('.classification-results'):raise SmokeFailure('Classification evidence crossed client scope')
    driver.select('#workspace-client','Robin Synthetic');driver.interactive('#classification-search');driver.wait_text('.classification-assistance','No missing-category expenses or fees','classification empty state is client scoped')
    status('local category suggestions reuse exact approved merchant evidence, stable bounded pages, unsaved draft/discard, explicit unapproved save, unchanged financial totals, immutable reason, lock, restart, encrypted recovery and client isolation')



def check_client_history(driver: WebDriver, artifacts: Path, data_root: Path,
                         password: str, binary: Path, restart_driver: Callable[[], None]) -> None:
    prefix='Fictional timeline session'
    original=f'{prefix} 00: meeting Tuesday.'
    corrected=f'{prefix} 00: meeting Wednesday, original source checked.'
    def history() -> None:
        driver.click_button('History & records');driver.interactive('#client-history-search')
    def search(text: str) -> None:
        driver.fill('#client-history-search','');driver.fill('#client-history-search',text)
        driver.click_button('Search client history');driver.interactive('#client-history-search')
    def note_review(text: str) -> None:
        driver.click("//article[contains(@class,'session-note')][p[normalize-space(.)="+json.dumps(text)+"]]//button",'xpath')
        driver.interactive('#corrected-note')
    driver.open_client('Alex Fictional');driver.click_button('Session notes');driver.interactive('#new-note')
    # Enough real saves to exercise stable pages independently of report count.
    for i in range(31):
        body=f'{prefix} {i:02}: meeting Tuesday.'
        driver.fill('#new-note',body);driver.click_button('Save session note');driver.wait_text('.notes-history',body,'timeline note saved explicitly');driver.interactive('#new-note')
    note_review(original)
    driver.click('.note-version-history > summary');driver.wait_text('.note-version-history',original,'original remains readable before correction')
    driver.fill('#corrected-note','');driver.fill('#corrected-note',corrected)
    driver.fill('#note-correction-reason','Fictional appointment day verified against original counseling record')
    driver.click_button('Save reviewed note correction');driver.wait_text('.notes-panel > .success-message','Reviewed note correction saved.','new immutable correction saved')
    driver.interactive('#new-note')
    if original in driver.text('.notes-history'):raise SmokeFailure('Superseded original was offered as the current note')
    driver.wait_text('.notes-history',corrected,'current note reflects reviewed correction')
    note_review(corrected);driver.click('.note-version-history > summary')
    driver.wait_text('.note-version-history',original,'original retained after amendment')
    driver.wait_text('.note-version-history',corrected,'corrected version retained with original')
    driver.wait_text('.note-version-history','Fictional appointment day verified','correction reason retained')
    driver.scroll_to('.note-correction','start');driver.screenshot(artifacts / 'native-note-correction-history.png')
    driver.click_button('Close note correction');history();driver.select('#client-history-kind','Session notes');search(prefix)
    driver.wait_text('.client-history-results','32 matching history records','31 original notes plus immutable correction')
    driver.click_button('Next client history page');driver.interactive('#client-history-search');driver.wait_text('.client-history-results','26–32','stable second history page')
    driver.click_button('Previous client history page');driver.interactive('#client-history-search');driver.wait_text('.client-history-results','1–25','stable previous history page')
    driver.select('#client-history-kind','All record types');search('meeting Wednesday')
    driver.wait_text('.client-history-results','1 matching history records','literal search isolates reviewed correction')
    driver.click('.client-history-event > summary');driver.wait_text('.client-history-event','Fictional appointment day verified','timeline amendment reason')
    driver.scroll_to('.client-history-results','start');driver.screenshot(artifacts / 'native-client-history-timeline.png')
    driver.click_button('Open Session notes section');driver.interactive('#new-note');driver.wait_text('.notes-history',corrected,'timeline opens existing scoped note workflow')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Reports']");driver.interactive('#workspace-client')
    driver.click('.saved-assessments > summary');driver.interactive('#record-include-evidence');driver.click('#record-include-evidence');driver.click_button('Review report evidence')
    driver.wait_text('.report-evidence-selection',corrected,'new reports offer latest corrected note')
    if original in driver.text('.report-evidence-selection'):raise SmokeFailure('New report selection offered superseded original note')
    driver.click_button('Discard assessment draft')
    folder=data_root/'client-history-backups';folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']");driver.click_button('Backup & recovery')
    driver.fill('#backup-folder',str(folder));driver.fill('#backup-password',password);driver.click_button('Create encrypted backup');driver.wait_text('.backup-success','Encrypted backup saved.','note chain and history backed up')
    backup=driver.text('.backup-file-path');driver.click_button('Lock workspace');driver.wait_text('.auth-card','Welcome back','note history locked')
    if driver.observe_script("return document.querySelector('.client-history,.note-correction')!==null"):raise SmokeFailure('Private client history remained after locking')
    restart_driver();driver.open(binary);driver.click('.restore-panel > summary');driver.fill('#restore-file',backup);driver.fill('#restore-password',password);driver.click_button('Verify backup for recovery');driver.wait_text('.restore-review','Fictional Oak Counseling','amended notes verified after full restart')
    driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace');driver.wait_text('.auth-card','Workspace restored. Unlock it','amendment recovery activated');driver.fill('#password',password);driver.click_button('Unlock workspace')
    driver.open_client('Alex Fictional');driver.click_button('Session notes');driver.interactive('#new-note');driver.wait_text('.notes-history',corrected,'current corrected note restored')
    note_review(corrected);driver.click('.note-version-history > summary');driver.wait_text('.note-version-history',original,'recovered original remains exact');driver.wait_text('.note-version-history','Fictional appointment day verified','recovered original amendment reason');driver.click_button('Close note correction')
    history();search(prefix);driver.wait_text('.client-history-results','32 matching history records','timeline rebuilt from recovered originals and correction')
    driver.open_client('Robin Synthetic');driver.click_button('History & records');driver.interactive('#client-history-search');search(prefix)
    driver.wait_text('.client-history-results','No matching client history records.','timeline and correction remain isolated by client')
    status('reviewed note correction, immutable original/reason/history, current report-note selection, timeline literal search/type filters/stable paging/source navigation, lock, full restart, encrypted recovery and client isolation')


def check_category_rules(driver: WebDriver, artifacts: Path, data_root: Path,
                         password: str, binary: Path, restart_driver: Callable[[], None]) -> None:
    label='Fictional reusable category rule'
    pattern='Fictional unmatched category row 00'
    def ready() -> None:
        driver.interactive('#classification-search')
    def save_rule(reason: str) -> None:
        driver.fill('#rule-reason',reason);driver.click_button('Save category rule')
        driver.wait_text('.category-rules > .success-message','Category rule saved.','rule saved explicitly without financial changes');ready()
    def applications() -> None:
        if not driver.observe_script("return document.querySelector('.rule-applications')?.open===true"):driver.click('.rule-applications > summary')
    def edit() -> None:
        driver.click_button(f'Edit rule {label}');driver.interactive('#rule-category')
    driver.select('#workspace-client','Alex Fictional');ready()
    driver.click_button('Working tools');driver.interactive('.imports-table tbody button');baseline=driver.text('.imports-totals')
    driver.click_button('Classification assistance');ready()
    driver.click_button('Create category rule');driver.interactive('#rule-label')
    driver.fill('#rule-label',label);driver.select('#rule-account','Fictional classification checking')
    driver.select('#rule-mode','Description (only when merchant is blank)');driver.fill('#rule-pattern',pattern);driver.fill('#rule-category','Food')
    save_rule('Fictional exact category rule reviewed by counselor')
    driver.wait_text('.category-rule-list','Active · Version 1','new rule version visible')
    edit();driver.fill('#rule-category','');driver.fill('#rule-category','Groceries');save_rule('Fictional category rule amended with a reason')
    driver.wait_text('.category-rule-list','Active · Version 2','edit appends a version')
    driver.fill('#classification-search',pattern);driver.click_button('Search category suggestions');ready()
    driver.wait_text('.classification-results','Suggested category: Groceries','rule proposes a category with no invented approval')
    driver.wait_text('.matched-category-rule','Version 2','exact applied rule version displayed')
    driver.click_button('Use suggested category in unsaved review');driver.interactive('#review-status')
    initial_status=driver.observe_script("return document.querySelector('#review-status').value")
    if initial_status not in ['Pending','Needs Clarification']:raise SmokeFailure('Rule application automatically selected approval')
    if driver.observe_script("return [document.querySelector('#review-category').value,document.querySelector('#review-category').readOnly,document.querySelector('#review-amount').readOnly]")!=['Groceries',True,True]:raise SmokeFailure('Rule draft lost category or captured-source protection')
    driver.fill('#review-reason','Fictional rule and original source checked; transaction remains unapproved')
    driver.click_button('Save transaction decision');driver.wait_text('.imports-panel > .success-message','Transaction decision saved','rule-bound decision saved explicitly');ready()
    applications();driver.wait_text('.rule-applications','Original version 2','immutable original rule reference retained')
    driver.wait_text('.rule-applications','Category Groceries','original applied category retained')
    edit();driver.click('#rule-archived');save_rule('Fictional category rule archived without deleting history')
    driver.wait_text('.category-rule-list','Archived · Version 3','rule archived with retained versions')
    edit();driver.click('#rule-archived');save_rule('Fictional category rule reactivated explicitly')
    driver.wait_text('.category-rule-list','Active · Version 4','rule reactivated with a new version')
    driver.click('.category-rule-list details > summary');driver.wait_text('.category-rule-list','Fictional category rule amended with a reason','prior rule reasons remain available')
    applications();driver.wait_text('.rule-applications','Original version 2','later edits do not relabel a saved application')
    driver.scroll_to('.category-rule-list','start');driver.screenshot(artifacts / 'native-category-rule-history.png')
    driver.click_button('Working tools');driver.interactive('.imports-table tbody button')
    if driver.text('.imports-totals')!=baseline:raise SmokeFailure('Rule CRUD or an unapproved application changed approved financial totals')
    folder=data_root / 'rule-backups';folder.mkdir()
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Settings']");driver.click_button('Backup & recovery')
    driver.fill('#backup-folder',str(folder));driver.fill('#backup-password',password);driver.click_button('Create encrypted backup');driver.wait_text('.backup-success','Encrypted backup saved.','rule histories and original application backed up')
    backup=driver.text('.backup-file-path');driver.click_button('Lock workspace');driver.wait_text('.auth-card','Welcome back','rule workspace locked')
    if driver.observe_script("return document.querySelector('.category-rules')!==null"):raise SmokeFailure('Rule page remained visible after lock')
    restart_driver();driver.open(binary);driver.click('.restore-panel > summary');driver.fill('#restore-file',backup);driver.fill('#restore-password',password);driver.click_button('Verify backup for recovery');driver.wait_text('.restore-review','Fictional Oak Counseling','rule history and application verified after process restart')
    driver.fill('#restore-confirmation','RESTORE');driver.click_button('Restore verified workspace');driver.wait_text('.auth-card','Workspace restored. Unlock it','rule recovery activated');driver.fill('#password',password);driver.click_button('Unlock workspace')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Transactions']");driver.select('#workspace-client','Alex Fictional');driver.interactive('.imports-table tbody button');driver.click_button('Classification assistance');ready()
    driver.wait_text('.category-rule-list','Active · Version 4','rule lifecycle restored exactly')
    applications();driver.wait_text('.rule-applications','Original version 2','restored rule application retains original version')
    driver.wait_text('.rule-applications','Category Groceries','restored application retains original category')
    driver.select('#workspace-client','Jamie Complete');ready()
    if label in driver.text('.category-rules') or pattern in driver.text('.category-rules'):raise SmokeFailure('Rules or application history crossed client scope')
    driver.select('#workspace-client','Robin Synthetic');ready();driver.wait_text('.category-rules','No category rules saved for this client.','rule empty state remains scoped')
    status('category-rule create/edit/archive/reactivate, current matched version, protected unsaved draft, explicit unapproved save, immutable original application, unchanged financial totals, history, lock, restart, encrypted recovery and client isolation')


def check_transaction_history(driver: WebDriver, artifacts: Path, data_root: Path, password: str,
                              binary: Path, restart_driver: Callable[[], None]) -> None:
    def ready() -> None:
        driver.interactive('#workspace-client')
        driver.interactive('#activity-text')
        driver.wait(lambda:driver.visible('.activity-results') is not None,'activity search completed')
    def search(text: str) -> None:
        driver.fill('#activity-text',text);driver.click_button('Search activity');ready()
    def open_first() -> None:
        driver.click('.activity-event > summary')
    # Create enough pending fictional import events for a real second page.
    # Existing financial decisions and all preceding assertions stay intact.
    source=data_root / 'history-pagination.csv'
    source.write_text('Date,Description,Amount,Category,Type\n'+''.join(f'2026-09-10,Fictional pagination row {i:02d},-0.01,Food,expense\n' for i in range(31)),encoding='utf-8')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Documents']")
    driver.interactive('#csv-path');driver.fill('#csv-path',str(source));driver.click_button('Inspect CSV')
    driver.wait_text('.csv-preview',source.name,'pagination fixture original source captured')
    driver.fill('#csv-account-label','Fictional pagination checking')
    for key,heading in [('date','1: Date'),('description','2: Description'),('amount','3: Amount'),('category','4: Category'),('kind','5: Type')]:driver.select(f'#csv-map-{key}',heading)
    for key,value in [('periodStart','2026-09-01'),('periodEnd','2026-09-30'),('openingBalance','100.00'),('closingBalance','99.69')]:driver.fill(f'#csv-{key}',value)
    driver.click_button('Import into review queue');driver.wait_text('.imports-panel > .success-message','CSV imported into the review queue','pending pagination fixture imported without approval')
    driver.interactive('#csv-path')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Transactions']")
    driver.interactive('.imports-table tbody button')
    progress=driver.text('.imports-progress')
    driver.click_button('Activity history');ready()
    first=driver.observe_script("return Array.from(document.querySelectorAll('.activity-event')).map(e=>e.dataset.eventId)")
    driver.click_button('Next activity page');ready()
    second=driver.observe_script("return Array.from(document.querySelectorAll('.activity-event')).map(e=>e.dataset.eventId)")
    if not first or not second or set(first)&set(second):raise SmokeFailure('Activity pagination repeated or omitted the next page')
    driver.click_button('Previous activity page');ready()
    if driver.observe_script("return Array.from(document.querySelectorAll('.activity-event')).map(e=>e.dataset.eventId)")!=first:raise SmokeFailure('Stable activity page changed without a source change')
    driver.click_button('Working tools');driver.interactive('.imports-table tbody button')
    if driver.text('.imports-progress')!=progress:raise SmokeFailure('Read-only activity paging changed review decisions')
    driver.click_button('Activity history');ready()
    search('Synthetic reserve transfer');open_first()
    driver.wait_text('.activity-event[open]','Current bank transfer counterpart','bank transfer history exposes reciprocal source')
    driver.click_button('Open counterpart history');ready();open_first()
    driver.wait_text('.activity-event[open]','Synthetic incoming transfer','other transfer leg history is independently available')
    driver.select('#workspace-client','Jamie Complete');ready()
    driver.select('#activity-kind','Card');driver.select('#activity-status','Approved')
    driver.fill('#activity-from','2026-09-01');driver.fill('#activity-through','2026-09-30')
    search('Fictional card groceries')
    index=driver.observe_script("return Array.from(document.querySelectorAll('.activity-event')).findIndex(e=>e.querySelector('summary').textContent.includes('100.02')&&e.textContent.includes('Historical event'))")
    if not isinstance(index,int) or index<0:raise SmokeFailure('Historical one-cent corrected card approval was not retained')
    driver.click(f'.activity-events > li:nth-child({index+1}) summary')
    driver.wait_text('.activity-event[open]','Original reported values','card history keeps original values')
    driver.wait_text('.activity-event[open]','100.01','original exact card amount remains')
    driver.wait_text('.activity-event[open]','Renamed fictional household card (archived)','current card identity is separate from original')
    driver.wait_text('.activity-event[open]','Historical event','prior corrected approval remains historical')
    driver.scroll_to('.transaction-activity','start');driver.screenshot(artifacts / 'native-transaction-activity.png')
    driver.scroll_to('.activity-event[open]','start');driver.screenshot(artifacts / 'native-transaction-evidence.png')
    driver.click_button('Clear activity filters');ready();search('Fictional card payment');open_first()
    driver.wait_text('.activity-event[open]','Latest recorded repayment link','card payment retains bank approval locator')
    driver.click_button('Open linked decision');ready();open_first()
    driver.wait_text('.activity-event[open]','Fictional matched card repayment','exact linked bank decision opens read-only')
    driver.click_button('Show all events for this record');ready()
    driver.wait_text('.activity-events','Fictional matched card repayment','all original and subsequent bank events found')
    driver.select('#workspace-client','Alex Fictional');ready()
    if 'Fictional matched card repayment' in driver.text('.activity-events'):raise SmokeFailure('Private card repayment crossed client context')
    search('No fictional activity matches this phrase');driver.wait_text('.activity-results','No activity matches','empty history is clear')
    driver.click_button('Lock workspace');driver.wait_text('.auth-card','Welcome back','history workspace locked')
    if 'Fictional matched card repayment' in driver.text('body') or driver.observe_script("return document.querySelector('.transaction-activity')!==null"):raise SmokeFailure('Activity evidence remained visible while locked')
    restart_driver();driver.open(binary);driver.fill('#password',password);driver.click_button('Unlock workspace')
    driver.click("nav[aria-label='Main navigation'] button[aria-label='Transactions']")
    driver.click_button('Activity history');driver.select('#workspace-client','Jamie Complete');ready()
    search('Fictional card groceries');open_first()
    driver.wait_text('.activity-event[open]','100.01','restored source history survives another full process restart')
    status('client-wide bank/card history, stable pagination, source/date/status filters, original/corrected evidence, both transfer legs, exact repayment decisions, client isolation, lock and restart over recovered encrypted records')


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
        control={'Approved activity':'#assessment-from','Working tools':'#assessment-from','Card completeness':'#card-facts-from','Bank & card activity':'#card-link-action'}.get(current)
        if control:driver.interactive(control)
        driver.click_button(area)
        driver.wait(lambda:driver.observe_script("return document.querySelector('.section-navigation button[aria-current=page]')?.textContent") == ('Approved activity' if area=='Working tools' else area),'selected financial review section')
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
        driver.client_section('Review & reconciliation');driver.click_button('Card transactions')
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
    driver.wait_text('.client-workspace-header h1','Jamie Complete','separate fictional final-report client')
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
    driver.interactive('#saved-assessment')
    record_id=driver.wait(lambda:driver.observe_script("return document.querySelector('#saved-assessment').value"), 'saved report directory contains the selected record after refresh')
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
    driver.interactive('#saved-assessment')
    record_id=driver.wait(lambda:driver.observe_script("return document.querySelector('#saved-assessment').value"), 'saved report directory contains the selected record after refresh')
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
    if count != 3:
        raise SmokeFailure("Expected Dashboard, Clients and Settings at the application level")
    for page, heading in [("Financial Review", "Approved financial activity"), ("Transactions", "Transaction review queue"), ("Documents", "Documents and statement accounts")]:
        driver.click(f"nav[aria-label='Main navigation'] button[aria-label='{page}']")
        driver.wait_text("#imports-title", heading, f"focused {page} page")
        driver.element(".financial-dashboard" if page=="Financial Review" else ".imports-totals" if page=="Transactions" else "#csv-path")
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
    status("three main pages and thirteen client sections, saved planner data, isolated client context, future areas and report/admin separation")


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
                    # stop_driver verified there are no live group members.
                    # WebKit's closed TCP sockets can still occupy a port in
                    # TIME_WAIT. Wait only on availability; never replay a UI
                    # action or weaken the full-process-restart assertion.
                    def ports_released() -> bool:
                        try:
                            ensure_available(options.port)
                            ensure_available(options.native_port)
                            return True
                        except SmokeFailure:
                            return False
                    driver.wait(ports_released, "closed native driver ports released", seconds=75)
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
                    try:
                        if driver.visible(".report-presentation [role=alert]"):
                            print("Presentation diagnostic: " + driver.safe(driver.text(".report-presentation [role=alert]")), flush=True)
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
        print(driver.safe("".join(traceback.format_tb(error.__traceback__))), file=sys.stderr, flush=True)
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
