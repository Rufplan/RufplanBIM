# Sync and Rufplan integration

> **Confirm against the live Rufplan Supabase schema before implementing.** Table and
> column names below are proposals. Any schema change requires owner approval.

## Authentication
- Supabase Auth, PKCE flow in the **system browser** (never an embedded login webview).
- Redirect URL: `rufplan-studio://auth/callback`, registered via `tauri-plugin-deep-link`
  and added to Supabase Auth allowed redirect URLs.
- Refresh token stored in the OS credential store (`keyring` crate). Access token in memory.
- The app ships only the Supabase URL and **anon** key. All access is governed by RLS.

## Proposed tables
```sql
create table studio_projects (
  id uuid primary key,                          -- = local project UUID
  rufplan_project_id uuid references projects(id), -- confirm name of Rufplan's projects table
  owner_id uuid not null references auth.users(id),
  name text not null,
  created_at timestamptz default now()
);
create table studio_issuances (
  id uuid primary key default gen_random_uuid(),
  studio_project_id uuid references studio_projects(id) on delete cascade,
  name text not null,                 -- "Permit Set", "Bid Set"
  issued_at timestamptz default now(),
  pdf_path text not null,             -- storage path
  ifc_path text,
  manifest jsonb not null,            -- sheet list, app version, element counts
  created_by uuid references auth.users(id)
);
-- RLS: owner (and later project members) can select/insert; nobody can update issuances.
```
- Storage bucket `studio-issuances`, path `{studio_project_id}/{issuance_id}/set.pdf|model.ifc|manifest.json`, private, signed URLs for viewing.

## Publish flow (M5)
1. User picks sheets + issuance name → export PDF and IFC to a temp dir.
2. Upload files to Storage (resumable for large IFC), then insert the `studio_issuances` row.
3. Show a link to the Rufplan project page. Rufplan web can render the IFC with That Open
   Engine and the PDF with its existing viewer.

## Worksharing (later, design constraint now)
- Local `changes` table + per-element `rev` enables element-level sync.
- First approach: **element check-out** (like Revit worksets) via a `studio_locks` table,
  then push changesets. Real-time CRDT co-editing only after that is solid.
- Never design anything that assumes a single writer forever.
