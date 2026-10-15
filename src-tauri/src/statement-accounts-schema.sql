CREATE TABLE statement_account_history (
 account_id TEXT NOT NULL,client_id TEXT NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision BETWEEN 1 AND 500),
 label TEXT NOT NULL CHECK(length(label) BETWEEN 1 AND 120),archived INTEGER NOT NULL CHECK(archived IN (0,1)),updated_at TEXT NOT NULL,
 PRIMARY KEY(account_id,client_id,organization_id,counselor_id,revision),
 FOREIGN KEY(account_id,client_id,organization_id,counselor_id) REFERENCES statement_accounts(id,client_id,organization_id,counselor_id));
CREATE TRIGGER statement_account_history_no_update BEFORE UPDATE ON statement_account_history BEGIN SELECT RAISE(ABORT,'Account history is append-only'); END;
CREATE TRIGGER statement_account_history_no_delete BEFORE DELETE ON statement_account_history BEGIN SELECT RAISE(ABORT,'Account history is append-only'); END;
CREATE TRIGGER statement_accounts_no_update BEFORE UPDATE ON statement_accounts BEGIN SELECT RAISE(ABORT,'Original account identity is immutable'); END;
CREATE TRIGGER statement_accounts_no_delete BEFORE DELETE ON statement_accounts BEGIN SELECT RAISE(ABORT,'Archive accounts instead of deleting history'); END;
CREATE TABLE audit_events_v19 (
 id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
 event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored','csv_imported','pdf_imported','transaction_reviewed','transfers_linked','recurring_reviewed','statement_balance_reviewed','financial_facts_reviewed','assessment_recorded','action_plan_recorded','scenario_recorded','client_care_saved','document_collection_saved','statement_account_saved')),
 entity_id TEXT,created_at TEXT NOT NULL,
 FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
INSERT INTO audit_events_v19 SELECT * FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_v19 RENAME TO audit_events;
CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
PRAGMA user_version=19;
