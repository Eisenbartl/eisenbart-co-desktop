# Statement running-balance checks — 0.10.0 source

For a synthetic USD checking/savings CSV, explicitly map **Reported running
balance column** and select **CSV row order for after-transaction balances**.
Choose oldest first or newest first only after verifying the original source.
The column must be signed balances after each transaction; before-transaction
balances, available balances, credit limits and credit-card balances are outside
this checkpoint. Amount inversion applies only to transaction amounts, never to
the signed balance column. Blank fields remain unknown; numeric zero is known.
The current selectable-text PDF parser does not extract balance columns.

Importing never approves a transaction. Declared order and successful source
validation can leave unchanged classified rows eligible for explicit bulk
approval. Unknown order, missing/invalid figures, discrepancies and date-order
conflicts require individual review. A source-level discrepancy blocks bulk
approval for that document. Existing duplicates, classifications and other
validation rules still apply. Saving an order choice does not change transaction
review states: previously uncertain rows remain Needs Clarification until
individually reviewed, even when the source-order issue has been resolved.

Open **Statement running-balance checks**, choose a source, and inspect the two
tracks. The visible sequence is chronological according to the declared file
order, with original record/page/line references and links to transaction review.
Newest-first sources are reversed as a complete sequence; dates never silently
sort the source. A contradictory or unknown date sequence prevents comparisons
for that track. Rejected dates are excluded from the approved sequence, so bad
rejected source dates cannot contaminate approved comparisons. Same-day rows preserve the declared file sequence and show a
warning because dates alone cannot establish their posting order.

| Track | Calculation and missing-data behavior |
|---|---|
| Original source step | Opening balance plus first original movement, then each previous reported balance plus the next original movement. Includes every imported source row, including rejected/unapproved rows, solely to check the file's arithmetic. A missing amount/anchor prevents that step; later known reported anchors can permit independent local steps. Predicted balances never replace missing reported values. |
| Approved cumulative step | Entered opening balance plus approved movements through that row, across all transaction types. Rejected movements contribute nothing and their reported balances never become approved anchors. An undecided row makes later approved expectations unknown until decided. Without a known opening balance, this track stays unknown. Corrected approved dates/amounts are used while original source values remain unchanged. |

Expected minus reported is the signed difference. Matched means exact cents agree;
Unknown, Pending, Excluded, Not Configured and Order Conflict never mean zero.
A calculated expectation may be shown while its comparison remains Unknown when
the reported balance is missing. The final reported balance is also compared
with the entered closing balance, when source order/dates and both references
are known. Existing whole-source and approved total-versus-closing checks remain
separate: a whole-statement total can agree while intermediate steps conflict.
All checks use Rust integer cents, never floating-point money calculations.

Independent example: opening $1,000.00, income $500.01, rent −$300.02 and fee
−$0.01 produce balances $1,500.01 → $1,199.99 → $1,199.98. Correcting approved
rent to −$300.01 leaves original source checks unchanged but produces approved
$1,200.00 → $1,199.99, each $0.01 above reported. Rejecting the rent leaves
approved movement $500.00, final expected $1,500.00, and a $300.02 difference from
reported $1,199.98. The rejected row has no approved expectation or difference.
An intermediate reported rent balance of $1,200.00 instead produces original
step differences −$0.01 then +$0.01 even though the overall closing total agrees.

Correct or clear a source-order declaration with a reason using **Save statement
balance setting**. This preserves immutable source metadata, reported values,
transaction decisions and financial plans. Source choices have append-only
reviewer/time/reason history. Corrections and approval reversals immediately
recompute comparisons and preserve existing transaction history; no automatic
approval, rejection, balance correction or planning change occurs. Arithmetic
agreement does not prove institution verification, correct balance meaning,
complete imports, or a verified asset balance for reporting.

Schema 12 adds scoped immutable balance-setting history and generic audit events.
Older inputs without balanceOrder default to unknown; existing client data,
planning inputs, source files and histories are retained. Failed migrations and
writes roll back. Authentication, optimistic import revisions, document/client
ownership and unsaved-edit guards reuse the existing review foundation. A
1,000-choice/client bound preserves all history and rejects further writes without
changing records. Encrypted backup snapshots include settings; restore validates
source references, allowed orders, bounded reasons and timestamps and recomputes
both tracks. Earlier applications cannot open schema 12. No new installer is
created for this milestone; clean Windows offline installation/OS-picker
acceptance remains separate from source CI and Linux native checks.
