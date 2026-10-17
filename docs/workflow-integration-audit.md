# End-to-end workflow integration audit — 0.51.0 / schema 32

This audit covers the supported individual-client checking, savings and credit-card
workflows. It adds no calculator, transaction store, report format, schema or
household/business feature. Existing household services retain their explicit
consent and planning-handoff boundaries. No installer or merge is authorized.

## Handoffs and approval boundaries

| Handoff | Canonical implementation and refresh | Counselor decision / retained history |
| --- | --- | --- |
| Client profile and intake → financial workflow | `storage::save_client`, `client_care_store` and `report_intake_store`; authenticated ownership scopes every subsequent service. `ClientOverview` reloads saved plans/care/notes on entry. | Intake prose never supplies verified income, balances or plan amounts. Selected intake is explicitly reviewed for reports; changing selected values, even temporarily, marks captured context historical. Unselected fields and unrelated follow-ups do not invalidate that selection. |
| Statement import → transaction queue | `imports_store`, `cards_store`, `statement_pdf` and the supervised PDF worker retain original fields, source hashes and locators. CSV/PDF staging produces Pending rows. | Extraction/preselection is not approval. Counselors verify warnings, categories, signs, dates and duplicates using existing review forms. Retaining originals needs existing authorization. Unsupported/scanned layouts remain unavailable rather than guessed. |
| Transaction approval/correction/reversal → current ledgers | Existing bank/card review and disposition services atomically save decisions, purpose bindings, source revisions and audit. Unified review reloads the same services after a focused decision. | New approval identities invalidate older purpose/method bindings. Transfers require existing paired approval; card repayments require reviewed matching. Reversal preserves history and withdraws dependent finalized evidence. No write is automatically replayed. |
| Approved transactions → Spending Analysis | `spending_analysis` reads current bank/card records and purpose histories; its save rereads current totals, mix, category bars, opportunities and table. Navigating back also reloads. | Unapproved/uncategorized/ineligible rows are excluded; refunds and transfers keep their separate bases. Purpose edits require a discussion/reason and acknowledgment. Original import data is unchanged. |
| Spending / verified facts → Financial dashboard | `dashboard` retains its labeled bank-only basis; `financial_health` combines supported bank/card services, exact existing arithmetic, coverage and verified inventories. Opening the section reads the latest source revision. | Required missing, stale, overlapping or unreconciled evidence withholds a definitive score. Reclassification/reapproval can require renewed facts/completeness review. The health score is a counseling policy, not a credit rating. |
| Actual financial observations → saved budget/debt/savings/calendar | Existing explicit editors own these assumptions. Transaction changes refresh actual analysis and do not write plan inputs. Health reductions call existing scenario/planning engines for a temporary preview. | Review the revised evidence with the client, explicitly edit the intended saved lines and save each agreed plan. A transaction, preview or scenario does not implicitly approve a plan change. Decision-success messages now state this handoff. |
| Saved budget → debt → savings → cash flow | `planning::debt_context`, `savings_context` and `cashflow_context` are reused by storage reads, combined scenarios, health previews and report validation. Current reads recompute dependent capacity, emergency targets and warnings from saved versions. The saved discretionary budget also supplies Spending Analysis comparison targets; actual spending is unchanged. | Debt terms/payment inputs, savings reserves/contributions and dated schedules stay independently entered. Changing the budget does not invent loan payments, move reserves or reschedule bills. Mismatches and unknowns remain visible. |
| Plans → scenarios | `get_assessment_planning` reads all four plans in one transaction. `scenarios` delegates to the same budget/debt/savings/calendar engines; `scenarios_store` saves immutable hypothetical proposals. | Expected plan revisions reject stale comparisons/saves. Saving a scenario does not apply it to live plans or create an agreement. Later plan changes mark its baseline Needs review; a plan-only scenario does not claim to validate actual transactions. |
| Current sources + selected intake/plans/scenario → assessments | `assessments_store` checks source/profile, selected intake, notes, branding, completeness/card evidence and plan/scenario versions in the save transaction. | Acknowledge the current context before saving a new record. Changed context rejects the save. Older payloads, figures, selected assumptions and digests stay unchanged. Summary-list and opened-record intake review flags now agree. |
| Assessments → action plans / final approval / reports | Existing `action_plans_store`, `final_reports_store`, `report_presentation_store` and `assessment_pdf` reuse scoped immutable parents and current review flags. Final policy/receipt and export checks run again at the relevant action. | New conclusions require a new reviewed assessment and applicable approval. Historical exports remain historical and disclose changed context. Explicit file export cannot overwrite an existing file or write inside the encrypted workspace. Older provisional card supplements need a new card-complete assessment; current card-inclusive approval already exists. |

Only the selected individual-client section is mounted. Reentering a section
reads its canonical data instead of reusing another section's cached totals.
Responses are checked against client identity, source/plan context where applicable,
and active request generation. Draft/busy guards prevent navigation during
unresolved edits; lock clears private UI. There is no continuous push subscription
across separate application processes: use the existing reload/reopen actions
after external edits. Mutations and report export recheck their authoritative
context; an old preview is not authority to overwrite a saved record.

## Findings and fixes

