CREATE TABLE client_import_state (
 client_id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision BETWEEN 1 AND 2147483647),
 FOREIGN KEY(client_id,organization_id) REFERENCES clients(id,organization_id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id)
);
CREATE TABLE statement_accounts (
 id TEXT PRIMARY KEY NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 label TEXT NOT NULL,kind TEXT NOT NULL CHECK(kind IN ('checking','savings')),
 UNIQUE(id,client_id,organization_id,counselor_id),
 FOREIGN KEY(client_id,organization_id) REFERENCES clients(id,organization_id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id)
);
CREATE TABLE import_documents (
 id TEXT PRIMARY KEY NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,account_id TEXT NOT NULL,
 file_name TEXT NOT NULL,source_hash TEXT NOT NULL,byte_count INTEGER NOT NULL CHECK(byte_count BETWEEN 1 AND 2097152),imported_at TEXT NOT NULL,
 input_json TEXT NOT NULL CHECK(length(input_json)<=16384),source_blob BLOB CHECK(source_blob IS NULL OR length(source_blob)<=2097152),
 UNIQUE(id,client_id,organization_id,counselor_id),UNIQUE(client_id,account_id,source_hash),
 FOREIGN KEY(account_id,client_id,organization_id,counselor_id) REFERENCES statement_accounts(id,client_id,organization_id,counselor_id)
);
CREATE TABLE import_records (
 id TEXT PRIMARY KEY NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,document_id TEXT NOT NULL,
 raw_json TEXT NOT NULL CHECK(length(raw_json)<=32768),
 UNIQUE(id,client_id,organization_id,counselor_id),
 FOREIGN KEY(document_id,client_id,organization_id,counselor_id) REFERENCES import_documents(id,client_id,organization_id,counselor_id)
);
CREATE TABLE transaction_choices (
 record_id TEXT PRIMARY KEY NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('Pending','Needs Clarification','Approved','Rejected')),
 fields_json TEXT NOT NULL CHECK(length(fields_json)<=4096),linked_id TEXT,reason TEXT NOT NULL,
 FOREIGN KEY(record_id,client_id,organization_id,counselor_id) REFERENCES import_records(id,client_id,organization_id,counselor_id),
 FOREIGN KEY(linked_id,client_id,organization_id,counselor_id) REFERENCES import_records(id,client_id,organization_id,counselor_id)
);
CREATE TABLE transaction_review_history (
 id TEXT PRIMARY KEY NOT NULL,record_id TEXT NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 counselor_name TEXT NOT NULL,from_status TEXT NOT NULL,to_status TEXT NOT NULL,
 from_fields_json TEXT NOT NULL,to_fields_json TEXT NOT NULL,from_linked_id TEXT,to_linked_id TEXT,reason TEXT NOT NULL,reviewed_at TEXT NOT NULL,
 FOREIGN KEY(record_id,client_id,organization_id,counselor_id) REFERENCES import_records(id,client_id,organization_id,counselor_id)
);
CREATE INDEX import_documents_scope ON import_documents(organization_id,counselor_id,client_id);
CREATE INDEX import_records_scope ON import_records(organization_id,counselor_id,client_id);
CREATE INDEX review_history_scope ON transaction_review_history(organization_id,counselor_id,client_id,reviewed_at);
CREATE TRIGGER source_documents_no_update BEFORE UPDATE ON import_documents BEGIN SELECT RAISE(ABORT,'Sources are immutable'); END;
CREATE TRIGGER source_documents_no_delete BEFORE DELETE ON import_documents BEGIN SELECT RAISE(ABORT,'Sources are immutable'); END;
CREATE TRIGGER source_records_no_update BEFORE UPDATE ON import_records BEGIN SELECT RAISE(ABORT,'Source rows are immutable'); END;
CREATE TRIGGER source_records_no_delete BEFORE DELETE ON import_records BEGIN SELECT RAISE(ABORT,'Source rows are immutable'); END;
CREATE TRIGGER review_history_no_update BEFORE UPDATE ON transaction_review_history BEGIN SELECT RAISE(ABORT,'Review history is immutable'); END;
CREATE TRIGGER review_history_no_delete BEFORE DELETE ON transaction_review_history BEGIN SELECT RAISE(ABORT,'Review history is immutable'); END;
CREATE TABLE audit_events_v8 (
 id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored','csv_imported','transaction_reviewed','transfers_linked')),
 entity_id TEXT,created_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
INSERT INTO audit_events_v8 SELECT * FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_v8 RENAME TO audit_events;
CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
PRAGMA user_version=8;
