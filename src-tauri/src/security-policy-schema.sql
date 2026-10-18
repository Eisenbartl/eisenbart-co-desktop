CREATE TABLE workspace_security_history (
 revision INTEGER PRIMARY KEY NOT NULL CHECK(revision BETWEEN 1 AND 1000),
 organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 idle_minutes INTEGER NOT NULL CHECK(idle_minutes BETWEEN 1 AND 60),
 saved_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),
 FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
CREATE TRIGGER workspace_security_no_update BEFORE UPDATE ON workspace_security_history BEGIN SELECT RAISE(ABORT,'Security history is immutable'); END;
CREATE TRIGGER workspace_security_no_delete BEFORE DELETE ON workspace_security_history BEGIN SELECT RAISE(ABORT,'Security history is immutable'); END;
CREATE TABLE audit_events_v29 (
 id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored','csv_imported','pdf_imported','transaction_reviewed','transfers_linked','recurring_reviewed','statement_balance_reviewed','financial_facts_reviewed','assessment_recorded','action_plan_recorded','scenario_recorded','client_care_saved','document_collection_saved','statement_account_saved','firm_branding_saved','coverage_review_saved','final_report_approved','card_statement_imported','card_transaction_reviewed','card_review_saved','card_facts_saved','category_rule_saved','category_rule_applied','note_amended','workspace_password_changed','security_policy_saved')),
 entity_id TEXT,created_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
INSERT INTO audit_events_v29 SELECT * FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_v29 RENAME TO audit_events;
CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
PRAGMA user_version=29;
