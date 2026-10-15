CREATE UNIQUE INDEX IF NOT EXISTS assessment_final_identity ON client_assessment_records(id,client_id,organization_id,counselor_id);
CREATE TABLE final_report_approvals (
 id TEXT PRIMARY KEY NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 assessment_id TEXT NOT NULL UNIQUE,approved_at TEXT NOT NULL,payload_json TEXT NOT NULL CHECK(length(CAST(payload_json AS BLOB))<=65536),digest TEXT NOT NULL,
 FOREIGN KEY(client_id,organization_id) REFERENCES clients(id,organization_id),
 FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id),
 FOREIGN KEY(assessment_id,client_id,organization_id,counselor_id) REFERENCES client_assessment_records(id,client_id,organization_id,counselor_id));
CREATE TRIGGER final_reports_no_update BEFORE UPDATE ON final_report_approvals BEGIN SELECT RAISE(ABORT,'Final approvals are append-only'); END;
CREATE TRIGGER final_reports_no_delete BEFORE DELETE ON final_report_approvals BEGIN SELECT RAISE(ABORT,'Final approvals are append-only'); END;
CREATE TABLE audit_events_v22 (
 id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored','csv_imported','pdf_imported','transaction_reviewed','transfers_linked','recurring_reviewed','statement_balance_reviewed','financial_facts_reviewed','assessment_recorded','action_plan_recorded','scenario_recorded','client_care_saved','document_collection_saved','statement_account_saved','firm_branding_saved','coverage_review_saved','final_report_approved')),
 entity_id TEXT,created_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
INSERT INTO audit_events_v22 SELECT * FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_v22 RENAME TO audit_events;
CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
PRAGMA user_version=22;
