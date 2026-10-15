# Statement account management

Open Documents → Statement accounts after selecting a client. Add a checking or
savings account before import, rename its current workflow name, or archive it.
Clear Archive and save to reactivate it. Account type is fixed at creation;
create a distinct account if another type is needed. The current limits are 20
accounts per client including archived accounts, and 500 metadata changes per
account. No balances, transactions or financial approvals are created here.

Current names appear in management and new-import selection. Financial review,
source associations, collection links and captured reports retain their original
account names and stable IDs. The management directory shows original names,
source counts and read-only change history in America/Chicago. Current and
original names remain reserved, case-insensitively, even for archived accounts.
There is no destructive deletion of saved account identities.

Schema 19 adds scoped append-only `statement_account_history` and protects
original `statement_accounts` against edits/deletion. Imported accounts start at
metadata version 0; their first change is version 1. Standalone creation writes
the original identity and version 1 together. Rust validates ownership, type,
name, bounds and expected revision in an immediate transaction with an audit
entry. A failed audit rolls the whole change back. Metadata updates use account
revisions; they do not change the import revision or invalidate financial
approvals. Creating an account adds to the financial source inventory and bumps
the import revision, so existing captures can correctly flag changed context.

CSV/PDF staging checks current archive status inside its write transaction,
including previews opened before another window archived the account. Reactivate
before importing another statement. Existing documents, reviews and collection
links remain accessible after archive. Source inputs still capture the original
account identity; renaming never rewrites their provenance or report bytes.

The UI retains failed drafts, guards navigation, disables conflicting operations,
checks response ownership/version and ignores replies from unmounted client
contexts. Recovery validates every account metadata revision, canonical names,
continuous history and name reservations before activation. Existing encrypted
backups migrate transactionally without changing original source/report payloads.

Future account work: institution/masked identifiers, other document account types,
explicit account coverage in final reports, and additional parser adapters. These
must keep immutable original evidence and historical capture validation. Planning
accounts stay independent until a separately reviewed mapping workflow exists.
