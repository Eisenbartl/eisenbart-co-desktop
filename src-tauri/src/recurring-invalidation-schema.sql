CREATE TABLE recurring_review_invalidations (
 review_id TEXT PRIMARY KEY NOT NULL, changed_revision INTEGER NOT NULL CHECK(changed_revision BETWEEN 1 AND 2147483647),
 FOREIGN KEY(review_id) REFERENCES recurring_review_history(id));
CREATE TRIGGER recurring_invalidations_no_update BEFORE UPDATE ON recurring_review_invalidations BEGIN SELECT RAISE(ABORT,'Payment evidence changes are append-only'); END;
CREATE TRIGGER recurring_invalidations_no_delete BEFORE DELETE ON recurring_review_invalidations BEGIN SELECT RAISE(ABORT,'Payment evidence changes are append-only'); END;
-- Decisions saved by the first schema-10 source checkpoint must be reviewed
-- once after upgrade; past transient evidence changes were not recorded there.
INSERT INTO recurring_review_invalidations(review_id,changed_revision)
 SELECT h.id,s.revision FROM recurring_review_history h JOIN client_import_state s
 ON s.client_id=h.client_id AND s.organization_id=h.organization_id AND s.counselor_id=h.counselor_id;
PRAGMA user_version=11;
