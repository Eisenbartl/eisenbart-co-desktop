# Optional budget method comparisons — 0.18.0

Save a client's entered budget, then open **Compare optional budget methods**.
Enter the known required minimum portion of the saved debt allocation, or leave it
blank if unknown. Explicit 0 means no required minimum portion in this entered
plan. Enter custom needs/wants/future percentages totaling 100.00%, then choose
**Compare saved budget**. The comparison shows zero-based assignment status,
50/30/20 and custom targets beside entered amounts and differences.

Minimums are reclassified from existing debt allocations into needs. The future
bucket contains proposed savings plus extra debt payments. Classification adds no
payment and must not exceed the budget's debt allocation. Never count a debt
payment both as a necessary expense and a debt allocation. The comparison uses the
saved budget, not separately entered debt/savings/cash-flow models; verify required
minimums against actual obligations before relying on this assumption.

Example: entered income 3,250.00, necessary 1,633.33, wants 200.00, savings 300.00
and debt 400.00. Assigned spending/allocations are 2,533.33; 716.67 remains unassigned.
A required debt minimum of 100.00 yields needs 1,733.33 and future allocations
600.00. Standard targets are 1,625.00 / 975.00 / 650.00, with entered-minus-target
differences +108.33 / -775.00 / -50.00. Custom 60/20/20 targets are 1,950.00 /
650.00 / 650.00. Neither framework assigns the remaining balance automatically.

Needs and wants targets are spending references; below a ceiling does not imply
money should be spent. Future targets are savings/extra-debt references. These
are optional guidelines, not mandatory rules or verified complete finances.
The original financial plan, source transactions and all agreements stay intact.

## Exact arithmetic and missing data

Rust derives every figure from canonical saved inputs. Percentages accept up to
two decimal places and use integer basis points. Targets floor each exact share,
then allocate leftover cents by largest fractional remainder; ties go needs,
wants, future. Targets always sum to monthly income. On 0.01 of income, 50/30/20
allocates the cent to needs; on 0.03, each bucket receives 0.01. For 1.01 at
33.33/33.33/33.34, targets are 0.34 / 0.33 / 0.34. No binary floating-point money
calculation is used.

Unknown income prevents targets; missing necessary expenses prevent an assignment
conclusion. Missing category amounts withhold affected actual values rather than
turning unknowns into zeros. Unknown required debt minimums withhold needs/future
actuals but do not alter the zero-based assignment check. Known zero income can
show zero targets with an explicit limitation, never proof of no income or
compliance. Missing optional categories describe no entered allocation, not
verification of no real obligation. Estimates and irregular monthly inputs retain
existing warnings and monthly normalization rules.

## Read-only scope and temporary inputs

IPC accepts a saved budget revision, minimum assumption and percentage strings,
never supplied financial outputs. Rust rechecks authenticated client ownership,
revision and canonical budget arithmetic. A stale revision fails without changes;
**Reload saved budget** reads the latest context when no budget edits are pending.
Percentages outside 0–100, precision errors, a sum other than exactly 100.00%, and
minimums exceeding saved debt are rejected with the draft retained.

Inputs/results are temporary. They are cleared across client changes/restart and
are not included in saved reports, planning snapshots, client action plans or
backups. The saved budget remains persistent and can be compared again after
recovery. Input changes hide old results, budget edits hide/disable comparisons,
and competing writes are blocked while a comparison is loading. Late or mismatched
client/version responses are ignored or rejected. No audit/financial write is made
by comparing, and no schema migration or new dependency is required.

Combined scenario comparisons, persistent scenario selection, remaining advanced
financial models, fuller intake/review/security/commercial work and Windows release
acceptance remain future checkpoints. See
[remaining-requirements.md](remaining-requirements.md).
