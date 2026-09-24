# Data model

## Identity and units
- `ElementId` = UUID v7 (`uuid` crate, `v7` feature). Stable forever; used as sync key.
- IFC GlobalId is derived deterministically from ElementId (22-char IFC base64 encoding).
- Lengths: `f64` millimeters. Angles: `f64` radians. Areas: mm² internally, displayed as SF.
- Coordinates: project internal origin; a `SurveyPoint`/`ProjectBasePoint` pair is stored
  for later georeferencing but unused in v0.1.

## Parameters
```rust
enum ParamValue { Length(f64), Angle(f64), Area(f64), Number(f64), Integer(i64),
                  Bool(bool), Text(String), ElementRef(ElementId), Material(ElementId) }
struct ParamDef { key: String, label: String, kind: ParamKind, group: ParamGroup,
                  scope: Scope /* Type | Instance */, read_only: bool }
```
- Built-in parameters use stable keys (`wall.unconnected_height`, `door.mark`).
- Shared/project parameters (user-defined) come later but the storage must allow them now:
  params are stored as a key→value map, not only as struct fields.

## Core element kinds (v0.1)
| Kind | Key data |
|---|---|
| Level | name, elevation |
| Grid | name, line (2D segment) |
| WallType | layers (material, thickness, function), total thickness, function (exterior/interior) |
| Wall | type_id, location line (straight segment in M1; arcs later), location_line_ref (centerline, finish face ext/int, core face), base level + offset, top constraint (level + offset, or unconnected height), join settings per end |
| FamilyType (door/window) | family_id, width, height, sill height (windows), frame depth, plan symbol, 3D profile |
| Door / Window | type_id, host wall_id, position along wall (distance from wall start, mm), flip hand, flip facing, mark |
| FloorType | layered like WallType |
| Floor | type_id, level + offset, boundary loop(s) |
| Room | level, placement point, name, number; boundary derived from bounding walls/separation lines |
| View | kind (FloorPlan, CeilingPlan, Section, Elevation, ThreeD, Schedule), level, view range, crop, scale, visibility overrides, name |
| Annotation | view_id + kind: Tag(target_id, tag_family), Dimension(refs), Text, DetailLine, Symbol |
| Sheet | number, name, title block type, viewports[], issue data, stage_ids[] (deliverable sets it belongs to) |
| Material | name, cut pattern, surface pattern, color |
| ProjectInfo | project name, number, address, client, issue date, current_stage_id, stage_history[] (from, to, at, note) |
| ProjectStage | name, abbreviation, order, planned start date, target date (ADR-010). Defaults: Pre-Design (PD), Schematic Design (SD), Design Development (DD), Construction Documents (CD), Bidding / Negotiation (BN), Construction Administration (CA) |

## Families (v0.1 scope)
Families are **built-in and code-defined** for v0.1 (single flush door, double door,
fixed window, casement window, a basic title block), each parameterized by type values.
Store them so user-editable families can be added later without a file-format break:
`Family { id, category, name, definition: FamilyDef }` where `FamilyDef` is an enum
with a `BuiltIn(String)` variant now and a `Parametric(...)` variant reserved.

## Persistence: `.rfproj` (SQLite)
```sql
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);          -- schema_version, app_version
CREATE TABLE elements (
  id BLOB PRIMARY KEY,            -- 16-byte UUID v7
  category TEXT NOT NULL,
  type_id BLOB,
  level_id BLOB,
  data BLOB NOT NULL,             -- MessagePack (rmp-serde) of the element payload
  params BLOB NOT NULL,           -- MessagePack map of parameter values
  rev INTEGER NOT NULL,           -- increments on each modification (for sync)
  modified_at INTEGER NOT NULL
);
CREATE INDEX elements_category ON elements(category);
CREATE TABLE edges (from_id BLOB, to_id BLOB, kind TEXT, PRIMARY KEY(from_id,to_id,kind));
CREATE TABLE changes (seq INTEGER PRIMARY KEY AUTOINCREMENT, tx_name TEXT, element_id BLOB,
                      op TEXT, rev INTEGER, at INTEGER, synced INTEGER DEFAULT 0);
```
- `schema_version` in `meta`; migrations live in `studio-io/migrations/` and are forward-only.
- Save is transactional (SQLite transaction). Autosave every N minutes to a sidecar file.
- The `changes` table is the local change log used by sync.
