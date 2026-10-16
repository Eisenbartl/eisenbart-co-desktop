CREATE TABLE category_rule_versions (
 rule_id TEXT NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision BETWEEN 1 AND 100),account_id TEXT NOT NULL,
 payload_json TEXT NOT NULL CHECK(length(CAST(payload_json AS BLOB))<=8192),digest TEXT NOT NULL CHECK(length(digest)=64),
 PRIMARY KEY(rule_id,client_id,organization_id,counselor_id,revision),
 FOREIGN KEY(account_id,client_id,organization_id,counselor_id) REFERENCES statement_accounts(id,client_id,organization_id,counselor_id));
CREATE TRIGGER category_rule_versions_no_update BEFORE UPDATE ON category_rule_versions BEGIN SELECT RAISE(ABORT,'Rule history is immutable'); END;
CREATE TRIGGER category_rule_versions_no_delete BEFORE DELETE ON category_rule_versions BEGIN SELECT RAISE(ABORT,'Archive rules instead of deleting history'); END;
CREATE UNIQUE INDEX review_history_rule_scope ON transaction_review_history(id,record_id,client_id,organization_id,counselor_id);
CREATE TABLE category_rule_applications (
 decision_id TEXT PRIMARY KEY NOT NULL,record_id TEXT NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 rule_id TEXT NOT NULL,rule_revision INTEGER NOT NULL,source_snapshot TEXT NOT NULL CHECK(length(source_snapshot)=64),
 FOREIGN KEY(rule_id,client_id,organization_id,counselor_id,rule_revision) REFERENCES category_rule_versions(rule_id,client_id,organization_id,counselor_id,revision),
 FOREIGN KEY(decision_id,record_id,client_id,organization_id,counselor_id) REFERENCES transaction_review_history(id,record_id,client_id,organization_id,counselor_id));
CREATE TRIGGER category_rule_applications_no_update BEFORE UPDATE ON category_rule_applications BEGIN SELECT RAISE(ABORT,'Rule applications are immutable'); END;
CREATE TRIGGER category_rule_applications_no_delete BEFORE DELETE ON category_rule_applications BEGIN SELECT RAISE(ABORT,'Rule applications are immutable'); END;
CREATE TABLE audit_events_v26 (
 id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored','csv_imported','pdf_imported','transaction_reviewed','transfers_linked','recurring_reviewed','statement_balance_reviewed','financial_facts_reviewed','assessment_recorded','action_plan_recorded','scenario_recorded','client_care_saved','document_collection_saved','statement_account_saved','firm_branding_saved','coverage_review_saved','final_report_approved','card_statement_imported','card_transaction_reviewed','card_review_saved','card_facts_saved','category_rule_saved','category_rule_applied')),
 entity_id TEXT,created_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
INSERT INTO audit_events_v26 SELECT * FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_v26 RENAME TO audit_events;
CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
PRAGMA user_version=26;
