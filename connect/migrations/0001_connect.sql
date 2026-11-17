PRAGMA foreign_keys = ON;
CREATE TABLE accounts (
 id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, name TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','suspended')),
 salt TEXT NOT NULL, password_hash TEXT NOT NULL, created_at INTEGER NOT NULL,
 approved_at INTEGER, reset_hash TEXT, reset_expires INTEGER
);
CREATE TABLE sessions (hash TEXT PRIMARY KEY,user_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,expires INTEGER NOT NULL);
CREATE INDEX sessions_user ON sessions(user_id);
CREATE INDEX sessions_expiry ON sessions(expires);
CREATE TABLE devices (
 id TEXT PRIMARY KEY, token_hash TEXT NOT NULL UNIQUE,address TEXT NOT NULL UNIQUE,mac_hash TEXT,
 platform TEXT NOT NULL, owner_id TEXT REFERENCES accounts(id) ON DELETE SET NULL,
 label TEXT NOT NULL DEFAULT '',pair_hash TEXT,pair_expires INTEGER,created_at INTEGER NOT NULL
);
CREATE INDEX device_mac ON devices(mac_hash);
CREATE INDEX device_owner ON devices(owner_id);
CREATE TABLE messages (
 id TEXT PRIMARY KEY, sender_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
 name TEXT NOT NULL, kind TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','delivered','expired')),
 cipher TEXT, created_at INTEGER NOT NULL, expires INTEGER NOT NULL,received_at INTEGER
);
CREATE INDEX messages_device ON messages(device_id,state,created_at);
CREATE INDEX messages_sender ON messages(sender_id,created_at);
CREATE INDEX messages_expiry ON messages(state,expires);
CREATE TABLE rate_limits(bucket TEXT PRIMARY KEY,count INTEGER NOT NULL,expires INTEGER NOT NULL);
CREATE INDEX rates_expiry ON rate_limits(expires);
CREATE TABLE audit(id INTEGER PRIMARY KEY,at INTEGER NOT NULL,actor TEXT NOT NULL,action TEXT NOT NULL,subject TEXT NOT NULL);
