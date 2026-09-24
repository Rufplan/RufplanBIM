-- Schema 1: project metadata only. Element tables arrive in M1.
CREATE TABLE IF NOT EXISTS meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
