-- Schema 2: element storage (DATA_MODEL.md). Payloads are MessagePack (rmp-serde).
CREATE TABLE IF NOT EXISTS elements (
  id          BLOB PRIMARY KEY,
  category    TEXT NOT NULL,
  type_id     BLOB,
  level_id    BLOB,
  data        BLOB NOT NULL,
  params      BLOB NOT NULL,
  rev         INTEGER NOT NULL,
  modified_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS elements_category ON elements(category);
CREATE TABLE IF NOT EXISTS edges (
  from_id BLOB,
  to_id   BLOB,
  kind    TEXT,
  PRIMARY KEY (from_id, to_id, kind)
);
CREATE TABLE IF NOT EXISTS changes (
  seq        INTEGER PRIMARY KEY AUTOINCREMENT,
  tx_name    TEXT,
  element_id BLOB,
  op         TEXT,
  rev        INTEGER,
  at         INTEGER,
  synced     INTEGER DEFAULT 0
);
