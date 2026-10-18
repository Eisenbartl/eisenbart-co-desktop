# Client Spending Analysis

Open a client workspace and select **Spending Analysis**. This live page is
separate from **Import statements**, **Review & reconciliation**, the saved
planning overview, and immutable assessment reports. Its navy, white, slate and
muted gold layout follows the supplied spending-page reference. No example
transactions or mock financial figures are installed.

Choose inclusive From/Through months (YYYY-MM, at most 24 months), optionally an
account, and **Update analysis**. The initial period is the current Chicago month.
Compare previous period uses the immediately preceding equal number of complete
months. The page shows five summary cards, a category donut with accessible exact
amount/share table, discretionary category bars, approved transaction details,
Opportunity Areas and spending insights. Table search, category/type filters,
opportunity selection and pagination narrow the table without changing the
snapshot. Date/account scope affects the entire analysis. All values are USD.

Purpose can also be saved with a source approval in **Review & reconciliation**
(bank Review queue or Card transactions). The combined save is atomic, appends to
the same existing histories and automatically supplies the current analysis.
[Statement extraction and downstream rules](pdf-import-review.md).

## Eligible evidence and totals

Only current **Approved** rows with a nonblank reviewed category enter the page.
Pending, clarification and rejected rows do not enter totals, charts or insights.
Necessary/discretionary type comes from a current counselor purpose decision bound
to that row's exact approval; category names never imply purpose. Missing or
invalidated purposes remain **Unclassified purpose**.

- Total spending is gross bank expenses/fees/paid interest plus card purchases,
  fees and interest. Necessary + discretionary + unclassified equals this total.
- Refunds are separate. Net spending subtracts refunds from gross spending.
  Refunds do not become negative donut slices or artificial discretionary savings.
- Debt payments show card statement payments. They are excluded from spending;
  the corresponding bank principal movement is not counted again. Other bank
  balance movements have no invented debt purpose.
- Transfers show the outgoing side only and are excluded from spending. Transfer
  approval stays in the existing paired-source review workflow. No savings purpose
  is inferred from a generic transfer.
- Card liability signs are normalized for the table: an outflow is negative.
  Original signs and values remain readable in History.
- Identical card rows across distinct overlapping sources on the same mapped
  account (or same unassigned source account label) are ambiguous and excluded,
  even if categories or purposes differ. Repeated purchases within one source
  remain. Resolve overlapping evidence in source review before relying on totals.
  This page cannot establish complete coverage or identify every fuzzy duplicate.

Rust uses checked integer cents throughout. Percentages round half up to two
decimal places. Donut slices use largest-remainder allocation of 10,000 basis
points, with deterministic category order, so displayed shares sum to 100.00%.
Bar scales are integer proportions of the largest discretionary category. Zero
spending has no percentage denominator. A zero prior period has no fabricated
percentage change. Periods preceding the supported 1900 boundary are unavailable.

## Change a spending type

After discussing an expense with the client, select **Necessary** or
**Discretionary** in its Type column, enter a discussion/reason, confirm the
conversation and choose **Save spending type**. The saved result is reread from
Rust and all amounts, shares, charts, filters and opportunities update together.
Cancel preserves the saved type. Navigation guards protect an unsaved discussion.

An income, refund, transfer, card payment or other balance movement is read-only
here. Correct its economic kind or source evidence through the separate review
workflow. This page changes spending purpose; it does not rewrite source facts or
silently approve unreviewed transactions.

Bank changes reuse immutable financial-facts reviews. Card changes reuse immutable
card-completeness snapshots, retain the counselor's conclusion and replace only
the latest Spending Analysis annotation; preceding annotations remain in prior
snapshots. A card type change marks inventory unreviewed and requires renewed
card completeness review. It never silently renews final readiness. Existing
stored conclusion-size bounds may require a separate reviewed conclusion before
adding an annotation. Reasons are bounded to 500 characters (card whitespace is
normalized to the existing single-line snapshot format).

Every save verifies client ownership, current approval, scope and the full source/
classification/planning context. Concurrent or stale edits fail without replay.
Audit failure rolls back the classification. Original import values, source files,
transaction decisions and captured report bytes stay unchanged. Saved reports can
be flagged **Needs review**; they are never recalculated in place. History shows
the original imported values and dated purpose changes, counselor and reasons;
dates are displayed in America/Chicago.

If a write cannot be confirmed, the draft stays available and is not automatically
resubmitted. Reload and verify before trying again. If the save succeeded but its
refresh fails, stale metrics are hidden and the page explicitly says it was saved.
Client switching, lock and unmount clear private drafts, history and results, and
late responses cannot repopulate another client's page.

## Opportunity Areas

These are evidence-based discussion prompts, not financial judgments or promised
savings. Select a prompt to inspect its contributing transactions.

- **Reviewed discretionary subscriptions:** current discretionary bank charges
  supported by an active counselor-confirmed subscription decision in Recurring
  payments. Merchant names alone never establish a subscription. Card subscription
  confirmation is not implemented; card recurring activity can be a candidate.
- **Frequent small purchases:** at least five discretionary expenses strictly
  under $15 in the selected period. Exactly $15, fees and interest are outside this threshold.
- **Discretionary spending above saved budget:** actual discretionary spending
  above the saved monthly discretionary budget times the selected month count.
  Available for All accounts only, with at least one saved discretionary line and
  all those amounts known. No target is invented for an absent/incomplete budget.
- **Possible recurring discretionary merchants:** current discretionary merchant
  activity in at least three distinct months on the same account. These are
  explicitly candidates, not confirmed subscriptions. Confirmed charges are not
  repeated in the candidate prompt.

Evidence stays within the authenticated firm/counselor/client scope and existing
encrypted workspace/backup stores. Schema 29 and old report formats are unchanged.
Existing safety limits remain: 20,000 eligible analysis rows, 10/25/50 table rows,
100 bank financial-facts reviews and 50 card-completeness snapshots per client;
500 bank purposes and 4,000 card purposes per snapshot.
Reaching a limit fails safely and retains history; history is never truncated.
This page is a live analysis view, with no new saved-analysis or export format.
