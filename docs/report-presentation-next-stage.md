# Next stage: readable report presentation

This is a development note, not implemented functionality. The original
specification and prototype map remain governing scope.

## Concrete problem and existing entry points

`assessment_pdf.rs::render_report` currently prints the final approval ID,
checksums, every readiness gate and account-scope decisions before the original
assessment's client/period information. Long approval evidence can occupy the
first several pages. Both v1 and v2 approvals already export correct captured
figures; do not repeat their approval workflow or change the calculators.

The existing `Layout` handles embedded offline typefaces, wrapping, letter-page
margins, footers and bounded output. `render_final_report` validates the immutable
approval before rendering; `render_action_plan` has a separate client-facing
privacy boundary. `FinalReportApproval.tsx` and `AssessmentRecords.tsx` use native
scoped PDF destination/export commands. Build on these entry points.

## Manageable next implementation

Put firm identity, client name, period, approval/historical status and a readable
financial summary first. Distinguish bank-only v1 figures from card-inclusive v2
figures and keep bank cash movement separate from spending. Preserve unknowns,
original assumptions, limitations and historical-review flags. Move detailed
gates, identities, evidence/checksums and counselor scope decisions to a clearly
marked appendix. Do not copy private counselor evidence into client action plans.

A changed PDF layout must not rewrite stored assessment/approval payloads,
receipts, digests or financial policy versions. Decide explicitly how users can
retain the existing detailed export alongside an improved presentation, and
label any new presentation format separately from financial capture versions.

Verify actual PDF page text/order, all original independent cents, original
identity, long content/margins, old formats, v1/v2 approvals, later-context flags,
client-plan exclusions, original retained exports, scope/lock/no-overwrite,
restart and encrypted recovery. Inspect rendered pages, not only extracted text.
Use existing fixtures and tests rather than creating another calculation system.

Direct desktop printing is a separate native capability. Choose its bounded local
file lifecycle, cancellation/failure behavior and explicit user action before
implementation. No automatic printing, external viewer integration or Windows
printer acceptance is established by the current Linux smoke test. Do not claim
Windows printing complete until tested on Windows; do not create an installer
unless the user asks for one.