1. **Assessment-directory intake status drift.** Opened records already called
   `report_intake_store::changed_since`, but the directory omitted it. A counselor
   could see a current-looking list entry for historical selected intake. The
   directory now calls that same bounded selected-field history check. A regression
   was first observed failing on the old implementation, then passed after the fix.
   It covers unrelated edits, selected employment changes and later reversion;
   original report bytes/digest and encrypted recovery remain checked.
2. **Planning handoff was easy to overlook after a decision.** Existing bank,
   card, unified and spending save messages now explain that live analysis refreshes
   while agreed plan changes need an explicit counselor save. Issued reports retain
   their original basis; revised conclusions require a new assessment.
3. **Outdated card-report guidance.** Import/report messages incorrectly described
   integrated card approval as unimplemented. They now direct counselors to the
   existing bank/card supplement and card-completeness capture before final approval.
   Older report formats and their approval gates are unchanged.

## Acceptance evidence

The new backend regression
`workflow_handoffs_refresh_current_results_and_preserve_reviewed_plans_and_reports`
uses real scoped services and independently specified amounts. It stages a retained
synthetic checking CSV; confirms no pending spending is finalized; approves and
classifies rows; compares the live dashboard, spending and health results; records
a scenario and selected-intake assessment; reclassifies, rejects, reapproves and
corrects amounts; rejects stale report/scenario writes; explicitly changes the
budget; checks refreshed debt capacity / emergency target / calendar context;
exports the original report and verifies isolation, restart and encrypted recovery.

Independent checks include $3,000 actual income, $1,850 gross spending, $50 refunds,
$1,800 net spending and $1,200 operating surplus. Rejection removes the $300 dining
row, and a $25 correction produces $1,825 net spending. A discussed purpose change
moves $300 from discretionary to necessary without changing net spending. An
explicit $2,800 income / $1,850 necessary budget produces $750 debt capacity and an
$11,100 six-month emergency target, while actual income stays $3,000 and unrelated
plan inputs/schedules/revisions remain unchanged. Old report/scenario bytes survive.

The new `App` regression exercises the actual section-routing and save callbacks:
Financial dashboard → Spending Analysis save → refreshed Financial dashboard →
budget/debt/savings/calendar → scenario baseline → assessment directory. It checks
new source reads, removal of the old definitive score, the Needs review report
label, exact client routing, a single classification write and zero plan/final
approval writes. Rust integration tests establish the calculations; this interface
test establishes the UI handoffs rather than inventing a frontend calculator.

Existing tests extend the audit to card purchases/refunds and matched repayments,
paired bank/savings transfers, unknown classifications/methods, overlap/coverage,
irregular income, negative cash flow, stale previews, card-inclusive approval,
immutable client agreements, report receipts, migration/rollback, source traceability,
authenticated isolation and encrypted recovery. The existing complete native smoke
workflow exercises their visible controls and original PDF rendering.

Automated validation passed: all **323 UI tests in 44 files**, all **318 Rust
library tests** and both actual supervised **PDF-worker integrations**, version
consistency, production frontend build, Rust formatting, warnings-denied all-target
Clippy and the embedded debug no-bundle desktop build. Both focused Rust
regressions and all 96 affected interface tests also passed.

The complete native workflow passed **119 checkpoints**, its completion marker
and exit 0 in `/workspace/shared/desktop-smoke-workflow-audit-final`. The added
visible intake checks verify directory/opened-record parity for unrelated edits,
selected changes and reversion. All earlier source, financial, scenario, report,
card/household, lock/restart and encrypted recovery cases remained enabled.
Intake and 800px Spending Analysis screenshots were visually inspected, including
the new explicit planning-save guidance. Every tracked non-documentation input
and the accepted binary stayed unchanged; driver ports were closed afterward.
The evidence index is `/workspace/shared/workflow-integration-audit`.

Initial failures are retained. The old intake implementation failed the new list
assertion before the fix. Four interface expectations needed the complete new
message text. The first backend fixture correctly rejected a PDF destination
inside its workspace; it now uses a separate export directory. An initial native
launch omitted `cloud-native-run.sh` and failed before any workflow action. The
fresh successful run uses the existing documented wrapper, private D-Bus/display
and synthetic-test-only sandbox exception. Nothing changed in product startup,
and no uncertain financial write was replayed.

Current-head Windows source CI results belong to the existing
[PR checks](https://github.com/Eisenbartl/eisenbart-co-desktop/pull/1/checks).
Review only with those checks green; packaging remains explicitly opt-in and was
not requested. This separates source verification from installed Windows acceptance.

## Limits and next action

Supported bank/card adapters and existing bounded records/periods remain the scope.
No OCR, loan/mortgage/credit-line transaction adapters, advanced debt terms, automated
plan adoption, household linking or business counseling were added. Planning dates
and actual cash movements are deliberately independent. The health projection is
live; existing historical report formats do not silently acquire its newer score.

Windows 11 installed/offline acceptance, maximum-volume and independent
security/accessibility/commercial verification remain separate release work.
Next: review the completed audit changes in the existing PR with current-head
source checks green.
Leave main unmerged and create no installer unless explicitly requested.
