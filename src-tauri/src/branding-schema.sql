CREATE TABLE firm_branding_history (
 organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,revision INTEGER NOT NULL CHECK(revision BETWEEN 1 AND 100),
 data_json TEXT NOT NULL CHECK(length(CAST(data_json AS BLOB))<=131072),updated_at TEXT NOT NULL,
 PRIMARY KEY(organization_id,revision),
 FOREIGN KEY(organization_id) REFERENCES organizations(id),
 FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
CREATE TRIGGER firm_branding_no_update BEFORE UPDATE ON firm_branding_history BEGIN SELECT RAISE(ABORT,'Firm branding history is append-only'); END;
CREATE TRIGGER firm_branding_no_delete BEFORE DELETE ON firm_branding_history BEGIN SELECT RAISE(ABORT,'Firm branding history is append-only'); END;
CREATE TABLE audit_events_v20 (
 id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored','csv_imported','pdf_imported','transaction_reviewed','transfers_linked','recurring_reviewed','statement_balance_reviewed','financial_facts_reviewed','assessment_recorded','action_plan_recorded','scenario_recorded','client_care_saved','document_collection_saved','statement_account_saved','firm_branding_saved')),
 entity_id TEXT,created_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
INSERT INTO audit_events_v20 SELECT * FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_v20 RENAME TO audit_events;
CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
PRAGMA user_version=20;
