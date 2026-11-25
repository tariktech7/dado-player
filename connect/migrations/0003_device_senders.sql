CREATE TABLE device_senders (
 device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
 sender_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 label TEXT NOT NULL,
 linked_at INTEGER NOT NULL,
 PRIMARY KEY (device_id,sender_id)
);
CREATE INDEX device_senders_by_sender ON device_senders(sender_id,linked_at);
INSERT INTO device_senders(device_id,sender_id,label,linked_at)
 SELECT id,owner_id,COALESCE(NULLIF(label,''),platform),created_at
 FROM devices WHERE owner_id IS NOT NULL;
