CREATE TABLE statement_balance_reviews (
 id TEXT PRIMARY KEY NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,document_id TEXT NOT NULL,
 row_order TEXT CHECK(row_order IN ('oldestFirst','newestFirst')),reason TEXT NOT NULL CHECK(length(reason) BETWEEN 1 AND 500),reviewed_at TEXT NOT NULL,counselor_name TEXT NOT NULL CHECK(length(counselor_name) BETWEEN 1 AND 120),
 FOREIGN KEY(document_id,client_id,organization_id,counselor_id) REFERENCES import_documents(id,client_id,organization_id,counselor_id));
CREATE INDEX statement_balance_scope ON statement_balance_reviews(client_id,organization_id,counselor_id);
CREATE TRIGGER statement_balance_no_update BEFORE UPDATE ON statement_balance_reviews BEGIN SELECT RAISE(ABORT,'Statement balance reviews are append-only'); END;
CREATE TRIGGER statement_balance_no_delete BEFORE DELETE ON statement_balance_reviews BEGIN SELECT RAISE(ABORT,'Statement balance reviews are append-only'); END;
CREATE TABLE audit_events_v12 (
 id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored','csv_imported','pdf_imported','transaction_reviewed','transfers_linked','recurring_reviewed','statement_balance_reviewed')),
 entity_id TEXT,created_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
INSERT INTO audit_events_v12 SELECT * FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_v12 RENAME TO audit_events;
CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
PRAGMA user_version=12;
