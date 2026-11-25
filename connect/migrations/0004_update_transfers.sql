CREATE TABLE update_transfers (
  id TEXT PRIMARY KEY,
  write_hash TEXT NOT NULL,
  expires INTEGER NOT NULL,
  cipher TEXT
);
CREATE INDEX update_transfers_expiry ON update_transfers(expires);
