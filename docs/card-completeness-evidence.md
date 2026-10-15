# Card completeness evidence

Version 0.36.0 adds Financial Review → Card completeness and optional format-9
internal report/PDF capture. It records and checks the card evidence needed for
a future card-inclusive final policy. It does not grant final approval or replace
bank completeness, repayment matching, household balances or planning models.

## Working workflow

1. Review/approve card rows in Documents → Credit-card statements. Create durable
   accounts and assign sources in Financial Review → Bank & card activity.
2. In Financial Review → Working tools, record the existing verified household
   debt inventory at assessment end. Enter each card debt once, with its evidence.
   A combined debt entry must be split explicitly before mapping individual cards.
3. In Card completeness, enter calendar months and review every known card,
   including archived cards. Add expected but unprovided cards without inventing
   account IDs or amounts. Record applicability, expected dates and explanations.
4. Select statement sources explicitly. Select a reconciled closing statement at
   assessment end and the corresponding **existing** verified debt entry. The
   application compares those amounts and does not add debt again.
5. Assign Necessary or Discretionary to approved purchases, refunds, fees and
   interest. Bindings use the exact approval ID. Categories never supply verified
   spending roles. Payments and adjustments cannot receive spending roles.
6. Confirm the inventory, record missing information/conclusion, acknowledge the
   reviewed evidence, and save. Incomplete reviews can be retained with their
   findings; unknowns remain visible. Earlier numbered versions are read-only.
7. To capture it in Reports → Working tools → Saved counselor assessments, include
   both the bank/card supplement and saved card completeness evidence. Load each
   explicitly for the same current source/profile and calendar period. Internal
   PDF export includes the captured evidence; client-plan rendering excludes it.

After changing source decisions, card account metadata, bank financial facts or
the client profile, compare current evidence and save a new reviewed version.
**Review latest card facts context (keep draft)** retains notes/mappings, updates
known account names/choices and clears confirmation. **Remove obsolete card role
bindings from draft** explicitly discards superseded or withdrawn classifications;
choose roles again for new approvals. It does not alter historical versions.
Changing applicability retains entered selections; clear incompatible dates,
sources and mappings explicitly before saving Unknown or Not applicable.

## Checked rules

- Each known card must be listed once with its durable identity. Unprovided cards,
  unreviewed inventory, unknown applicability, missing information or a missing
  conclusion leave findings unresolved.
- Applicable dates must be ordered inside the assessment period. Selected sources
  must belong to the same reviewed card. Missing sources, gaps/overlaps, conflicting
  consecutive balances and unreconciled/undecided sources remain unresolved.
  Source selection never filters approved financial amounts.
- Non-applicability requires an explanation and cannot hide a declared in-period
  statement. Approved activity outside recorded applicability is flagged.
- Potential duplicate approved date/amount/description rows within a card remain
  unresolved. No automatic rejection, deduplication or matching occurs.
- An approved adjustment requires future explicit spending treatment. Unknown
  dates/amounts cannot be approved by the underlying card workflow.
- Each card maps to a distinct existing Debt entry. Unknown debt, a different
  closing amount, a missing end-date closing statement or a balance inventory
  without full-inventory/end-date verification remains unresolved. Negative card
  balances are credits and need a future explicit asset treatment.
- Unclassified approved spending is checked by **row count**, even when offsetting
  positive/negative amounts total zero. New approvals require renewed bindings.

All calculations use integer USD cents in Rust. An independently checked example
assigns a 100.01 purchase to Discretionary and a -20.02 refund, 5.04 fee and 2.05
interest to Necessary: Discretionary = **100.01**, Necessary = **-12.93**, total
card spending = **87.08**. A reported closing debt of **1037.05** mapped to the
existing **1037.05** debt gives a zero difference and leaves household debt
unchanged. A verified balance correction to **1037.06** gives a **-0.01** difference;
the earlier review/report keeps the original amount. Role totals can be negative
when refunds exceed charges; the software does not clamp them to zero or income.

“Recorded card evidence checks satisfied” describes these bounded checks only.
It does not establish complete household finances. The existing bank/card v1
calendar calculator remains unchanged, including its conservative whole-period
and repayment requirements; a shorter card applicability window does not override
its issues. Early card closure, credits, split/cross-period repayments and other
obligations still require explicit supported rules before final approval.

## Storage and compatibility

Schema 25 stores scoped append-only card completeness snapshots with immutable
captured account/source choices, exact role bindings, the original verified-facts
review and independently recomputed findings. Save requires expected review/source/
profile versions and a fresh acknowledgement; its audit is atomic. Saving this
context does not bump the financial-source revision or alter verified facts.
Later review versions explicitly flag linked reports as historical.

Read/recovery reconstructs original card decisions at the captured source revision,
checks original durable account/source assignments and verifies the exact scoped
financial-facts history. Rehashed foreign sources or altered calculations are
rejected before recovery activation. These are integrity checks, not signatures
or independent security certification.

Limits: 40 expected cards (20 known durable accounts), 20 statements and 4,000
role bindings; 50 versions, 4 MiB per snapshot and 16 MiB total history per client.
Existing import/report limits still apply, including 8 MiB per report. Oversized
saves fail without replacing working records. Original data remains encrypted in
the workspace; explicitly exported PDFs are readable files outside that encryption.

Format 9 requires the original bank/card supplement plus the matching saved card
review. Optional omission preserves formats 1–8 bytes/digests. Historical cards,
manual facts, role choices and findings remain unchanged after later corrections.
Final policy v1 still blocks card reports; old approval receipts retain their
original interpretation.

## Next implementation priority

Build and test a separately versioned card-inclusive final policy that combines
bank completeness, this recorded card evidence, reviewed repayment treatment and
all household obligations. Require complete current evidence, fresh professional
confirmation and exact parent/digest binding. Keep bank-only v1 approvals,
card-review v1 and card-completeness-evidence-v1 stable for historical records.
Do not redo card-account CRUD, source assignments, roles or balance mappings.
Broader parsers, OCR, printing, licensing and installed Windows acceptance remain
separate original requirements.
