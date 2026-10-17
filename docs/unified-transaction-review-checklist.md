# Unified transaction review — implementation checklist

Inspection baseline: 0.49.0 / schema 31 / `d83b653`. The checkout was clean;
extraction is completed, not being edited by another worker. Reuse this checkout.
The supplied [brief](unified-transaction-review-specification.md) names Forespan;
this feature assignment preserves the existing Eisenbart application identity.
No installer is authorized.

## Phase 1: inspected existing systems

| Requirement | Baseline status | Existing implementation / intended integration |
| --- | --- | --- |
| Secure local intake, retained originals, immutable locators | Already implemented and functioning | `imports_store`, `cards_store`, supervised `pdf_process`; use existing encrypted BLOBs and page/line evidence. |
| Transaction extraction and source selection | Already implemented and functioning | 0.48 bank/card detector, editable preselection, exact preview; no second parser. |
| Account recognition/periods | Implemented but needing improvement | Explicit checking/savings and card inputs, numeric period detection, reviewed durable identity; never invent institution, masks, dates or loan components. |
| Chronological current rows across accounts | Partially implemented | Existing event history combines bank/card decisions by event time, not one current row per transaction date. Add a current-row projection over the same validated records. Preserve event history. |
| Loan/credit-line/mortgage transactions | Blocked by other unfinished work | There are no loan-ledger transactions or supervised loan adapters. Show supported account scope explicitly; do not relabel card liability or cash records as loans or invent amortization. Money-market deposit recognition can reuse reviewed deposit accounts. |
| Full transaction provenance and dates | Partially implemented | Existing raw/current fields, record IDs, hashes, page/line, uncertainty and statuses; add explicit optional posting dates and reviewed payment-method details without replacing raw evidence. |
| Transaction financial classifications and corrections | Already implemented and functioning | Existing bank/card kind/category/date/amount/description approval services, warning acknowledgments, reversals and immutable review history. Reuse writes. |
| Detailed payment classifications and account metadata | Not implemented | Add bounded source-bound supplemental review metadata (institution, masked identifier, supported subtype and payment method). Do not infer debit-card use from a checking withdrawal. |
| Combined filters and deterministic transaction-date sort | Implemented but needing improvement | Extend one bank/card current-row query, bounded pages, literal search, stable same-day tie breaks, explicit unknown dates and stale tokens. |
| Original PDF canvas, page/zoom, optional split inspection | Not implemented | Scoped retained-original read; bundled maintained PDF renderer with scripting/attachments/links disabled; no remote viewer, plaintext temp file or invented coordinates. Non-retained originals show captured evidence only. |
| Review actions alongside source | Implemented but needing improvement | Share existing review APIs and types; source inspection and correction open from current table; preserve atomic purpose workflows and audit. |
| Payment-method totals/counts/averages and observations | Partially implemented | Reuse Spending Analysis eligibility, duplicate exclusion, confirmed subscriptions and explicit discretionary purposes. Add exact Rust calculations for reviewed card/debit methods, with unclassified coverage shown. |
| Transfers/card repayments/refunds excluded from purchase totals | Already implemented and functioning | Bank financial-facts and card-role/link engines; preserve these boundaries in new method analysis. |
| Statement arithmetic, coverage, duplicates and discrepancies | Already implemented and functioning | Existing reconciliation/completeness/duplicate advisories; surface their actual states, distinguish suspicion from confirmed exclusions, preserve original periods/balances. |
| Client/household/planning/report compatibility | Already implemented and functioning | Shared underlying approved records; changes invalidate source bindings and update live views, while saved plans and immutable reports retain their assumptions/history. |
| Persistence/authorization/backup | Already implemented and functioning | SQLCipher scope, optimistic revisions, atomic audit, backup validation; extend these for supplemental metadata only. |

## Implementation sequence and acceptance

1. Preserve baseline behavior and add the current-row projection/filter model.
2. Add scoped supplemental details and integrate existing correction/approval APIs.
3. Add optional document inspection with reliable page links; no coordinate highlighting.
4. Integrate exact eligible method analysis and explicit coverage/unknowns.
5. Exercise synthetic independent money/sign examples, filtering/paging/staleness,
   source security and correction history, original viewing, privacy, restart and
   encrypted recovery; then run full source/native regression and document limits.

Checks and outcomes will be recorded as they execute. Unavailable loan adapters,
OCR and installed Windows acceptance must not be described as completed.

## Executed stage result (0.50.0 / schema 32)

The current bank/card projection, filters, supplemental details, optional original
viewer, focused shared financial forms and exact method observations are working.
No second financial ledger, approval pipeline or parser was introduced. Money-market
labels reuse existing savings transactions; loan/credit-line/mortgage transaction
adapters remain a documented dependency rather than fabricated records.

All 312 UI tests passed. Rust coverage includes the 303-test matrix with its legacy
fixture follow-up, then all 13 final affected tests (one added; 304 library tests
now). Actual PDF-worker integration tests, Clippy and no-bundle builds passed.
The complete native regression passed 117 checkpoints before the localized channel
correction; final focused native acceptance passed on that corrected binary,
including independent loan/repayment/refund treatment, source viewing, edits,
original/history preservation, narrow layout, lock/restart and encrypted recovery.
Exact run/binary distinctions and limits are in `development-status.md`.
No installer was created or provided.

## Supplemental refinements within the same assignment

These complete gaps in the existing 0.50.0 implementation; there is no additional
phase, transaction store, approval service, analysis system or navigation section.
The existing optional viewer, purchase exclusions, unknown-method coverage and
averages are reused. Previously recorded acceptance results above remain historical.

- [x] Opening Review & reconciliation defaults to the existing combined current
  chronological table. All transactions is first; the original Review queue remains
  selectable. Switching client tabs preserves the deliberately selected section.
- [x] The optional action reads **View Source Document**. Closing its panel removes
  the source split immediately; the original full-width transaction table stays
  available. Existing resize/collapse/narrow-window behavior remains.
- [x] The existing Rust method totals supply separate dollar and count percentages,
  plus averages, for the same approved purchase selection and preceding period.
  Unknown/other methods remain separate and in both denominators. Existing
  transfer, card/loan repayment, refund, fee, interest, discrepancy and duplicate
  exclusions are reused. Empty denominators remain explicitly unavailable.
- [x] Independent acceptance example: four $10 credit purchases and one $160 debit
  purchase produce credit 20% dollars / 80% count / $10 average, and debit 80%
  dollars / 20% count / $160 average. Unknown-method example produces 40%/40%/20%
  dollars with 33.33% count each. Filters, rejection and restored approval recalculate
  count shares without altering original records or historical decisions.

Execution complete: 313 UI tests, all 14 affected Rust tests, Clippy and production/
standalone builds passed. The 305-test library matrix passed 304 tests and its
process-exit test passed on sequential rerun; both actual PDF-worker tests passed.
Updated focused native acceptance passed, including count recalculation, source
panel closing, isolation, restart and encrypted recovery. See development status
for exact run distinctions and evidence. No installer was created.
