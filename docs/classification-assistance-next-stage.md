# Next stage: bounded local transaction classification assistance

The initial bounded bank suggestion/draft workflow is implemented in 0.41.0; see
[usage and limits](classification-assistance.md). Saved reusable rules remain
future work, with [next-stage notes](classification-rules-next-stage.md).
The original design notes follow. The
original specification requests category suggestions, and Transactions already
contains a planned Classification assistance page. Merchant normalization,
recurring-payment review and immutable individual source decisions already work;
do not rebuild them or add another financial calculation/persistence layer.

Begin with a bounded bank-record workflow. Inspect existing approved category
evidence and merchant normalization, then define conservative local suggestions
whose exact evidence and conflicts are visible. Suggestions must stay separate
from approved facts. Unknown or conflicting evidence should remain unresolved;
do not guess confidence percentages, income/expense types, debt roles or transfer
relationships. Bank/card financial roles and repayments must keep their existing
explicit approval boundaries.

Use authenticated Rust scope, literal filters and stable bounded paging. Bind a
suggestion to its original source row, current review revision and the exact
approved evidence used. A stale/foreign/locked suggestion must fail before writes.
Applying a selected suggestion must require explicit counselor confirmation and
reuse the existing correction/approval/reversal/provenance/audit workflow. Define
whether the first action only fills a draft or saves a new decision, and label it
accurately. Never automatically approve a suggested category or alter saved reports.

If reusable rules are included, scope them before persistence, define revision,
edit/archive/restore semantics and preserve rule/evidence history. Keep the first
stage manageable rather than combining suggestions, OCR, broad statement adapters
and bulk automatic decisions. Source CSV/PDF review remains essential and offline.

Test independent examples, mixed/conflicting categories, deceptive merchant text,
literal Unicode/punctuation, pending/rejected evidence exclusion, stale decisions,
client/counselor isolation, bounded lists, atomic audit failures, reversal,
restart and encrypted backup/recovery. Keep the complete native workflow and all
existing report/financial assertions. Document only the offered behavior as
working. Do not create an installer unless explicitly requested by the user.
