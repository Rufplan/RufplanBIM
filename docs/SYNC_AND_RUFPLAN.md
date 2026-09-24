# Sync and Rufplan integration

> Checked against the Rufplan codebase (`C:\Rufplan Drive\Rufplan`, `supabase/*.sql`,
> `packages/data`) on 2026-09-24. Any schema or bucket-config change requires owner approval.
> Decisions: ADR-016.

## Authentication (built, M5)
- Supabase project `https://gdqvrhoufwklgcanfjpj.supabase.co`. The app ships only the
  **anon** key, baked in at build time from `RUFPLAN_SUPABASE_ANON_KEY` or the gitignored
  `app/.env.local`. All access is governed by Rufplan's RLS.
- **Email and password** (`/auth/v1/token?grant_type=password`), the same accounts as rufplan.io.
- **Google**: PKCE in the **system browser** (never an embedded login webview). The redirect
  goes to a one-shot loopback listener at `http://127.0.0.1:53682/auth/callback`
  (RFC 8252), then the code is exchanged at `/auth/v1/token?grant_type=pkce`.
  **Owner action:** add that URL under Supabase → Authentication → URL Configuration →
  Redirect URLs.
- The refresh token is stored in the OS credential store (`keyring`, service "Rufplan Studio");
  the access token is kept in memory and refreshed a minute before it expires.

## Link (built)
- `ProjectInfo.rufplan = { id, name, slug }`, chosen from
  `open_projects?owner_business_id=eq.<uid>`. The project page is `rufplan.io/projects/<slug>`.

## Publish (built): uses existing Rufplan tables, no schema change
1. Take the current design stage's sheet set; export the PDF and the IFC.
2. Upload each file to Storage bucket `project-media` at
   `{uid}/deliverables/{project_id}/{phase_kind}/{deliverable_id}/{ms}-{file}` (the same
   path shape as Rufplan's `uploadDeliverable`).
3. Insert a `project_deliverables` row per file: `phase_kind` (PD/SD → `sd`, DD → `dd`,
   CD/BN/CA → `cd`), `deliverable_id` from Rufplan's catalog (e.g. `sd60`, `dd-owner`,
   `permit`, `ifc`), slot `drawings` for the PDF (with `sheet_count`), slot `A` for the IFC,
   plus `uploader_id = auth.uid()`, `uploaded_by`, `file_url`, `size_mb` and `notes`.
4. Record an `Issuance` locally so title blocks list it, and show the project link.

**Blocked on the owner:** `project-media.allowed_mime_types` is PDF/images/video only, so
Storage refuses the IFC. Publishing reports this and still uploads the PDF. To enable the IFC:
```sql
update storage.buckets
set allowed_mime_types = array_append(allowed_mime_types, 'application/x-step')
where id = 'project-media';
```
Rufplan's viewer would then also need an IFC viewer (That Open Engine / web-ifc) for slot `A`
files ending in `.ifc`.

## Worksharing (later, design constraint now)
- Local `changes` table + per-element `rev` enables element-level sync.
- First approach: **element check-out** (like Revit worksets) via a `studio_locks` table,
  then push changesets. Real-time CRDT co-editing only after that is solid.
- Never design anything that assumes a single writer forever.
