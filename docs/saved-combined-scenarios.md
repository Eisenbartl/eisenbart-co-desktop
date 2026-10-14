# Saved combined scenarios — 0.20.0

Counselors can record a reviewed combined proposal independently of live financial
plans, reopen its full baseline/proposed comparison, and explicitly select it for
an internal counselor assessment. A save retains the client, server creation time,
four baseline versions, canonical input assumptions/changes, both complete model
sets and uncertainty warnings. It never applies the proposal to live plans or
approved statements and is not a client agreement. No installer or dependency
is added. Further controls for extra payments, new expenses, goals and temporary
income shocks remain separate checkpoints.

## Integrity and persistence

Schema 16 adds append-only `client_scenarios` and `assessment_scenario_links` with
client/organization/counselor ownership and compound foreign keys. Existing
records are not rewritten. Scenario save checks authenticated scope, expected
history count and all plan versions in one immediate transaction. It captures and
recalculates saved inputs in Rust, writes a format 1 `combined-scenario-v1` payload
with SHA-256 byte digest, and appends `scenario_recorded` atomically. Failed audit,
stale versions/counts or invalid inputs roll back the entire save. The expected
count prevents an uncertain response from silently becoming a duplicate save;
the interface does not retry mutations and tells the counselor to inspect history.

The saved calculator uses the existing exact-cent models and conservative reserve
rule. Savings reserves the higher of the budget debt allocation and applicable
debt-model payment including extras. Dated schedules remain independent explicit
inputs. Unknowns/unsupported projections remain unavailable. Reads and recovery
recompute both model sets from the captured baseline and input changes, checking
canonical descriptions/versions/results/warnings, scoped metadata and digest.
These checks establish arithmetic and byte integrity, not a digital signature or
complete statement coverage. History is limited to 50 records, 8 MiB each and
32 MiB total per client. Reading/exporting history adds no financial writes.

Later live-model revisions flag the saved scenario and its linked assessment
Needs review; reverting values does not restore the earlier version. Historical
inputs/outcomes/digests remain unchanged. Stale scenarios can be viewed/exported
through existing reports but cannot be selected for a new assessment. A new
baseline/proposal must be reviewed and recorded. Restart and encrypted recovery
retain scenarios and their report relations. Corrupt scenario arithmetic or links
are rejected during staged recovery before the active workspace changes.

## Report selection

The counselor explicitly chooses either current saved planning models or a saved
combined scenario. IPC sends scenario ID/digest only. The report transaction
rechecks scope, integrity and current baseline, derives the selected proposed
`entered-plans-v1` planning snapshot in Rust and atomically writes a compound
scenario relation with the assessment and audit. Format 4 retains that selection
reference; formats 1–3 and their existing optional supporting evidence remain
supported. Selected session/recurring evidence can accompany format 4 too.

Opening/exporting the report validates its relationship and exact equality with
the original selected proposal. The viewer presents both baseline/proposed
outcomes and the full captured proposed assumptions. The internal PDF records
scenario identity/time/digest, baseline/proposed income, savings-model buffer and
dated closing, original uncertainty warnings, and the existing detailed proposed
debt/savings/cash-flow terms and milestones. Approved activity stays in its
separate approved-data assessment. Client action plans remain separate reviewed
agreements with their existing feasibility/context checks.

## Independent examples and limits

With necessary spending 1,000.00, wages 1,500 biweekly = 3,250.00 and discretionary
200.00, the baseline buffer after debt 100.00 and savings 250.00 is 1,700.00.
A 1,600 biweekly wage/100.00 discretionary/200.00 debt/350.00 savings proposal
leaves 1,816.67. Proposed snowball interest is 23.01/payoff February 2027 and
emergency completion is May 31, 2027. Approved activity is still 0.00 in that
synthetic report. Native data instead has necessary spending 1,633.33 and budget
debt allocation 400.00: the saved comparison retains 766.67 → 983.34 buffers,
35.35 → 23.01 interest, June 2028 → November 2027 emergency dates, and 3,300.00
dated closing. Approved report operating cash flow remains 1,820.02.

Missing supported debt terms may be saved as an unavailable comparison; selecting
an unsupported repayment projection for a report is rejected. Choose explicitly
no repayment projection when an appropriate report cannot include one. This
checkpoint does not certify affordability/completeness, implement advanced terms,
change live plans, sign client agreements or establish commercial readiness.
See [development-status.md](development-status.md) for completed checks.
