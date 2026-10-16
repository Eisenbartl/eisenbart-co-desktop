CREATE UNIQUE INDEX note_amendment_scope ON client_notes(id,client_id,organization_id,counselor_id);
CREATE TABLE note_amendments (
 root_id TEXT NOT NULL,revision INTEGER NOT NULL CHECK(revision BETWEEN 1 AND 100),previous_id TEXT NOT NULL UNIQUE,note_id TEXT NOT NULL UNIQUE,
 client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,reason TEXT NOT NULL CHECK(length(CAST(reason AS BLOB))<=2000),
 PRIMARY KEY(root_id,client_id,organization_id,counselor_id,revision),
 FOREIGN KEY(root_id,client_id,organization_id,counselor_id) REFERENCES client_notes(id,client_id,organization_id,counselor_id),
 FOREIGN KEY(previous_id,client_id,organization_id,counselor_id) REFERENCES client_notes(id,client_id,organization_id,counselor_id),
 FOREIGN KEY(note_id,client_id,organization_id,counselor_id) REFERENCES client_notes(id,client_id,organization_id,counselor_id));
CREATE TRIGGER note_amendments_no_update BEFORE UPDATE ON note_amendments BEGIN SELECT RAISE(ABORT,'Note corrections are immutable'); END;
CREATE TRIGGER note_amendments_no_delete BEFORE DELETE ON note_amendments BEGIN SELECT RAISE(ABORT,'Note corrections are immutable'); END;
CREATE TABLE audit_events_v27 (
 id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored','csv_imported','pdf_imported','transaction_reviewed','transfers_linked','recurring_reviewed','statement_balance_reviewed','financial_facts_reviewed','assessment_recorded','action_plan_recorded','scenario_recorded','client_care_saved','document_collection_saved','statement_account_saved','firm_branding_saved','coverage_review_saved','final_report_approved','card_statement_imported','card_transaction_reviewed','card_review_saved','card_facts_saved','category_rule_saved','category_rule_applied','note_amended')),
 entity_id TEXT,created_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
INSERT INTO audit_events_v27 SELECT * FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_v27 RENAME TO audit_events;
CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
PRAGMA user_version=27;
