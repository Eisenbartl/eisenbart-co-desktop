# Final counselor reports — 0.33.0, schema 22

Card-inclusive format-9 approval is now separately implemented by
[policy v2](card-inclusive-final-approval.md). The following describes the preserved
bank-only v1 policy and its original receipts.

Reports → Final report checklist now reviews readiness and records an explicit
counselor approval linked to an original immutable internal assessment. A separate
final-report PDF includes the original financial evidence, recorded scope,
professional decisions and limitations. Original internal reports and client
plans retain their existing formats and bytes.

## Working workflow

1. Finish source review, classifications and the dated verified balance inventory.
   Save a household completeness review for the intended assessment months.
2. In Reports → Working tools, record an assessment with that exact saved review.
   Include a reviewed planning scenario when applicable.
3. Open Final report checklist, select that assessment, and choose Review
   final-report readiness. Resolve every blocked check before approval. Changes
   to supporting information require a new assessment; old evidence stays intact.
4. Explain limited account periods, exceptions and scenario applicability. Record
   agreement on assumptions and professional approval notes. Review all four
   confirmations about the client, sources, household inventory and assumptions.
5. Approve final counselor report. The encrypted approval is append-only with an
   atomic audit event. The original report remains an internal assessment.
6. Choose a private local destination and a new filename to export the final PDF.
   It is readable confidential information outside workspace encryption. Existing
   files are never overwritten. Reopen through the same checklist to read the
   original approval or export it again.

Drafts participate in the application's navigation/client guards. Refreshing
readiness keeps notes but clears confirmations so they must be reconsidered.
Failed writes retain notes and never retry automatically. Locking removes private
approval data from the interface.

## Defined approval policy

Policy `counselor-final-report-v1` requires all of the following:

- The original client, source decisions, firm, selected completeness review and
  selected planning context still match the assessment.
- A completed assessment period ending before today and within 90 days, using
  America/Chicago for the review/report date. Historical internal exports remain
  available outside this window.
- Approved dated activity, no pending/clarification/undated activity, complete
  verified balances aligned to period end, current classifications and no
  unclassified income or spending.
- An attached reviewed household inventory with notes, no missing information,
  and no unprovided/unlinked expected accounts or obligations.
- Exactly one selected source covering every expected applicable account day.
  Limited periods need recorded explanations and a counselor decision. Both
  dates may be absent only for explicitly explained nonapplicability with no
  approved in-period activity. No account decision can hide approved activity.
- Every approved in-period transaction represented by a selected account source.
  Source choices never filter financial amounts.
- Applicable original sources retained with authorization, declared dates,
  zero whole-source and approved closing differences, completed source-wide row
  decisions and no running-balance conflicts.
- Explicit source/duplicate/exclusion review, scope decisions, professional notes
  and scenario applicability/assumptions agreement. If no scenario is captured,
  explain why it is not applicable; approval does not invent scenario advice.

These are deliberately bounded rules for the existing checking/savings evidence
model. A checkbox or exception cannot waive a failed financial gate. Automated
checks cannot establish that unrecorded accounts exist, verify a bank, inspect
all real-world evidence or replace counselor judgment. Arithmetic checks do not
auto-approve transactions. Original warnings, unknown ratios and hypothetical
planning assumptions remain disclosed in the final PDF.

## History, scope and recovery

Rust derives organization/counselor ownership from the authenticated session and
binds every approval to the scoped parent ID and SHA-256 digest. The readiness
receipt is rechecked inside the immediate approval transaction. A local date
change requires another review. Approval renders the full PDF in memory before
saving, so unsupported text/render limits cannot publish an unexportable report.
No financial result is accepted from the interface.

Schema 22 adds compound parent ownership links, append-only approval triggers and
`final_report_approved` audit history. Limits are 50 approvals per client and
64 KiB per approval, with bounded account notes (1,000 characters), assumptions
notes (2,000) and professional notes (4,000). There is one approval per assessment;
revisions require a new assessment and approval. Retention/deletion policy remains
separate future work.

Reads and encrypted recovery validate every stored approval's canonical policy,
original scope, decisions, dates and immutable parent. Rehashed forged gates are
rejected. Later supporting changes or a period older than 90 days add current
review flags while preserving original approval bytes and decisions. Historical
PDFs identify that original approval and the current review flags together.
SHA-256 checks stored bytes; it is not a digital signature or independent
certification. Final approval does not automatically finalize a client action plan
or establish commercial release readiness.

## Verification

Synthetic checks cover positive exact-cent approval, required/unknown evidence,
source selection, missing originals, limited dates, required exception decisions,
current context, freshness/date boundaries, counselor confirmations, scope,
receipt mismatches, duplicate approval, atomic audit failure, migration rollback,
immutability, PDF output/no overwrite, restart, encrypted recovery and forged
history. UI checks cover retained drafts, renewed confirmations, scope mismatches,
late replies, read-only historical approvals and global navigation/lock guards.
Live backend/native approval fixtures use the previous completed month so the
freshness policy does not expire the tests. The native workflow includes both an
incomplete blocked household and a separate complete example: USD 2,000.01 income, USD 1,200.02 spending, USD 799.99 operating
cash and USD 1,799.99 closing cash. It exports before and after changed context,
restart and encrypted restore. Executed results are in development-status.md.

Remaining work includes additional document types, OCR, advanced financial terms,
printing, multiple counselor permissions, security review and installed Windows
release acceptance. This source milestone creates no installer.
