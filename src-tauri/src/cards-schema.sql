CREATE TABLE card_statements (
 id TEXT PRIMARY KEY NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 payload_json TEXT NOT NULL CHECK(length(CAST(payload_json AS BLOB))<=1048576),digest TEXT NOT NULL,
 source_hash TEXT NOT NULL,source_blob BLOB CHECK(source_blob IS NULL OR length(source_blob)<=262144),
 UNIQUE(id,client_id,organization_id,counselor_id),UNIQUE(client_id,source_hash),
 FOREIGN KEY(client_id,organization_id) REFERENCES clients(id,organization_id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
CREATE TABLE card_decisions (
 id TEXT PRIMARY KEY NOT NULL,statement_id TEXT NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 row_index INTEGER NOT NULL CHECK(row_index BETWEEN 0 AND 199),fields_json TEXT NOT NULL CHECK(length(CAST(fields_json AS BLOB))<=8192),
 status TEXT NOT NULL CHECK(status IN ('Pending','Needs Clarification','Approved','Rejected')),reason TEXT NOT NULL,reviewed_at TEXT NOT NULL,counselor_name TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision>0),UNIQUE(client_id,revision),
 FOREIGN KEY(statement_id,client_id,organization_id,counselor_id) REFERENCES card_statements(id,client_id,organization_id,counselor_id));
CREATE TRIGGER card_statements_no_update BEFORE UPDATE ON card_statements BEGIN SELECT RAISE(ABORT,'Card sources are immutable'); END;
CREATE TRIGGER card_statements_no_delete BEFORE DELETE ON card_statements BEGIN SELECT RAISE(ABORT,'Card sources are immutable'); END;
CREATE TRIGGER card_decisions_no_update BEFORE UPDATE ON card_decisions BEGIN SELECT RAISE(ABORT,'Card decisions are append-only'); END;
CREATE TRIGGER card_decisions_no_delete BEFORE DELETE ON card_decisions BEGIN SELECT RAISE(ABORT,'Card decisions are append-only'); END;
CREATE TABLE audit_events_v23 (
 id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored','csv_imported','pdf_imported','transaction_reviewed','transfers_linked','recurring_reviewed','statement_balance_reviewed','financial_facts_reviewed','assessment_recorded','action_plan_recorded','scenario_recorded','client_care_saved','document_collection_saved','statement_account_saved','firm_branding_saved','coverage_review_saved','final_report_approved','card_statement_imported','card_transaction_reviewed')),
 entity_id TEXT,created_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
INSERT INTO audit_events_v23 SELECT * FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_v23 RENAME TO audit_events;
CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
PRAGMA user_version=23;
