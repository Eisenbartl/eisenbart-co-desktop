# Firm and report branding

Open Settings and Administration → Branding. Edit the firm's display name,
optional address, email, phone, website and disclosure. Choose or enter a local
PNG path, inspect it, review the draft preview and save explicitly. Clear optional
fields or remove the logo and save to stop including them in future captures.
The workspace name updates after save and unlock; the application's product name
and product mark remain separate from the firm's presentation.

Use a static PNG up to 256 × 256 pixels and 64 KiB. Rust bounds file reads and PNG
decoding, rejects animation/invalid images, flattens transparency onto white and
re-encodes RGB pixels without source metadata. Only that normalized PNG is saved
inside SQLCipher; its original path is not retained. Website text is displayed
and printed without fetching it. SVG, JPEG, remote logos, themes and native
printing are not part of this stage.

Limits: name 120 characters, address 500, email/website 254 each, phone 80,
disclosure 2,000; 100 saved branding versions, at most 128 KiB per version.
Plain text must be representable by the bundled PDF font. No signature,
certification or legal approval is implied by a firm-entered disclosure. Existing
financial disclosures remain in every relevant report.

## Captured presentation

The report form loads and displays the current saved branding alongside its
financial basis. Refresh firm branding preview after a conflict, review it and
confirm the assessment basis again. Once branding is configured, Rust requires
its expected revision before recording a new report. Concurrent changes reject
the write and retain the interface draft; they never silently substitute branding.

Assessment format 6 captures the exact scoped firm version, timestamp, contact
text, disclosure and normalized logo. New client plans inherit that presentation
through their immutable parent assessment, without changing client-plan format 1
or duplicating confidential internal evidence. Both PDF types embed the captured
logo as bounded RGB pixels and print the original firm details entirely offline.

Later firm edits flag previous reports and linked client plans for review, even
when values are reverted. Original payloads, checksums, financial amounts, source
associations and presentation remain historical. Exporting an older report never
uses today's logo/contact details. Formats 1–5 omit the new optional JSON field,
retain their original serialization and continue to validate/export/recover.
Unconfigured workspaces keep their initial firm name and existing report formats.

## Storage and verification

Schema 20 adds append-only organization-scoped `firm_branding_history`. Writes
use authenticated workspace identity, expected revision, an immediate transaction,
organization-name update and atomic audit. A failed audit rolls back both the
name and the new history version. Session access reloads the authenticated firm's
name so another window cannot keep writing a stale cached identity. No caller can
choose another organization through this API; counselor/role administration stays
planned for a separately implemented permission model.

Reads validate bounded, continuous history and canonical image/text data. Report
reads/export and restore compare each captured snapshot against its original
organization-scoped history, so a rehashed forged snapshot still fails. Restore
validates firm history even in a workspace with no clients. Migrations remain
transactional, and encrypted backups include branding and captured logos.

The tests cover canonical PNG pixels/alpha, stripped metadata, invalid/oversized
files and text, authenticated access, optimistic conflicts, audit rollback,
second-window name refresh, migration rollback, corrupted history, forged capture
bindings, retained legacy bytes/digests, unchanged financial plans/facts, report
and client-plan PDF content/image resources, and encrypted round trips. Native
checks exercise visible settings/report controls, restart and recovery with
captured branding. Windows source checks and installer acceptance are recorded
separately in development status.

Future work: additional image formats/resizing after a bounded parser review,
report themes/print acceptance, default spending categories, counseling templates
and broader organization administration. These must preserve historical report
presentation and mandatory disclosures.
