# Card-inclusive final counselor approval — 0.37.0

Reports → Final report checklist supports the separately versioned
`counselor-final-report-v2` policy for format-9 assessments containing matching
bank completeness, bank/card review and saved card completeness evidence. A
successful readiness review still requires explicit professional approval. This
is a development milestone, not installed-Windows or commercial acceptance.

## Workflow

1. Authorize retention of original bank/card sources when importing. Finish
   individual source decisions and reconcile original and approved balances.
2. In Financial Review → Bank & card activity, review durable accounts, statement
   assignments and exact repayment counterparts. Renew stale links explicitly;
   neither matching descriptions nor unchanged amounts renew an approval.
3. In Working tools, verify the full dated household balance inventory, bank
   income/spending roles and known required monthly payments. Record each debt
   once. In Card completeness, review full-period applicability, source coverage,
   approval-bound roles and comparisons to those existing debts; save a new version.
4. Save a current bank completeness review for the same months, including all
   expected bank accounts and selected sources. Describe the separately reviewed
   card inventory and any missing obligations truthfully. Missing evidence blocks
   approval; an explanation is not a waiver.
5. In Reports → Working tools, save an assessment with all three reviewed
   supplements: saved completeness context, bank/card supplement, saved card
   completeness. The original assessment and its bank-only metrics stay intact. The final PDF
   labels those spending/ratio estimates as bank-only supporting evidence.
6. Select it in Final report checklist and review readiness. Inspect the combined
   figures, each bank/card scope decision, source warnings and assumptions. Record
   professional notes and a separate card/repayment review, then confirm all five
   review statements and explicitly approve.
7. Export to a private new local PDF filename. Refreshing readiness retains notes
   but clears confirmations; failed writes retain drafts and do not retry. Locking
   clears private approval fields. Later changes require a new assessment/approval.

## Supported policy and exact figures

V2 retains every original bank-only financial/source/completeness gate, recent
completed-period rule (90 days, America/Chicago), authenticated parent/digest
binding and required counselor decisions. It replaces the unconditional card
block with additional required checks:

- Complete current captured card inventory, closing-debt comparisons, roles and
  source choices, with no unresolved card-evidence or original calculator issues.
- Full-period applicable cards or explicitly explained nonapplicability. Early
  opening/closure balance semantics are not supported by this policy.
- Every relevant card source selected, inside the assessment period, with dated
  source-wide completed row decisions. Approved activity cannot disappear through
  a source selection or declared date window.
- Current exact bank income/spending roles checked per row. Necessary and
  discretionary amounts must agree with original bank/card subtotals.
- Every bank balance movement linked to its exact current approved card repayment
  in the same period. Transfers need reciprocal approved in-period legs.
- Every recorded debt has a known required monthly payment and a distinct
  applicable-card mapping. Other loan/debt obligations require a separate supported
  evidence workflow. Never omit a known obligation to obtain approval.

Split/cross-period repayments, card credits, adjustments, early closure, unknown
obligations and unsupported bank-refund role treatments remain blocked. There is
no guessed allocation, automatic match, invented asset, or debt override. Full
real-world household completeness still requires inspecting original evidence and
professional judgment; these checks cannot discover unrecorded accounts.

The frozen `card-inclusive-final-v1` summary uses integer USD cents in Rust:

| Independently checked fictional example | USD |
|---|---:|
| Bank income | 2000.01 |
| Bank net spending | 1200.02 |
| Card spending (100.01 - 20.02 + 5.04 + 2.05) | 87.08 |
| Combined necessary spending (1200.02 - 12.93) | 1187.09 |
| Combined discretionary spending | 100.01 |
| Combined recorded spending | 1287.10 |
| Income less combined spending | 712.91 |
| Matched repayment, excluded from spending | 50.03 |
| Bank cash movement (2000.01 - 1200.02 - 50.03) | 749.96 |
| Existing verified household debt, counted once | 1037.05 |

Income less spending differs from bank cash movement because card purchases and
repayments have different timing. Card refunds offset their recorded role, even
when the role total becomes negative. Required monthly payments are verified
context, not fabricated historical expenses or a new repayment projection. The
summary is withheld when financial-basis checks fail. Context/age flags may still
show original captured figures; they never authorize a changed-context approval.

## Persistence, compatibility and recovery

Schema 25 is unchanged. Existing append-only scoped approval storage accepts new
format-2/v2 approval JSON, its independently recomputed summary and explicit card
decision. Original format-1/v1 policy, receipts, optional-field omission and report
formats 1–9 bytes remain compatible. Older format-8 supplements remain provisional;
bank-only reports with separately stored cards stay blocked. A matching format-9
assessment no longer receives the unconditional provisional context flag; actual
later source/profile/firm/review changes still flag it.

Approval rechecks the receipt/date and all evidence in an immediate transaction,
renders the complete PDF before saving, and audits atomically. Reads/recovery
dispatch by exact policy and format, reconstruct the original scoped parent and
recompute figures/gates at the original approval date. Rehashed altered summaries,
wrong policy/format combinations, foreign parents and mismatched decisions fail.
Later review flags never change original approval bytes or figures. Old supplement
disclosures remain historical; the separate v2 summary clearly states its basis.
SHA-256 is an integrity check, not a digital signature or independent certification.

The existing 50-approval/64-KiB limits apply. A client with unusually extensive
scope/notes can exceed that limit; the write fails without replacing records.
Exported PDFs are readable files outside workspace encryption. No new dependency,
network service, schema migration or installer is introduced.

Executed results are recorded in [development-status.md](development-status.md).
The next implementation priority is searchable client-wide transaction history
over existing bank/card decisions, with filters, pagination, provenance and scope
tests. Broader loan/paystub/card-PDF adapters, OCR, direct printing, advanced models,
multiple counselors and security/commercial/installed-Windows acceptance remain
separate original requirements.
