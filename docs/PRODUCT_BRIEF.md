# Product brief — Rufplan Studio

## Vision
A modern, fast, cloud-connected BIM authoring tool for producing construction documents.
It competes on the things Revit does poorly: responsiveness, clean UX, painless
collaboration, open data (IFC-native), and a direct path from drawing set to marketplace
(bidding, hiring, and project delivery on Rufplan).

## Primary users
- Architects and designers at small-to-mid firms producing permit and construction sets.
- Solo practitioners and small studios priced out of or frustrated by Revit.
- Later: consultants (structural, MEP) who need to reference and annotate the model.

## What "done" looks like for v0.1 (end of M5)
A user can:
1. Create a project with levels and grids.
2. Draw walls, place doors and windows in them, draw floors, and place rooms.
3. See the model in plan, section, elevation and 3D, updating live as they edit.
4. Tag doors and rooms, dimension walls, and generate a door schedule.
5. Place views on sheets with a title block and export a vector PDF set at true scale.
6. Export IFC4 and publish the PDF set + IFC to a linked Rufplan project.

## Non-goals (for now)
- DWG import/export (licensing cost; revisit after v0.1).
- Structural analysis, MEP systems, energy analysis, rendering.
- A full family editor with constraint solving (post-v0.1; see ADR-006).
- Real-time multi-user co-editing (post-v0.1; element check-out first).
- Mac/Linux builds are welcome if free, but Windows is the priority platform.

## Domain glossary (for the engineer)
- **Element** — any object in the model (wall, door, level, room, tag, view, sheet).
- **Category** — the kind of element (Walls, Doors, Windows, Floors, Rooms, Levels…).
- **Family** — a parametric definition of a component (e.g., "Single Flush Door").
- **Type** — a named set of parameter values for a family ("36\" x 84\""). Shared by instances.
- **Instance** — one placed occurrence of a type. Has its own instance parameters.
- **Host / hosted** — doors and windows are hosted by walls; they cut openings and move with them.
- **Level** — a named horizontal datum (elevation). Elements are constrained to levels.
- **Grid** — a named reference line used for layout and dimensioning.
- **Wall location line** — the baseline a wall is drawn along (centerline, finish face, etc.).
- **Wall join** — how two walls meet (L, T, cross); geometry is cleaned so they merge.
- **View** — a derived representation: floor plan, reflected ceiling plan, section, elevation, 3D, schedule.
- **Cut plane** — the height at which a plan view slices the model (default 1200 mm above level).
- **View range** — top, cut plane, bottom, and view depth that control what a plan shows.
- **Projection vs. cut** — cut elements are drawn with heavy lines/poché; elements below the cut are drawn in projection (lighter).
- **Annotation** — view-specific elements: tags, dimensions, text, symbols, detail lines.
- **Tag** — an annotation that reads parameters from an element (door number, room name/area).
- **Schedule** — a table view listing elements and their parameters.
- **Sheet** — a printable page with a title block and viewports showing views at a scale.
- **Viewport** — a placed view on a sheet at a given scale (e.g., 1/4" = 1'-0").
- **Title block** — the sheet border with project info, sheet number and name.
- **Regeneration** — recomputing derived geometry and views after a change.
- **Drawing set / issuance** — the collection of sheets issued together (e.g., "Permit Set").
- **IFC** — Industry Foundation Classes, the open BIM exchange standard.
