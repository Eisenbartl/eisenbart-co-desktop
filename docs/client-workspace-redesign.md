# Client workspace redesign — 0.46.0

The user requested a visual and navigation pivot using the supplied
[implementation brief](design/client-redesign-brief.md) and
[HTML reference](design/client-workspace-reference.html). These are design inputs;
the HTML's fictional records, simulated actions and sample figures are not runtime
data or financial implementations. The separately named ChatGPT conversation was
not accessible in this environment. This stage implements the supplied files in
the actual Tauri/React app.

## Directory and client tabs

The application sidebar contains Dashboard, Clients and Settings. All twelve
client workflows are accessible inside each client workspace. This user-directed
navigation change supersedes the earlier twelve-item global sidebar; it does not
remove the underlying specification workflows.

Clients opens a directory without individual record forms or imported transaction
queries. Counts use real lifecycle statuses, including the separate Financial
Review status. Search matches name/email/phone; status filtering, name/update
sorting and 25-row pages keep the directory manageable. Profile updated means the
saved profile timestamp, not an invented last interaction. Suggested next steps
come from lifecycle status and do not pretend to be saved appointments.

Open workspace creates or focuses one in-app tab per stable client ID. Names can
change without changing tab identity; same-name tabs include their IDs in
accessible labels. Each tab remembers its last section/subtab while unlocked.
Only the selected client's current section is mounted. Persistent drafts require
explicit discard before navigating away or closing the active tab; ongoing writes
hold navigation. Inactive tabs retain navigation metadata, not hidden forms.
Discard prompts use a focus-contained in-app dialog with Keep editing and Discard
changes. This fixes the native dialog plugin's asynchronous `window.confirm`
result being treated as synchronous approval; standalone fallback consumers also
wait for the actual result. Pending prompts cancel on workspace locking.
Closing the active tab restores the most recently used remaining client, or All
clients, and keyboard focus follows. Arrow/Home/End keys navigate the tab strip.
Tabs, selected identities, searches and private drafts clear on lock; reopening
the app starts with a clean navigation session and saved encrypted records.

## Sections and retained functionality

| Local section | Existing implementations retained |
|---|---|
| Overview | Read-only scoped saved-plan summary, follow-ups, latest current note and module shortcuts |
| Profile & intake | Profile/contact/status/goals; separate intake and appointment/task subtab |
| Session notes | Authored notes, reviewed corrections and original history; searchable client history subtab |
| Monthly budget | Existing complete budget editor and optional method comparison |
| Debt payoff | Existing debt terms, fixed-monthly models, snowball/avalanche, warnings and schedules |
| Savings & sinking funds | Existing reserves, unknown balances, coverage targets, goals, contributions and coordination |
| Cash-flow calendar | Existing accounts, dated activity, paired modeled transfers, calendar/ledger and previews |
| Scenario comparisons | Existing coordinated temporary proposals, saved snapshots and historical review |
| Import statements | Existing bank CSV/selectable-text PDF import, source checks, account management, collection checklist and signed-card CSV review |
| Review & reconciliation | Queue, verified figures, running balances, recurring payments, transfer matching, source checks, transaction history, classification/rules, completeness and bank/card reviews as separate subtabs |
| Financial dashboard | Read-only approved-period bank activity; existing combined bank/card and completeness views remain accessible |
| Assessments & reports | Existing assessments/action plans, immutable evidence, final checklists/approval, PDF layouts and viewer handoff |

Only the selected review subtab mounts its corresponding tools. Source transaction
links from recurring/balance checks still open the scoped original review editor.
Future unsupported areas remain visibly Planned and do not offer fake saves.
Profile creation uses the existing validated save flow in a focus-contained dialog;
a new client opens Overview using its real returned ID.

## Financial and storage boundaries

Schema remains 29. No data migration, calculator equation, import parser, approval
policy, source BLOB, report payload or audit-history format changes. The new Rust
`client_workspace` read command authorizes the client and reuses the existing
transactional saved-plan read. It returns exact saved-plan results and a checked
integer-cent sum of entered debts, without querying full statement datasets or
writing audit events. React formats returned decimal strings; it does not compute
financial totals.

Unknown or incomplete income/necessary expenses/debt stay Unknown. Unallocated
cash requires saved complete budget amounts and entered income. Restricted savings
remain Unknown when unspecified. Explicit saved zeroes remain zeroes. A saved
simulation payment and saved budget debt allocation mismatch is shown until the
underlying records are saved consistently; the overview does not reconcile them
silently. The supplied $3,000 / $1,175 / $1,825 / $5,000 / $200 / $0 example is a
synthetic regression fixture, never a production default. Emergency targets and
payoff dates are returned from existing engines; sample dates are not hard-coded.

Overview badges identify counselor-entered/estimated planning values. Approved
statement figures remain in their period-specific financial dashboard. No pending
imports or budget estimates become observed transaction facts.

The navy sidebar, restrained gold accents, white cards, soft background and local
fonts follow the supplied reference. At narrower desktop sizes the sidebar
compacts and the local rail becomes a horizontal scrollable menu; data tables keep
scoped horizontal scrolling where necessary. Development-build warnings, local
password authentication, configurable idle locking, password changes and encrypted
backup/verified recovery remain active.

See development status for actual validation and installer evidence. An unsigned
development installer does not establish clean Windows 11 offline acceptance,
independent security/accessibility review or commercial readiness.
