# Unified transaction review and document inspection

Open a client’s **Review & reconciliation → All transactions**. This is a current
transaction table over the existing bank/card records, not a second transaction
store. Each source record appears once using its latest financial decision.
**Activity history** still shows every import and review event separately.

The normal table uses the application’s navy/white/slate/gold language. Optional
inspection appears beneath it. Close inspection to return to the normal table;
opening a source never reserves half the workspace permanently.

## Accounts, dates, filters and evidence

Checking/savings and credit-card statements are supported. A reviewed savings
account can be identified as a money-market deposit or other deposit subtype;
the original source classification remains unchanged. The institution and last
four digits can be recorded explicitly under **Review account details**. Only
those last four digits are accepted, and unknown fields stay unknown. Unassigned
card sources remain clearly separate until existing card inventory assigns them
a durable reviewed identity.

Loan, mortgage and personal-credit-line transaction adapters are unavailable.
The corresponding filter returns no unsupported records. Personal debt plans and
verified balance inventories are not transaction ledgers and are never converted
to invented loan transactions. Separate actual interest/fee records can be
classified in existing review; missing principal/interest splits are not guessed.
All currently supported statement transactions require verified USD. This stage
does not add OCR, foreign-exchange conversion or general institution recognition.

Most-recent transaction date is the default. Oldest-first and recorded-posting-date
sorting are available. Same-day ties use stable internal row IDs; no times are
invented. Both dates appear separately. Unknown dates sort last and do not match
a date-range filter. Posting dates are supplemental reviewed details because the
existing adapters do not extract them reliably.

Combine account type, institution, individual account, source, dates, financial
kind, detailed method, category, direction, absolute amount range, current status
and reconciliation filters. Search is literal and includes descriptions, merchant,
account/source names, institution, masked identifier and internal row ID. Draft
filters take effect only with **Apply transaction filters**. Applied criteria are
shown explicitly. Reset returns to all supported current rows. Paging uses bounded
25-row pages and rejects stale context after a source/review/detail change.

The table preserves original source identity and provides original fields, full
financial decision history and supplemental-detail history from **Inspect /
review → Original values & history**. Corrected descriptions, dates, amounts,
categories and financial kinds remain distinct from the original imported values.

Bank amounts retain their cash direction. For the combined table, card debt
increases/purchases are negative and liability reductions/payments/refunds are
positive. The direction label and account type are separate: a card payment's
positive liability reduction is not a checking deposit. The original signed card
liability amount remains visible in evidence and the original review form.

## Optional original PDF inspection

Choose **Open original document** after selecting a row. Authorized retained PDF
bytes are read from SQLCipher with scoped client/source and SHA-256 checks. They
are not exported to temporary plaintext files or external viewers/services. If
original bytes were not retained, or the source is CSV, captured original fields
remain available with an explicit explanation; no original PDF is fabricated.

The bundled PDF.js 6.4.299 worker renders canvas pages. Page navigation and zoom
are provided. A reliable recorded page opens directly; unknown location opens
page 1 with an explicit warning. Exact highlights are unavailable because intake
does not capture source coordinates. Rendering provides no PDF scripting,
attachments, external links, forms or XFA activation. Font/CMap assets are local
and allowlisted. The CSP permits bundled workers and local assets, not remote
financial-document services. No active PDF content is handed to a browser plugin.

Existing intake limits remain 2 MiB / 100 pages, with the supervised Rust parser
unchanged. The viewer additionally limits images/canvas area and rendering time.
Rendering errors are visible while captured evidence stays available. Oversized
images can be omitted; the panel discloses that limit and does not establish
statement completeness.
The split width is adjustable on a wide window; narrower windows stack panels.
Lock or navigation destroys the viewer and clears the canvas. Session inspection,
filters and drafts are not put into URLs or browser storage.

## Financial decisions and supplemental details

**Correct / approve transaction** embeds the existing bank/card decision form.
It uses the same authoritative APIs, warning acknowledgments, revision checks,
atomic Necessary/Discretionary purpose workflow and immutable review history as
the original queue. No alternative approval service or automatic approval is
introduced. Refunds, internal transfers and liability payments retain their
existing financial treatment. Transfer pairing remains in **Transfer matching**.

**Edit method & posting date** records verified detailed method and optional
posting date, with reason and acknowledgment. Checking expenses are not assumed
to be debit-card purchases. A method must agree with the financial kind, direction
and supported account type. Correct financial fields through the original review
form first when they conflict. Cash withdrawals and debt payments are movements,
not automatically new consumer purchases.

Suspected duplicates stay visible and are excluded from method analysis. To mark
a **confirmed duplicate**, first reject it through the existing financial review,
then record that confirmation in supplemental details with its evidence/reason.
This does not delete the source or manufacture an approval. The table/filter
separately identifies confirmed rejected duplicates. Reapproving/correcting a row
invalidates its old supplemental confirmation and method/posting binding; the
older entry remains in history.

Supplemental details use an append-only scoped schema-32 journal with contiguous
revisions, canonical payloads, digest chaining and existing audit events. They
share the financial revision, stale-write protection and atomic audit rollback.
Approved-source changes make earlier transaction details inactive until verified
again. Existing planning inputs are not silently rewritten. Live analyses reread
current decisions; immutable reports keep their original values and require
renewed review when their source context changes. Encrypted recovery validates
supplemental history before activation.

## Purchase-method observations

Expand **Payment methods & observed patterns**. Calculations use all rows matching
the applied filters, not only the visible page. Rust uses integer cents; averages
round half a cent upward and percentages truncate to two decimals. Rounded shares
can therefore sum below 100%.

Only current approved, categorized, nonduplicate rows from arithmetically matched
statements contribute. Missing balances, pending decisions or discrepancies
exclude source observations rather than imply completeness. Running-balance/order
conflicts also exclude that statement. The count of excluded rows remains visible.
Statement arithmetic alone does not prove complete account or period coverage;
existing **Completeness & final validation** checks gaps/overlaps/expected sources.

Only actual expense/purchase classifications supply purchase value. Card
repayments, internal transfers, loan movements, refunds, fees and interest do not
inflate purchases. Refunds/net spending remain in the existing Spending Analysis.
The purchase mix shows credit cards, explicitly reviewed checking-linked debit
cards, other reviewed purchase methods, and unknown methods. Unknown value stays
in the denominator and its amount/share remains visible. A complete card/debit
comparison uses a documented 60% threshold; a 40–60% split is described as balanced
within these records. Other/unknown methods prevent an overall preference claim.
Credit use alone never establishes financial health or spending necessity.

Observations include actual reviewed overdraft/NSF fees, interest charges, cash
withdrawals, existing current counselor-confirmed subscriptions, category amounts
and explicitly discretionary small-purchase frequency. No merchant category is
automatically declared unnecessary. When both date bounds are supplied, an
adjacent equal-length period is shown with its own counts/averages/shares; missing
coverage can limit comparison. These are discussion prompts, not character
judgments, predictions or guaranteed savings.

## Verification and limits

See [development status](development-status.md) for executed source/native checks
and the [implementation checklist](unified-transaction-review-checklist.md) for
baseline reuse and dependencies. Existing bounded imports (5,000 bank rows,
20 card sources × 200 rows) and history limits remain. Supplemental details allow
10,000 events per client, at most 8 KiB per event. This remains a development build;
Windows installed acceptance is separate. No installer is created or provided.
