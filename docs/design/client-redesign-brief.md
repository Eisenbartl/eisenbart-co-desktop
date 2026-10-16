# Eisenbart & Co. — Client Experience Redesign

## Objective

Refactor the current excessively long **Clients** screen into (1) a **single client directory** and (2) an individual **client workspace** opened in a new *in-app tab*. The redesign is primarily an information-architecture/navigation change, **not** a rebuild of the calculations, persistence, importers, or business logic.

Use the attached `eisenbart_client_workspace_mockup.html` as a visual and behavioral reference. Match the existing product identity: understated, professional, generous whitespace, dark navy sidebar, restrained gold accents, white cards, soft gray page background. Preserve existing UX patterns where practical.

## Navigation architecture

### Application-level navigation

Retain the existing main sidebar: Dashboard, Clients, Settings.

Directly below the application header, add a tab strip:
- **All clients**: fixed directory tab; can always return to this.
- **[Client name]**: open one tab per client. Clicking the same client again focuses its existing tab, never duplicates it.
- A close button on each client tab returns focus to the most recently used remaining tab, or All clients.
- Tabs must be keyed by immutable/stable client ID, never display name.
- Navigation between clients and subsections must not silently discard unsaved changes. Prompt to save/discard, or preserve draft state, according to existing application conventions.
- A client tab shows only that client's records. Switching tabs must never leak a previous client's transactions, planner state, notes, filters, or imports into another client.
- App tabs mean tabs **inside the desktop app**, not new OS/browser windows. If the current framework already implements robust document tabs, reuse that mechanism.

### Page 1: All clients / directory

Show only directory-level functions; **remove** all individual client's detailed sections from this view.

- Page title: **Clients**, brief supporting text, prominent **Add client** button.
- Compact status counters: total clients, new intakes, in-progress clients, pending client reviews. Derive all counts from real data; no hard-coded sample totals.
- Search by name, email, or phone; status filter; optional sorting by last activity/name.
- A simple directory table: client name and contact, stable client ID, counseling status, last activity, next step, **Open workspace** action.
- The entire relevant row can open the workspace, with proper accessibility and keyboard navigation.
- Optional pagination or virtualization when client counts grow; avoid querying all imported transactions just to render the directory.
- Keep Add Client and associated validation functioning as before.

### Page 2: Single-client workspace

At the top, show client avatar/initials, name, stable ID, contact details, current counseling status, last updated date, and context-aware shortcuts (**New session note**, **Import statement**, **New assessment**).

On the first visit, select **Overview** by default. Inside the client workspace, use a compact local subsection navigation area (left rail on desktop; horizontally scrollable submenu or equivalent on narrower windows). **Do not render every section in one long vertical page**. Show only the selected section and its associated controls and results.

#### Section structure

**Workspace**
1. Overview — important alerts, small read-only summary, follow-up tasks, quick links to the modules, latest note.
2. Profile & intake — personal information, contact details, household size, counseling status, goals.
3. Session notes — new note plus dated immutable history; corrections by subsequent entries, retaining author and timestamp.

**Planning tools**
4. Monthly budget — income/expenses/allocations, frequency conversion, method comparison, full existing editing form and calculations.
5. Debt payoff — debt terms, minimums, snowball and avalanche, interest model, fees, future payment changes, monthly schedule, comparisons, warnings.
6. Savings & sinking funds — accessible and restricted savings, emergency coverage of 1/3/6/custom months, monthly contributions, target dates, earmarks, irregular expenses and priority coordination.
7. Cash-flow calendar — opening account balances, actual paydays and bill due dates, payment scheduling, cash shortfall flags, internal transfers, date-specific forecast.
8. Scenario comparisons — coordinated plans, scenario snapshots, side-by-side results and assumptions.

**Documents & review**
9. Import statements — file selection, source/account matching, extraction preview, parser warnings, preserved source references.
10. Review & reconciliation — existing client review queue with filters, statuses, bulk approval, source figure approvals, running-balance checks, recurring merchant groups, payment pattern decisions, transfer matching. Keep these as **subtabs within the review section**, not separate pages and not appended below all the planning forms.
11. Financial dashboard — read-only approved transaction activity by selected date period; monthly income/spending, categories and trends; do not conflate budget estimates with verified history.
12. Assessments & reports — saved counselor assessments, action plans, progress reviews, client PDF generation.

## Overview layout

- Highlight only the most relevant unresolved conditions (e.g. missing source data or incompatible plan revisions).
- Summary cards: estimated monthly net income, necessary expenses, unallocated budget cash, entered debt balance. Other useful values can go into a compact planning snapshot (emergency target, savings balance availability, debt projected payoff, approved transaction count).
- Explicit source badges / footnotes: **Estimated / Counselor-entered / Approved statement / Unknown**.
- For the fictional Jon Doe data shown in reference screenshots, sample figures are: $3,000 take-home, $1,175 necessary expenses, $1,825 unallocated before additional allocations, $5,000 entered credit-card balance, $200 simulated monthly debt payment, May 2029 projected payoff, and $3,525 three-month emergency goal. These are **examples only**, not defaults or assumptions.
- Do not assume or display a zero when an opening balance, restricted savings, or savings contribution is unknown.
- Surface the existing mismatch: $200 simulated debt payment versus $0 saved monthly budget debt allocation. Reconcile the sources before treating combined scenarios as validated.

## Engineering boundaries

1. Inspect the current app component architecture, client storage layer, tests, and routing or view-state mechanism first.
2. Preserve all real data, schema migrations, stable client IDs, approval history, simulation equations, import routines, saved revisions, and local security controls. Do not reset or fabricate client data.
3. Refactor by composition. Move the current section implementations to lazily mounted client-specific views; do **not** rewrite calculation engines merely to change navigation.
4. Keep financial numbers consistent across views. Separate approved historical totals from planned/estimated inputs at every entry point.
5. Use clear loading/empty/error/success states. Inactive client sections need not execute expensive calculations or load full statement datasets.
6. Maintain keyboard accessibility, readable contrast, focus restoration on tab close, proper form labels, and narrow-window behavior.
7. Restrict data access by current client ID; verify updates, deletion, import, export, and report generation are explicitly scoped to the selected client.
8. Preserve the development-build warning that the software is not yet approved for confidential client data until its safeguards are completed and tested.

## Test requirements / acceptance criteria

- Directory renders many clients without showing a selected client's entire long form.
- Search and filters work; status counts reflect actual records.
- Opening Jon Doe creates the Jon Doe in-app tab; clicking him again focuses that tab without duplication.
- Opening Sara Smith creates a second client tab; switching between the two never cross-contaminates data.
- Each of the 12 sections works for both existing clients and a newly added client; all original fields, editing, save/cancel behavior, calculation outputs, and links remain available.
- On switching between sections or client tabs, unsaved edits are handled explicitly rather than lost silently.
- Review filters, merchant recurrence, running balance checks, and transfer matching remain accessible under the new review area.
- No default zero values for unknown fields; no budget numbers labeled as observed statement facts.
- A debt-versus-budget mismatch is visible and correctly resolved when the underlying saved records are reconciled.
- Screen remains navigable at common Windows desktop resolutions and window sizes.
- Existing regression tests pass; add UI navigation and client-isolation tests. Test offline launch and reopen behavior.

## Deliverable request for implementation agent

Implement this refactor in the **actual Eisenbart & Co. desktop app** using its current component framework. Do not ship only a parallel demo UI or static HTML page. Preserve working features, then provide a concise file-level change summary, test results, and a list of any unsupported behavior or deferred work. The provided HTML is a layout prototype, not a persistence or financial calculation implementation.
