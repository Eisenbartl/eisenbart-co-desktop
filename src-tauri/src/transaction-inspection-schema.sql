CREATE TABLE transaction_inspection_details (
 client_id TEXT NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision BETWEEN 1 AND 10000),
 payload_json TEXT NOT NULL CHECK(length(CAST(payload_json AS BLOB))<=8192),digest TEXT NOT NULL,
 PRIMARY KEY(client_id,organization_id,counselor_id,revision),
 FOREIGN KEY(client_id,organization_id) REFERENCES clients(id,organization_id),
 FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
CREATE TRIGGER inspection_details_no_update BEFORE UPDATE ON transaction_inspection_details BEGIN SELECT RAISE(ABORT,'Inspection details are append-only'); END;
CREATE TRIGGER inspection_details_no_delete BEFORE DELETE ON transaction_inspection_details BEGIN SELECT RAISE(ABORT,'Inspection details are append-only'); END;
PRAGMA user_version=32;
