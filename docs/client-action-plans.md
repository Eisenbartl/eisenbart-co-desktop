# Client action plans — 0.16.0

Open a client, open **Saved counselor assessments**, and select a current saved
assessment with a captured planning scenario. Open **Client action plans** and
choose **Review client plan basis**. Enter agreed budget recommendations, three to
five distinct steps in priority order, responsible people, follow-up arrangements
and reviewed limitations. Target/follow-up dates are optional; blank dates stay
“To arrange.” Review the figures/disclosures and acknowledge the correct client,
approved data, assignments, scenario, estimates and missing information before
recording. A new action plan is a separate immutable record, not an edit to an
assessment. No signature or certification of completeness is implied.

The saved view shows the agreement. Choose a private PDF destination outside the
encrypted workspace and export. Existing files are never overwritten; choose a
new filename. The PDF contains readable client information and should be kept
private. It includes the firm's saved name, client/counselor, America/Chicago report
date, historical period/position, approved income/spending/category bars, agreed
budget/allocations, emergency target, selected debt strategy and conditional
milestones, priorities, follow-up and disclosures. Original documents and full
internal evidence remain in the workspace. Print the exported PDF in a local PDF
viewer; direct application printing and logo/contact branding remain future work.

## Financial and persistence rules

Rust supplies all figures from the immutable supporting assessment. IPC accepts
only the assessment ID/digest, title, notes, ordered steps/dates and acknowledgment;
unknown fields and supplied financial outputs are rejected. Known entered income
and necessary expenses are required, all entered budget amounts must be known,
and the selected debt/primary savings payments cannot exceed monthly surplus.
Estimates remain disclosed. When saved models exist, their monthly debt payment
and primary emergency-plus-goal savings contributions define the agreement.
Otherwise the captured budget allocations are used with missing-model warnings.
Alternative savings forecasts are not selected contributions. Differences from
the original budget are explicitly disclosed; active plans are not synchronized.

Example: captured income 3,250.00 less necessary 500.00 and discretionary 200.00
leaves 2,550.00. Selected debt 100.00 and primary savings 250.00 leave a buffer of
2,200.00. Captured budget debt 150.00 originally left 2,150.00; both original and
agreed figures remain separately labeled. Approved historical cash flow 799.99
is not used as entered planning income. Missing inventories remain unavailable.

Schema 15 adds scoped append-only `client_action_plans`, a composite supporting
assessment foreign key and atomic `action_plan_recorded` audit entries. Upgrades
preserve old records/audits and roll back on failure. Payload format 1 uses
`client-action-plan-v1`, references the immutable assessment/digest, and stores
canonical notes, Chicago date, exact allocations and required disclosures. Limits
are 50 records/client, 64 KiB/plan and 4 MiB/client. Reads, export and restore verify
scope, digest/header identity, supporting assessment, canonical text/dates and
recomputed allocations/disclosures against captured inputs. Chicago dates use
bundled `chrono-tz` rules, including daylight saving, without system-timezone or
network dependence.

Supporting source/profile/firm/planning changes flag the plan **Needs review**.
Historical guidance remains available/exportable with review notices and original
values; stale supporting assessments cannot create new plans. Record a new current
assessment and plan after reviewing changes. Restart and encrypted recovery retain
both agreement and historical review state. Creation does not alter source review,
plan inputs or import revisions. Draft/busy guards protect client navigation,
assessment selection and competing edits; failed writes retain drafts and are not
automatically replayed.

## Scope still open

This is counselor-reviewed guidance with an explicitly incomplete financial basis.
The app cannot verify that every account/statement exists. No complete-basis
certification, client signing, action completion history, automatic reminder,
electronic delivery, logo/contact branding, direct print command or commercial
release acceptance is added. The internal report still needs recurring/session
evidence and remaining original reporting/model requirements. See
[remaining-requirements.md](remaining-requirements.md).
