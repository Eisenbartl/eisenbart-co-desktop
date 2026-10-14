CREATE TABLE recurring_review_history (
 id TEXT PRIMARY KEY NOT NULL, client_id TEXT NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
 review_json TEXT NOT NULL CHECK(length(CAST(review_json AS BLOB))<=262144),
 FOREIGN KEY(client_id,organization_id) REFERENCES clients(id,organization_id), FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
CREATE INDEX recurring_review_scope ON recurring_review_history(client_id,organization_id,counselor_id);
CREATE TRIGGER recurring_review_no_update BEFORE UPDATE ON recurring_review_history BEGIN SELECT RAISE(ABORT,'Payment review history is append-only'); END;
CREATE TRIGGER recurring_review_no_delete BEFORE DELETE ON recurring_review_history BEGIN SELECT RAISE(ABORT,'Payment review history is append-only'); END;
CREATE TABLE audit_events_v10 (
 id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored','csv_imported','pdf_imported','transaction_reviewed','transfers_linked','recurring_reviewed')),
 entity_id TEXT,created_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
INSERT INTO audit_events_v10 SELECT * FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_v10 RENAME TO audit_events;
CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
PRAGMA user_version=10;
